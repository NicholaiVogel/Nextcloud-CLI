mod capability_cache;
mod command_metadata;
mod commands;
mod credential_store;
mod error;
mod output;

use chrono::Utc;
use clap::Parser;
use std::time::Duration;
use tokio::time::{Instant, sleep};

use capability_cache::{CachedCapabilities, CapabilityCache};
use commands::{
    AuthCommand, Cli, Command, CommandsCommand, ConfigCommand, FilesCommand, ProfilesCommand,
    ServerCommand, UpdateCommand,
};
use credential_store::CredentialStore;
use error::{CliError, CliResult};
use nextcloud::{
    CapabilitiesClient, CliConfig, ConfigPaths, ConfigStore, LoginFlowV2Client, NextcloudClient,
    Profile, ServerCapabilities, WebDavClient,
};
use output::{print_error, print_success};
use serde::Serialize;
use serde_json::{Value, json};

pub async fn run_cli() {
    init_tracing();
    let cli = Cli::parse();
    let format = cli.format;

    match run(cli).await {
        Ok(value) => {
            if let Err(error) = print_success(&value, format) {
                print_error(&error);
                std::process::exit(error.exit_code());
            }
        }
        Err(error) => {
            print_error(&error);
            std::process::exit(error.exit_code());
        }
    }
}

async fn run(cli: Cli) -> CliResult<Value> {
    let paths = ConfigPaths::from_override(cli.config_dir.clone())?;
    let store = ConfigStore::new(paths.clone());
    let credential_store = CredentialStore::new(&paths.config_dir);
    let capability_cache = CapabilityCache::new(&paths.cache_dir);
    let selected_profile = selected_profile_name(cli.profile.as_deref());

    match cli.command {
        Command::Commands(CommandsCommand::Schema) => {
            json_value(command_metadata::command_schema())
        }
        Command::Config(command) => handle_config(command, &store, &credential_store),
        Command::Profiles(command) => handle_profiles(command, &store),
        Command::Auth(command) => {
            handle_auth(
                command,
                selected_profile.as_deref(),
                &store,
                &credential_store,
                &capability_cache,
            )
            .await
        }
        Command::Server(command) => {
            handle_server(
                command,
                selected_profile.as_deref(),
                &store,
                &credential_store,
                &capability_cache,
            )
            .await
        }
        Command::Files(command) => {
            handle_files(
                command,
                selected_profile.as_deref(),
                &store,
                &credential_store,
            )
            .await
        }
        Command::Update(UpdateCommand::Check) => json_value(UpdateCheck {
            current_version: env!("CARGO_PKG_VERSION").to_owned(),
            latest_version: env!("CARGO_PKG_VERSION").to_owned(),
            update_available: false,
            install_method: "development".to_owned(),
            can_self_update: false,
        }),
    }
}

fn handle_config(
    command: ConfigCommand,
    store: &ConfigStore,
    credential_store: &CredentialStore,
) -> CliResult<Value> {
    match command {
        ConfigCommand::Path => json_value(ConfigPathOutput {
            config_dir: store.paths().config_dir.display().to_string(),
            config_file: store.paths().config_file.display().to_string(),
            cache_dir: store.paths().cache_dir.display().to_string(),
            audit_dir: store.paths().audit_dir.display().to_string(),
            credential_file: credential_store.path().display().to_string(),
        }),
        ConfigCommand::Show => {
            let config = store.load()?;
            json_value(config)
        }
        ConfigCommand::Doctor => {
            store.paths().ensure()?;
            json_value(ConfigDoctorOutput {
                ok: true,
                config_dir_exists: store.paths().config_dir.exists(),
                config_file_exists: store.paths().config_file.exists(),
                cache_dir_exists: store.paths().cache_dir.exists(),
                audit_dir_exists: store.paths().audit_dir.exists(),
                credential_file_exists: credential_store.path().exists(),
            })
        }
    }
}

fn handle_profiles(command: ProfilesCommand, store: &ConfigStore) -> CliResult<Value> {
    match command {
        ProfilesCommand::List => {
            let config = store.load()?;
            let profiles: Vec<ProfileListItem> = config
                .profiles
                .values()
                .map(|profile| ProfileListItem {
                    name: profile.name.clone(),
                    server: profile.server.to_string(),
                    username: profile.username.clone(),
                    is_default: config.default_profile.as_deref() == Some(profile.name.as_str()),
                    agent_mode: profile.policy.agent_mode,
                })
                .collect();
            json_value(json!({
                "default_profile": config.default_profile,
                "profiles": profiles,
            }))
        }
        ProfilesCommand::Show(args) => {
            let config = store.load()?;
            let profile = config
                .profiles
                .get(&args.name)
                .ok_or_else(|| nextcloud::Error::ProfileNotFound { name: args.name })?;
            json_value(profile)
        }
        ProfilesCommand::SetDefault(args) => {
            let mut config = store.load()?;
            config.set_default_profile(&args.name)?;
            store.save(&config)?;
            json_value(json!({
                "default_profile": args.name,
                "updated": true,
            }))
        }
    }
}

async fn handle_auth(
    command: AuthCommand,
    selected_profile: Option<&str>,
    store: &ConfigStore,
    credential_store: &CredentialStore,
    capability_cache: &CapabilityCache,
) -> CliResult<Value> {
    match command {
        AuthCommand::Login(args) => {
            let server = Profile::parse_server(&args.server)?;
            let client = NextcloudClient::new(server.clone(), None)?;
            let login_client = LoginFlowV2Client::new(client);
            let flow = login_client.start().await?;

            eprintln!("Open this URL to authorize nextcloud-cli:");
            eprintln!("{}", flow.login);

            let browser_opened = if args.no_open {
                false
            } else {
                webbrowser::open(flow.login.as_str()).map_err(|error| CliError::BrowserOpen {
                    url: flow.login.to_string(),
                    message: error.to_string(),
                })?;
                true
            };

            let credentials = poll_login_flow(
                &login_client,
                &flow.poll,
                args.timeout_seconds,
                args.poll_interval_seconds,
            )
            .await?;
            let profile_name = args
                .profile
                .clone()
                .or_else(|| selected_profile.map(str::to_owned))
                .unwrap_or_else(|| {
                    default_profile_name(&credentials.login_name, credentials.server.host_str())
                });
            let profile = Profile::new(
                profile_name.clone(),
                credentials.server.clone(),
                credentials.login_name.clone(),
            );

            credential_store.put_app_password(
                &profile.credential,
                &profile.username,
                &credentials.app_password,
            )?;

            let mut config = store.load()?;
            config.upsert_profile(profile.clone(), args.set_default);
            store.save(&config)?;
            let cache_invalidated = capability_cache.invalidate(&profile.name)?;

            json_value(AuthLoginOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                username: profile.username,
                auth_type: "app_password".to_owned(),
                credential_backend: credential_store.backend_name().to_owned(),
                credential_stored: true,
                browser_opened,
                cache_invalidated,
                default_profile: config.default_profile,
            })
        }
        AuthCommand::Add(args) => {
            let app_password = args.app_password.ok_or(CliError::MissingAppPassword)?;
            let server = Profile::parse_server(&args.server)?;
            let profile_name = args
                .profile
                .clone()
                .or_else(|| selected_profile.map(str::to_owned))
                .unwrap_or_else(|| default_profile_name(&args.user, server.host_str()));
            let profile = Profile::new(profile_name.clone(), server, args.user.clone());

            credential_store.put_app_password(
                &profile.credential,
                &profile.username,
                &app_password,
            )?;

            let mut config = store.load()?;
            config.upsert_profile(profile.clone(), args.set_default);
            store.save(&config)?;
            let cache_invalidated = capability_cache.invalidate(&profile.name)?;

            json_value(AuthAddOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                username: profile.username,
                auth_type: "app_password".to_owned(),
                credential_backend: credential_store.backend_name().to_owned(),
                credential_stored: true,
                cache_invalidated,
                default_profile: config.default_profile,
            })
        }
        AuthCommand::Status => {
            let config = store.load()?;
            let profile = config.selected_profile(selected_profile)?;
            let credential_stored = credential_store.has_credential(&profile.credential)?;
            json_value(AuthStatusOutput {
                authenticated: credential_stored,
                profile: profile.name.clone(),
                server: profile.server.to_string(),
                username: profile.username.clone(),
                credential_backend: credential_store.backend_name().to_owned(),
                credential_stored,
            })
        }
    }
}

async fn handle_files(
    command: FilesCommand,
    selected_profile: Option<&str>,
    store: &ConfigStore,
    credential_store: &CredentialStore,
) -> CliResult<Value> {
    let profile = store.selected_profile(selected_profile)?;
    let app_password = credential_store.get_app_password(&profile.credential)?;
    let client = NextcloudClient::from_profile(&profile, Some(app_password))?;
    let webdav = WebDavClient::new(client, profile.username.clone());

    match command {
        FilesCommand::List(args) => {
            let entries = webdav.list(&args.path).await?;
            json_value(json!({
                "path": args.path,
                "entries": entries,
            }))
        }
        FilesCommand::Stat(args) => {
            let entry = webdav.stat(&args.path).await?;
            json_value(json!({
                "path": args.path,
                "entry": entry,
            }))
        }
        FilesCommand::Mkdir(args) => {
            if !args.dry_run {
                webdav.mkdir(&args.path).await?;
            }
            json_value(json!({
                "path": args.path,
                "dry_run": args.dry_run,
                "created": !args.dry_run,
                "method": "MKCOL",
            }))
        }
    }
}

async fn handle_server(
    command: ServerCommand,
    selected_profile: Option<&str>,
    store: &ConfigStore,
    credential_store: &CredentialStore,
    capability_cache: &CapabilityCache,
) -> CliResult<Value> {
    let profile = store.selected_profile(selected_profile)?;
    let app_password = credential_store.get_app_password(&profile.credential).ok();
    let client = NextcloudClient::from_profile(&profile, app_password)?;
    let capabilities = CapabilitiesClient::new(client);

    match command {
        ServerCommand::Status => json_value(capabilities.server_status().await?),
        ServerCommand::Capabilities(args) => {
            if !args.refresh
                && let Some(cached) = capability_cache.load_fresh(&profile.name, &profile.server)?
            {
                return capabilities_output(profile.name, profile.server.to_string(), cached, true);
            }

            let fetched = capabilities.capabilities().await?;
            let cached = capability_cache.save(&profile.name, &profile.server, fetched)?;
            capabilities_output(profile.name, profile.server.to_string(), cached, false)
        }
    }
}

async fn poll_login_flow(
    login_client: &LoginFlowV2Client,
    poll: &nextcloud::LoginFlowV2Poll,
    timeout_seconds: u64,
    poll_interval_seconds: u64,
) -> CliResult<nextcloud::LoginFlowV2Credentials> {
    let deadline = Instant::now() + Duration::from_secs(timeout_seconds);
    let interval = Duration::from_secs(poll_interval_seconds.max(1));

    loop {
        match login_client.poll(poll).await {
            Ok(credentials) => return Ok(credentials),
            Err(nextcloud::Error::HttpStatus { status, .. }) if status.as_u16() == 404 => {
                if Instant::now() >= deadline {
                    return Err(CliError::LoginTimeout { timeout_seconds });
                }
                sleep(interval).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn capabilities_output(
    profile: String,
    server: String,
    cached: CachedCapabilities,
    cache_used: bool,
) -> CliResult<Value> {
    let ServerCapabilities {
        version,
        capabilities,
    } = cached.capabilities;
    json_value(json!({
        "profile": profile,
        "server": server,
        "version": version,
        "capabilities": capabilities,
        "checked_at": Utc::now(),
        "cache": {
            "used": cache_used,
            "cached_at": cached.cached_at,
            "ttl_seconds": cached.ttl_seconds,
        }
    }))
}

fn selected_profile_name(flag: Option<&str>) -> Option<String> {
    flag.map(str::to_owned)
        .or_else(|| std::env::var("NEXTCLOUD_CLI_PROFILE").ok())
        .filter(|value| !value.trim().is_empty())
}

fn default_profile_name(username: &str, host: Option<&str>) -> String {
    match host {
        Some(host) => format!("{username}@{host}"),
        None => username.to_owned(),
    }
}

fn json_value<T>(value: T) -> CliResult<Value>
where
    T: Serialize,
{
    Ok(serde_json::to_value(value)?)
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init();
}

#[derive(Debug, Serialize)]
struct ConfigPathOutput {
    config_dir: String,
    config_file: String,
    cache_dir: String,
    audit_dir: String,
    credential_file: String,
}

#[derive(Debug, Serialize)]
struct ConfigDoctorOutput {
    ok: bool,
    config_dir_exists: bool,
    config_file_exists: bool,
    cache_dir_exists: bool,
    audit_dir_exists: bool,
    credential_file_exists: bool,
}

#[derive(Debug, Serialize)]
struct ProfileListItem {
    name: String,
    server: String,
    username: String,
    is_default: bool,
    agent_mode: bool,
}

#[derive(Debug, Serialize)]
struct AuthAddOutput {
    profile: String,
    server: String,
    username: String,
    auth_type: String,
    credential_backend: String,
    credential_stored: bool,
    cache_invalidated: bool,
    default_profile: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuthLoginOutput {
    profile: String,
    server: String,
    username: String,
    auth_type: String,
    credential_backend: String,
    credential_stored: bool,
    browser_opened: bool,
    cache_invalidated: bool,
    default_profile: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuthStatusOutput {
    authenticated: bool,
    profile: String,
    server: String,
    username: String,
    credential_backend: String,
    credential_stored: bool,
}

#[derive(Debug, Serialize)]
struct UpdateCheck {
    current_version: String,
    latest_version: String,
    update_available: bool,
    install_method: String,
    can_self_update: bool,
}

#[allow(dead_code)]
fn _assert_config_is_used(_config: &CliConfig) {}
