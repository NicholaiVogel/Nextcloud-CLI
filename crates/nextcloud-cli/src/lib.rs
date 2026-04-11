mod audit;
mod capability_cache;
mod command_metadata;
mod commands;
mod credential_store;
mod error;
mod output;

use chrono::{DateTime, Days, Local, LocalResult, NaiveDate, TimeZone, Utc};
use clap::Parser;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use tokio::time::{Instant, sleep};
use zeroize::Zeroizing;

use audit::{command_executed, record, remote_path_target, share_id_target, target};
use capability_cache::{CachedCapabilities, CapabilityCache};
use commands::{
    ActivityCommand, AuthAppPasswordArgs, AuthCommand, CalendarCommand, CalendarEventsArgs, Cli,
    Command, CommandsCommand, ConfigCommand, ContactsCommand, DeckCommand, FilesCommand,
    NotesCommand, ProfilePolicySetArgs, ProfilesCommand, ServerCommand, SharesCommand,
    UpdateCommand,
};
use credential_store::CredentialStore;
use error::{CliError, CliResult};
use nextcloud::{
    AppPasswordClient, CapabilitiesClient, CliConfig, ClientAuth, ConfigPaths, ConfigStore,
    LoginFlowV2Client, NextcloudClient, Profile, ServerCapabilities, SharesClient, WebDavClient,
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
        Command::Shares(command) => {
            handle_shares(
                command,
                selected_profile.as_deref(),
                &store,
                &credential_store,
            )
            .await
        }
        Command::Calendar(command) => {
            handle_calendar(
                command,
                selected_profile.as_deref(),
                &store,
                &credential_store,
            )
            .await
        }
        Command::Contacts(command) => {
            handle_contacts(
                command,
                selected_profile.as_deref(),
                &store,
                &credential_store,
            )
            .await
        }
        Command::Activity(command) => {
            handle_activity(
                command,
                selected_profile.as_deref(),
                &store,
                &credential_store,
            )
            .await
        }
        Command::Notes(command) => {
            handle_notes(
                command,
                selected_profile.as_deref(),
                &store,
                &credential_store,
            )
            .await
        }
        Command::Deck(command) => {
            handle_deck(
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
            let profile = config
                .profiles
                .get(&args.name)
                .ok_or_else(|| nextcloud::Error::ProfileNotFound {
                    name: args.name.clone(),
                })?
                .clone();
            store.save(&config)?;
            record(
                store.paths(),
                &command_executed(
                    &profile.name,
                    profile.server.as_str(),
                    "profiles.set-default",
                    false,
                    "CONFIG",
                    store.paths().config_file.to_string_lossy().as_ref(),
                    target([("default_profile", json!(args.name))]),
                ),
            );
            json_value(json!({
                "default_profile": args.name,
                "updated": true,
            }))
        }
        ProfilesCommand::Policy(command) => handle_profile_policy(command, store),
    }
}

fn handle_profile_policy(
    command: commands::ProfilePolicyCommand,
    store: &ConfigStore,
) -> CliResult<Value> {
    match command {
        commands::ProfilePolicyCommand::Show(args) => {
            let config = store.load()?;
            let profile = config
                .profiles
                .get(&args.name)
                .ok_or_else(|| nextcloud::Error::ProfileNotFound { name: args.name })?;
            json_value(ProfilePolicyOutput {
                profile: profile.name.clone(),
                server: profile.server.to_string(),
                policy: profile.policy.clone(),
                updated: false,
            })
        }
        commands::ProfilePolicyCommand::Set(args) => {
            let mut config = store.load()?;
            let profile = config.profiles.get_mut(&args.name).ok_or_else(|| {
                nextcloud::Error::ProfileNotFound {
                    name: args.name.clone(),
                }
            })?;
            let changed = apply_profile_policy_changes(profile, &args)?;
            if changed.is_empty() {
                return Err(CliError::NoPolicyChanges);
            }
            profile.updated_at = Utc::now();
            let output = ProfilePolicySetOutput {
                profile: profile.name.clone(),
                server: profile.server.to_string(),
                policy: profile.policy.clone(),
                updated: true,
                changed,
            };
            store.save(&config)?;
            record(
                store.paths(),
                &command_executed(
                    &output.profile,
                    &output.server,
                    "profiles.policy.set",
                    false,
                    "CONFIG",
                    store.paths().config_file.to_string_lossy().as_ref(),
                    target([("changed", json!(output.changed))]),
                ),
            );
            json_value(output)
        }
        commands::ProfilePolicyCommand::Reset(args) => {
            if !args.yes {
                return Err(CliError::ConfirmationRequired);
            }

            let mut config = store.load()?;
            let profile = config.profiles.get_mut(&args.name).ok_or_else(|| {
                nextcloud::Error::ProfileNotFound {
                    name: args.name.clone(),
                }
            })?;
            profile.policy = nextcloud::ProfilePolicy::default();
            profile.updated_at = Utc::now();
            let output = ProfilePolicyResetOutput {
                profile: profile.name.clone(),
                server: profile.server.to_string(),
                policy: profile.policy.clone(),
                updated: true,
                reset: true,
            };
            store.save(&config)?;
            record(
                store.paths(),
                &command_executed(
                    &output.profile,
                    &output.server,
                    "profiles.policy.reset",
                    false,
                    "CONFIG",
                    store.paths().config_file.to_string_lossy().as_ref(),
                    target([("reset", json!(true))]),
                ),
            );
            json_value(output)
        }
    }
}

fn apply_profile_policy_changes(
    profile: &mut Profile,
    args: &ProfilePolicySetArgs,
) -> CliResult<Vec<String>> {
    let mut changed = Vec::new();
    if let Some(value) = args.agent_mode
        && profile.policy.agent_mode != value
    {
        profile.policy.agent_mode = value;
        changed.push("agent_mode".to_owned());
    }
    if let Some(value) = args.default_dry_run
        && profile.policy.default_dry_run != value
    {
        profile.policy.default_dry_run = value;
        changed.push("default_dry_run".to_owned());
    }
    if let Some(value) = args.allow_destructive
        && profile.policy.allow_destructive != value
    {
        profile.policy.allow_destructive = value;
        changed.push("allow_destructive".to_owned());
    }
    if let Some(value) = args.allow_public_shares
        && profile.policy.allow_public_shares != value
    {
        profile.policy.allow_public_shares = value;
        changed.push("allow_public_shares".to_owned());
    }

    Ok(changed)
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
        AuthCommand::AppPassword(args) => {
            let server = Profile::parse_server(&args.server)?;
            let (account_password, password_source) = read_account_password(&args)?;
            let client = NextcloudClient::new(
                server.clone(),
                Some(ClientAuth {
                    username: args.user.clone(),
                    app_password: account_password.to_string(),
                }),
            )?;
            let app_password_client = AppPasswordClient::new(client);
            let credentials = app_password_client.create_app_password().await?;

            let profile_name = args
                .profile
                .clone()
                .or_else(|| selected_profile.map(str::to_owned))
                .unwrap_or_else(|| default_profile_name(&args.user, server.host_str()));
            let profile = Profile::new(profile_name.clone(), server, args.user.clone());

            credential_store.put_app_password(
                &profile.credential,
                &profile.username,
                &credentials.app_password,
            )?;

            let mut config = store.load()?;
            config.upsert_profile(profile.clone(), args.set_default);
            store.save(&config)?;
            let cache_invalidated = capability_cache.invalidate(&profile.name)?;

            json_value(AuthAppPasswordOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                username: profile.username,
                auth_type: "app_password".to_owned(),
                credential_backend: credential_store.backend_name().to_owned(),
                credential_stored: true,
                password_source: password_source.to_owned(),
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

fn read_account_password(
    args: &AuthAppPasswordArgs,
) -> CliResult<(Zeroizing<String>, &'static str)> {
    if args.password_stdin {
        let mut password = String::new();
        std::io::stdin()
            .read_to_string(&mut password)
            .map_err(CliError::PasswordStdinRead)?;
        let trimmed_len = password.trim_end_matches(&['\r', '\n'][..]).len();
        password.truncate(trimmed_len);
        if password.is_empty() {
            return Err(CliError::MissingAccountPassword);
        }
        return Ok((Zeroizing::new(password), "stdin"));
    }

    if let Some(name) = &args.password_env {
        let password =
            std::env::var(name).map_err(|_| CliError::PasswordEnvMissing { name: name.clone() })?;
        if password.is_empty() {
            return Err(CliError::MissingAccountPassword);
        }
        return Ok((Zeroizing::new(password), "env"));
    }

    Err(CliError::MissingAccountPassword)
}

async fn handle_shares(
    command: SharesCommand,
    selected_profile: Option<&str>,
    store: &ConfigStore,
    credential_store: &CredentialStore,
) -> CliResult<Value> {
    let profile = store.selected_profile(selected_profile)?;
    let app_password = credential_store.get_app_password(&profile.credential)?;
    let client = NextcloudClient::from_profile(&profile, Some(app_password))?;
    let shares_client = SharesClient::new(client);

    match command {
        SharesCommand::List(args) => {
            let path = args
                .path
                .as_deref()
                .map(nextcloud::webdav::normalize_remote_path)
                .transpose()?;
            let shares = shares_client
                .list(&nextcloud::ShareListOptions {
                    path: path.clone(),
                    shared_with_me: args.shared_with_me,
                    include_tags: args.include_tags,
                })
                .await?;
            let count = shares.len();

            json_value(json!({
                "profile": profile.name,
                "server": profile.server.to_string(),
                "path": path,
                "shared_with_me": args.shared_with_me,
                "include_tags": args.include_tags,
                "shares": shares,
                "count": count,
            }))
        }
        SharesCommand::Create(args) => {
            if !args.public {
                return Err(CliError::UnsupportedShareCreateMode);
            }

            let path = nextcloud::webdav::normalize_remote_path(&args.path)?;
            let expire_date = args
                .expire_date
                .as_deref()
                .map(validate_expire_date)
                .transpose()?;
            let password_protected = args.password.is_some();

            if args.dry_run {
                record(
                    store.paths(),
                    &command_executed(
                        &profile.name,
                        profile.server.as_str(),
                        "shares.create",
                        true,
                        "POST",
                        "/ocs/v2.php/apps/files_sharing/api/v1/shares",
                        target([
                            ("remote_path", json!(path)),
                            ("public", json!(true)),
                            ("password_protected", json!(password_protected)),
                            ("expiration", json!(expire_date)),
                        ]),
                    ),
                );
                return json_value(json!({
                    "profile": profile.name,
                    "server": profile.server.to_string(),
                    "path": path,
                    "dry_run": true,
                    "created": false,
                    "public": true,
                    "share_type": "public_link",
                    "permissions": 1,
                    "password_protected": password_protected,
                    "expiration": expire_date,
                    "policy_allowed": profile.policy.allow_public_shares,
                    "requires_confirmation": true,
                }));
            }

            if !args.yes {
                return Err(CliError::SensitiveConfirmationRequired {
                    command: "shares create --public".to_owned(),
                });
            }

            if !profile.policy.allow_public_shares {
                return Err(CliError::PolicyDenied {
                    profile: profile.name,
                    command: "shares create --public".to_owned(),
                    policy: "allow_public_shares=false".to_owned(),
                });
            }

            let share = shares_client
                .create_public(&nextcloud::ShareCreatePublicOptions {
                    path: path.clone(),
                    password: args.password,
                    expire_date: expire_date.clone(),
                    permissions: 1,
                })
                .await?;
            record(
                store.paths(),
                &command_executed(
                    &profile.name,
                    profile.server.as_str(),
                    "shares.create",
                    false,
                    "POST",
                    "/ocs/v2.php/apps/files_sharing/api/v1/shares",
                    target([
                        ("remote_path", json!(path)),
                        ("share_id", json!(share.id)),
                        ("public", json!(true)),
                        ("password_protected", json!(password_protected)),
                        ("expiration", json!(expire_date)),
                    ]),
                ),
            );

            json_value(json!({
                "profile": profile.name,
                "server": profile.server.to_string(),
                "path": path,
                "dry_run": false,
                "created": true,
                "share": share,
            }))
        }
        SharesCommand::Delete(args) => {
            handle_share_delete(
                args.share_id,
                args.dry_run,
                args.yes,
                "shares delete",
                &profile,
                &shares_client,
                store,
            )
            .await
        }
        SharesCommand::Revoke(args) => {
            handle_share_delete(
                args.share_id,
                args.dry_run,
                args.yes,
                "shares revoke",
                &profile,
                &shares_client,
                store,
            )
            .await
        }
    }
}

async fn handle_share_delete(
    share_id: String,
    dry_run: bool,
    yes: bool,
    command: &str,
    profile: &Profile,
    shares_client: &SharesClient,
    store: &ConfigStore,
) -> CliResult<Value> {
    let share_id = validate_share_id(&share_id)?;
    let command_key = command.replace(' ', ".");
    if dry_run {
        record(
            store.paths(),
            &command_executed(
                &profile.name,
                profile.server.as_str(),
                &command_key,
                true,
                "DELETE",
                "/ocs/v2.php/apps/files_sharing/api/v1/shares/{share-id}",
                share_id_target(&share_id),
            ),
        );
        return json_value(ShareDeleteOutput {
            profile: profile.name.clone(),
            server: profile.server.to_string(),
            share_id,
            dry_run: true,
            deleted: false,
            confirmed: false,
            command: command.to_owned(),
        });
    }

    if !yes {
        return Err(CliError::SensitiveConfirmationRequired {
            command: command.to_owned(),
        });
    }

    shares_client.delete(&share_id).await?;
    record(
        store.paths(),
        &command_executed(
            &profile.name,
            profile.server.as_str(),
            &command_key,
            false,
            "DELETE",
            "/ocs/v2.php/apps/files_sharing/api/v1/shares/{share-id}",
            share_id_target(&share_id),
        ),
    );
    json_value(ShareDeleteOutput {
        profile: profile.name.clone(),
        server: profile.server.to_string(),
        share_id,
        dry_run: false,
        deleted: true,
        confirmed: true,
        command: command.to_owned(),
    })
}

fn validate_expire_date(value: &str) -> CliResult<String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| value.to_owned())
        .map_err(|_| CliError::InvalidExpireDate {
            value: value.to_owned(),
        })
}

fn validate_share_id(value: &str) -> CliResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CliError::InvalidShareId {
            value: value.to_owned(),
        });
    }
    Ok(trimmed.to_owned())
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
    let profile_name = profile.name.clone();
    let server = profile.server.to_string();

    match command {
        FilesCommand::List(args) => {
            let path = nextcloud::webdav::normalize_remote_path(&args.path)?;
            let entries = webdav.list(&path).await?;
            let count = entries.len();
            json_value(json!({
                "path": path,
                "entries": entries,
                "count": count,
            }))
        }
        FilesCommand::Search(args) => {
            if args.search_mode != "name" {
                return Err(CliError::UnsupportedSearchMode {
                    mode: args.search_mode,
                });
            }
            let scope = nextcloud::webdav::normalize_remote_path(&args.path)?;
            let files = webdav.search(&args.query, &scope, args.limit).await?;
            let count = files.len();
            json_value(json!({
                "query": args.query,
                "scope": scope,
                "search_mode": "name",
                "limit": args.limit.clamp(1, 100),
                "files": files,
                "count": count,
            }))
        }
        FilesCommand::Stat(args) => {
            let path = nextcloud::webdav::normalize_remote_path(&args.path)?;
            let entry = webdav.stat(&path).await?;
            json_value(json!({
                "path": path,
                "entry": entry,
            }))
        }
        FilesCommand::Mkdir(args) => {
            let normalized = nextcloud::webdav::normalize_remote_path(&args.path)?;
            let parents_created = if args.dry_run {
                Vec::new()
            } else if args.parents {
                webdav.mkdir_parents(&normalized).await?
            } else {
                webdav.mkdir(&normalized).await?;
                vec![normalized.clone()]
            };
            record(
                store.paths(),
                &command_executed(
                    &profile_name,
                    &server,
                    "files.mkdir",
                    args.dry_run,
                    "MKCOL",
                    "/remote.php/dav/files/{username}{remote-path}",
                    remote_path_target(&normalized),
                ),
            );
            json_value(json!({
                "path": normalized,
                "dry_run": args.dry_run,
                "created": !args.dry_run,
                "method": "MKCOL",
                "parents": args.parents,
                "parents_created": parents_created,
                "profile": profile_name,
                "server": server,
            }))
        }
        FilesCommand::Upload(args) => {
            let remote = nextcloud::webdav::normalize_remote_path(&args.remote)?;
            let local = args.local;
            ensure_upload_file(&local)?;
            if !args.overwrite && webdav.exists(&remote).await? {
                return Err(CliError::RemotePathExists { path: remote });
            }

            let bytes = fs::read(&local).map_err(|source| nextcloud::Error::ReadFile {
                path: local.clone(),
                source,
            })?;
            let bytes_uploaded = bytes.len() as u64;
            let etag = webdav
                .upload(&remote, bytes, args.content_type.as_deref())
                .await?;
            let entry = webdav.stat(&remote).await?.map(|mut entry| {
                if entry.etag.is_none() {
                    entry.etag.clone_from(&etag);
                }
                entry
            });
            record(
                store.paths(),
                &command_executed(
                    &profile_name,
                    &server,
                    "files.upload",
                    false,
                    "PUT",
                    "/remote.php/dav/files/{username}{remote-path}",
                    target([
                        ("remote_path", json!(remote)),
                        ("bytes", json!(bytes_uploaded)),
                    ]),
                ),
            );

            json_value(FilesUploadOutput {
                profile: profile_name,
                server,
                remote,
                local: local.display().to_string(),
                bytes_uploaded,
                etag,
                overwritten: args.overwrite,
                entry,
            })
        }
        FilesCommand::Download(args) => {
            let remote = nextcloud::webdav::normalize_remote_path(&args.remote)?;
            let local = args.local;
            let existed_before = local.exists();
            if existed_before && !args.overwrite {
                return Err(CliError::LocalFileExists { path: local });
            }

            let partial = partial_download_path(&local);
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&partial)
                .map_err(|source| nextcloud::Error::WriteFile {
                    path: partial.clone(),
                    source,
                })?;

            let download = match webdav.download_to_writer(&remote, &mut file).await {
                Ok(download) => download,
                Err(error) => {
                    let _ = fs::remove_file(&partial);
                    return Err(error.into());
                }
            };

            if let Err(source) = file.sync_all() {
                drop(file);
                let _ = fs::remove_file(&partial);
                return Err(nextcloud::Error::WriteFile {
                    path: partial.clone(),
                    source,
                }
                .into());
            }
            drop(file);

            if !args.overwrite && local.exists() {
                let _ = fs::remove_file(&partial);
                return Err(CliError::LocalFileExists { path: local });
            }

            fs::rename(&partial, &local).map_err(|source| {
                let _ = fs::remove_file(&partial);
                nextcloud::Error::WriteFile {
                    path: local.clone(),
                    source,
                }
            })?;
            record(
                store.paths(),
                &command_executed(
                    &profile_name,
                    &server,
                    "files.download",
                    false,
                    "GET",
                    "/remote.php/dav/files/{username}{remote-path}",
                    target([
                        ("remote_path", json!(remote)),
                        ("bytes", json!(download.bytes_written)),
                    ]),
                ),
            );

            json_value(FilesDownloadOutput {
                profile: profile_name,
                server,
                remote,
                local: local.display().to_string(),
                bytes_written: download.bytes_written,
                content_length: download.content_length,
                overwritten: args.overwrite && existed_before,
            })
        }
        FilesCommand::Delete(args) => {
            let path = nextcloud::webdav::normalize_remote_path(&args.path)?;
            nextcloud::webdav::reject_root_path(&path)?;
            if !args.dry_run && !args.yes {
                return Err(CliError::ConfirmationRequired);
            }
            let entry = webdav.stat(&path).await?;
            if !args.dry_run {
                webdav.delete(&path).await?;
            }
            record(
                store.paths(),
                &command_executed(
                    &profile_name,
                    &server,
                    "files.delete",
                    args.dry_run,
                    "DELETE",
                    "/remote.php/dav/files/{username}{remote-path}",
                    remote_path_target(&path),
                ),
            );

            json_value(FilesDeleteOutput {
                profile: profile_name,
                server,
                path,
                dry_run: args.dry_run,
                deleted: !args.dry_run,
                confirmed: args.yes,
                entry,
            })
        }
    }
}

fn ensure_upload_file(path: &Path) -> CliResult<()> {
    let metadata = fs::metadata(path).map_err(|source| nextcloud::Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(CliError::LocalUploadNotFile {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn partial_download_path(path: &Path) -> std::path::PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("download");
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    path.with_file_name(format!(
        ".{file_name}.{}.{}.part",
        std::process::id(),
        timestamp
    ))
}

async fn handle_calendar(
    command: CalendarCommand,
    selected_profile: Option<&str>,
    store: &ConfigStore,
    credential_store: &CredentialStore,
) -> CliResult<Value> {
    let profile = store.selected_profile(selected_profile)?;
    let app_password = credential_store.get_app_password(&profile.credential)?;
    let client = NextcloudClient::from_profile(&profile, Some(app_password))?;
    let calendar = nextcloud::CalendarClient::new(client, profile.username.clone());

    match command {
        CalendarCommand::Events(args) => {
            let range = resolve_calendar_range(&args)?;
            let events = calendar
                .events(&nextcloud::CalendarEventsOptions {
                    from: range.from,
                    to: range.to,
                    calendar: args.calendar.clone(),
                })
                .await?;
            let count = events.len();
            json_value(CalendarEventsOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                calendar: args.calendar,
                range,
                events,
                count,
            })
        }
        CalendarCommand::Create(args) => {
            let event = build_calendar_create_options(&args)?;
            if args.dry_run {
                record(
                    store.paths(),
                    &command_executed(
                        &profile.name,
                        profile.server.as_str(),
                        "calendar.create",
                        true,
                        "PUT",
                        "/remote.php/dav/calendars/{username}/{calendar}/{uid}.ics",
                        target([
                            ("calendar", json!(event.calendar.clone())),
                            ("uid", json!(event.uid.clone())),
                            ("summary", json!(event.summary.clone())),
                            ("all_day", json!(event.all_day)),
                        ]),
                    ),
                );
                return json_value(CalendarCreateOutput {
                    profile: profile.name,
                    server: profile.server.to_string(),
                    dry_run: true,
                    created: false,
                    event: CalendarCreatePreview {
                        uid: event.uid,
                        calendar: event.calendar,
                        summary: event.summary,
                        starts_at: event.starts_at,
                        ends_at: event.ends_at,
                        all_day: event.all_day,
                        location: event.location,
                        description_present: event.description.is_some(),
                    },
                });
            }

            let created = calendar.create_event(&event).await?;
            record(
                store.paths(),
                &command_executed(
                    &profile.name,
                    profile.server.as_str(),
                    "calendar.create",
                    false,
                    "PUT",
                    "/remote.php/dav/calendars/{username}/{calendar}/{uid}.ics",
                    target([
                        ("calendar", json!(event.calendar.clone())),
                        ("uid", json!(event.uid.clone())),
                        ("summary", json!(event.summary.clone())),
                        ("all_day", json!(event.all_day)),
                    ]),
                ),
            );
            json_value(json!({
                "profile": profile.name,
                "server": profile.server.to_string(),
                "dry_run": false,
                "created": true,
                "event": created,
            }))
        }
        CalendarCommand::Delete(args) => {
            if args.dry_run {
                record(
                    store.paths(),
                    &command_executed(
                        &profile.name,
                        profile.server.as_str(),
                        "calendar.delete",
                        true,
                        "DELETE",
                        "/remote.php/dav/calendars/{username}/{calendar}/{uid}.ics",
                        target([
                            ("calendar", json!(args.calendar.clone())),
                            ("uid", json!(args.uid.clone())),
                        ]),
                    ),
                );
                return json_value(DeleteObjectOutput {
                    profile: profile.name,
                    server: profile.server.to_string(),
                    command: "calendar delete".to_owned(),
                    object_type: "calendar_event".to_owned(),
                    collection: args.calendar,
                    id: args.uid,
                    dry_run: true,
                    deleted: false,
                    confirmed: false,
                });
            }
            if !args.yes {
                return Err(CliError::ConfirmationRequired);
            }
            calendar.delete_event(&args.calendar, &args.uid).await?;
            record(
                store.paths(),
                &command_executed(
                    &profile.name,
                    profile.server.as_str(),
                    "calendar.delete",
                    false,
                    "DELETE",
                    "/remote.php/dav/calendars/{username}/{calendar}/{uid}.ics",
                    target([
                        ("calendar", json!(args.calendar.clone())),
                        ("uid", json!(args.uid.clone())),
                    ]),
                ),
            );
            json_value(DeleteObjectOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                command: "calendar delete".to_owned(),
                object_type: "calendar_event".to_owned(),
                collection: args.calendar,
                id: args.uid,
                dry_run: false,
                deleted: true,
                confirmed: true,
            })
        }
    }
}

fn build_calendar_create_options(
    args: &commands::CalendarCreateArgs,
) -> CliResult<nextcloud::CalendarCreateOptions> {
    let uid = format!(
        "nextcloud-cli-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );

    if args.all_day {
        let starts = parse_calendar_date_only(&args.starts_at)?;
        let ends = parse_calendar_date_only(&args.ends_at)?;
        if ends <= starts {
            return Err(CliError::InvalidCalendarEventRange);
        }
        return Ok(nextcloud::CalendarCreateOptions {
            calendar: args.calendar.clone(),
            uid,
            summary: args.summary.clone(),
            starts_at: starts.to_string(),
            ends_at: ends.to_string(),
            location: args.location.clone(),
            description: args.description.clone(),
            all_day: true,
        });
    }

    let starts = parse_calendar_bound(&args.starts_at)?;
    let ends = parse_calendar_bound(&args.ends_at)?;
    if ends <= starts {
        return Err(CliError::InvalidCalendarEventRange);
    }
    Ok(nextcloud::CalendarCreateOptions {
        calendar: args.calendar.clone(),
        uid,
        summary: args.summary.clone(),
        starts_at: starts.to_rfc3339(),
        ends_at: ends.to_rfc3339(),
        location: args.location.clone(),
        description: args.description.clone(),
        all_day: false,
    })
}

fn parse_calendar_date_only(value: &str) -> CliResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| CliError::InvalidCalendarBound {
        value: value.to_owned(),
    })
}

async fn handle_contacts(
    command: ContactsCommand,
    selected_profile: Option<&str>,
    store: &ConfigStore,
    credential_store: &CredentialStore,
) -> CliResult<Value> {
    let profile = store.selected_profile(selected_profile)?;
    let app_password = credential_store.get_app_password(&profile.credential)?;
    let client = NextcloudClient::from_profile(&profile, Some(app_password))?;
    let contacts = nextcloud::ContactsClient::new(client, profile.username.clone());

    match command {
        ContactsCommand::Search(args) => {
            if args.limit == 0 || args.limit > 100 {
                return Err(CliError::InvalidLimit { value: args.limit });
            }
            let results = contacts
                .search(&nextcloud::ContactSearchOptions {
                    query: args.query.clone(),
                    limit: args.limit,
                    addressbook: args.addressbook.clone(),
                })
                .await?;
            let count = results.len();
            json_value(ContactsSearchOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                query: args.query,
                addressbook: args.addressbook,
                limit: args.limit,
                contacts: results,
                count,
            })
        }
        ContactsCommand::Create(args) => {
            let contact = build_contact_create_options(&args);
            if args.dry_run {
                record(
                    store.paths(),
                    &command_executed(
                        &profile.name,
                        profile.server.as_str(),
                        "contacts.create",
                        true,
                        "PUT",
                        "/remote.php/dav/addressbooks/users/{username}/{addressbook}/{uid}.vcf",
                        target([
                            ("addressbook", json!(contact.addressbook.clone())),
                            ("uid", json!(contact.uid.clone())),
                            ("full_name", json!(contact.full_name.clone())),
                            ("email_count", json!(contact.emails.len())),
                            ("phone_count", json!(contact.phones.len())),
                            (
                                "organization_present",
                                json!(contact.organization.is_some()),
                            ),
                        ]),
                    ),
                );
                return json_value(ContactsCreateOutput {
                    profile: profile.name,
                    server: profile.server.to_string(),
                    dry_run: true,
                    created: false,
                    contact: ContactsCreatePreview {
                        uid: contact.uid,
                        addressbook: contact.addressbook,
                        full_name: contact.full_name,
                        email_count: contact.emails.len(),
                        phone_count: contact.phones.len(),
                        organization_present: contact.organization.is_some(),
                    },
                });
            }

            let created = contacts.create(&contact).await?;
            record(
                store.paths(),
                &command_executed(
                    &profile.name,
                    profile.server.as_str(),
                    "contacts.create",
                    false,
                    "PUT",
                    "/remote.php/dav/addressbooks/users/{username}/{addressbook}/{uid}.vcf",
                    target([
                        ("addressbook", json!(contact.addressbook)),
                        ("uid", json!(contact.uid)),
                        ("full_name", json!(contact.full_name)),
                        ("email_count", json!(contact.emails.len())),
                        ("phone_count", json!(contact.phones.len())),
                        (
                            "organization_present",
                            json!(contact.organization.is_some()),
                        ),
                    ]),
                ),
            );
            json_value(json!({
                "profile": profile.name,
                "server": profile.server.to_string(),
                "dry_run": false,
                "created": true,
                "contact": created,
            }))
        }
        ContactsCommand::Delete(args) => {
            if args.dry_run {
                record(
                    store.paths(),
                    &command_executed(
                        &profile.name,
                        profile.server.as_str(),
                        "contacts.delete",
                        true,
                        "DELETE",
                        "/remote.php/dav/addressbooks/users/{username}/{addressbook}/{uid}.vcf",
                        target([
                            ("addressbook", json!(args.addressbook.clone())),
                            ("uid", json!(args.uid.clone())),
                        ]),
                    ),
                );
                return json_value(DeleteObjectOutput {
                    profile: profile.name,
                    server: profile.server.to_string(),
                    command: "contacts delete".to_owned(),
                    object_type: "contact".to_owned(),
                    collection: args.addressbook,
                    id: args.uid,
                    dry_run: true,
                    deleted: false,
                    confirmed: false,
                });
            }
            if !args.yes {
                return Err(CliError::ConfirmationRequired);
            }
            contacts.delete(&args.addressbook, &args.uid).await?;
            record(
                store.paths(),
                &command_executed(
                    &profile.name,
                    profile.server.as_str(),
                    "contacts.delete",
                    false,
                    "DELETE",
                    "/remote.php/dav/addressbooks/users/{username}/{addressbook}/{uid}.vcf",
                    target([
                        ("addressbook", json!(args.addressbook.clone())),
                        ("uid", json!(args.uid.clone())),
                    ]),
                ),
            );
            json_value(DeleteObjectOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                command: "contacts delete".to_owned(),
                object_type: "contact".to_owned(),
                collection: args.addressbook,
                id: args.uid,
                dry_run: false,
                deleted: true,
                confirmed: true,
            })
        }
    }
}

fn build_contact_create_options(
    args: &commands::ContactsCreateArgs,
) -> nextcloud::ContactCreateOptions {
    let uid = format!(
        "nextcloud-cli-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    nextcloud::ContactCreateOptions {
        addressbook: args.addressbook.clone(),
        uid,
        full_name: args.full_name.clone(),
        emails: args.email.clone(),
        phones: args.phone.clone(),
        organization: args.organization.clone(),
    }
}

async fn handle_activity(
    command: ActivityCommand,
    selected_profile: Option<&str>,
    store: &ConfigStore,
    credential_store: &CredentialStore,
) -> CliResult<Value> {
    let profile = store.selected_profile(selected_profile)?;
    let app_password = credential_store.get_app_password(&profile.credential)?;
    let client = NextcloudClient::from_profile(&profile, Some(app_password))?;
    let activity = nextcloud::ActivityClient::new(client);

    match command {
        ActivityCommand::Recent(args) => {
            if args.limit == 0 || args.limit > 100 {
                return Err(CliError::InvalidLimit { value: args.limit });
            }
            let activities = activity
                .recent(&nextcloud::ActivityRecentOptions { limit: args.limit })
                .await?;
            let count = activities.len();
            json_value(ActivityRecentOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                limit: args.limit,
                activities,
                count,
            })
        }
    }
}

async fn handle_notes(
    command: NotesCommand,
    selected_profile: Option<&str>,
    store: &ConfigStore,
    credential_store: &CredentialStore,
) -> CliResult<Value> {
    let profile = store.selected_profile(selected_profile)?;
    let app_password = credential_store.get_app_password(&profile.credential)?;
    let client = NextcloudClient::from_profile(&profile, Some(app_password))?;
    let notes_client = nextcloud::NotesClient::new(client);

    match command {
        NotesCommand::List(args) => {
            if args.limit == 0 || args.limit > 100 {
                return Err(CliError::InvalidLimit { value: args.limit });
            }
            let notes = match notes_client
                .list(&nextcloud::NotesListOptions {
                    category: args.category.clone(),
                    exclude_content: args.exclude_content,
                    limit: args.limit,
                })
                .await
            {
                Ok(notes) => notes,
                Err(nextcloud::Error::HttpStatus { status, .. }) if status.as_u16() == 404 => {
                    return Err(CliError::AppUnavailable {
                        app: "notes".to_owned(),
                        api_source: "notes".to_owned(),
                    });
                }
                Err(error) => return Err(error.into()),
            };
            let count = notes.len();
            json_value(NotesListOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                category: args.category,
                exclude_content: args.exclude_content,
                limit: args.limit,
                notes,
                count,
            })
        }
        NotesCommand::Create(args) => {
            let note = build_note_create_options(&args)?;
            if args.dry_run {
                record(
                    store.paths(),
                    &command_executed(
                        &profile.name,
                        profile.server.as_str(),
                        "notes.create",
                        true,
                        "POST",
                        "/index.php/apps/notes/api/v1/notes",
                        target([
                            ("title", json!(note.title.clone())),
                            ("category", json!(note.category.clone())),
                            ("content_present", json!(note.content.is_some())),
                            (
                                "content_bytes",
                                json!(note.content.as_ref().map(|content| content.len())),
                            ),
                        ]),
                    ),
                );
                return json_value(NotesCreateOutput {
                    profile: profile.name,
                    server: profile.server.to_string(),
                    dry_run: true,
                    created: false,
                    note: NoteWriteSummary::from_create_options(&note),
                });
            }

            let created = match notes_client.create(&note).await {
                Ok(note) => note,
                Err(nextcloud::Error::HttpStatus { status, .. }) if status.as_u16() == 404 => {
                    return Err(CliError::AppUnavailable {
                        app: "notes".to_owned(),
                        api_source: "notes".to_owned(),
                    });
                }
                Err(error) => return Err(error.into()),
            };
            record(
                store.paths(),
                &command_executed(
                    &profile.name,
                    profile.server.as_str(),
                    "notes.create",
                    false,
                    "POST",
                    "/index.php/apps/notes/api/v1/notes",
                    target([
                        ("note_id", json!(created.id.clone())),
                        ("title", json!(created.title.clone())),
                        ("category", json!(created.category.clone())),
                        ("content_present", json!(created.content.is_some())),
                    ]),
                ),
            );
            json_value(NotesCreateOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                dry_run: false,
                created: true,
                note: NoteWriteSummary::from_note(&created),
            })
        }
        NotesCommand::Delete(args) => {
            if args.dry_run {
                record(
                    store.paths(),
                    &command_executed(
                        &profile.name,
                        profile.server.as_str(),
                        "notes.delete",
                        true,
                        "DELETE",
                        "/index.php/apps/notes/api/v1/notes/{note-id}",
                        target([("note_id", json!(args.note_id.clone()))]),
                    ),
                );
                return json_value(DeleteObjectOutput {
                    profile: profile.name,
                    server: profile.server.to_string(),
                    command: "notes delete".to_owned(),
                    object_type: "note".to_owned(),
                    collection: "notes".to_owned(),
                    id: args.note_id,
                    dry_run: true,
                    deleted: false,
                    confirmed: false,
                });
            }

            if !args.yes {
                return Err(CliError::ConfirmationRequired);
            }

            match notes_client.delete(&args.note_id).await {
                Ok(()) => {}
                Err(nextcloud::Error::HttpStatus { status, .. }) if status.as_u16() == 404 => {
                    return Err(CliError::AppUnavailable {
                        app: "notes".to_owned(),
                        api_source: "notes".to_owned(),
                    });
                }
                Err(error) => return Err(error.into()),
            }
            record(
                store.paths(),
                &command_executed(
                    &profile.name,
                    profile.server.as_str(),
                    "notes.delete",
                    false,
                    "DELETE",
                    "/index.php/apps/notes/api/v1/notes/{note-id}",
                    target([("note_id", json!(args.note_id.clone()))]),
                ),
            );
            json_value(DeleteObjectOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                command: "notes delete".to_owned(),
                object_type: "note".to_owned(),
                collection: "notes".to_owned(),
                id: args.note_id,
                dry_run: false,
                deleted: true,
                confirmed: true,
            })
        }
    }
}

fn build_note_create_options(
    args: &commands::NotesCreateArgs,
) -> CliResult<nextcloud::NotesCreateOptions> {
    let content = resolve_note_content(args.content.clone(), args.from_file.as_deref())?;
    Ok(nextcloud::NotesCreateOptions {
        title: args.title.clone(),
        content,
        category: args.category.clone(),
    })
}

fn resolve_note_content(
    inline: Option<String>,
    from_file: Option<&Path>,
) -> CliResult<Option<String>> {
    if inline.is_some() && from_file.is_some() {
        return Err(CliError::ConflictingNoteContentSources);
    }
    if let Some(path) = from_file {
        return fs::read_to_string(path)
            .map(Some)
            .map_err(|source| CliError::LocalFileRead {
                path: path.to_path_buf(),
                source,
            });
    }
    Ok(inline)
}

async fn handle_deck(
    command: DeckCommand,
    selected_profile: Option<&str>,
    store: &ConfigStore,
    credential_store: &CredentialStore,
) -> CliResult<Value> {
    let profile = store.selected_profile(selected_profile)?;
    let app_password = credential_store.get_app_password(&profile.credential)?;
    let client = NextcloudClient::from_profile(&profile, Some(app_password))?;
    let deck = nextcloud::DeckClient::new(client);

    match command {
        DeckCommand::Boards(args) => {
            let boards = match deck.boards().await {
                Ok(boards) => boards,
                Err(nextcloud::Error::HttpStatus { status, .. }) if status.as_u16() == 404 => {
                    return Err(CliError::AppUnavailable {
                        app: "deck".to_owned(),
                        api_source: "deck".to_owned(),
                    });
                }
                Err(error) => return Err(error.into()),
            };
            let count = boards.len();
            json_value(DeckBoardsOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                details: args.details,
                boards,
                count,
            })
        }
        DeckCommand::Cards(args) => {
            let cards = match deck
                .cards(&nextcloud::DeckCardsOptions {
                    board_id: args.board.clone(),
                    include_archived: args.include_archived,
                })
                .await
            {
                Ok(cards) => cards,
                Err(nextcloud::Error::HttpStatus { status, .. }) if status.as_u16() == 404 => {
                    return Err(CliError::AppUnavailable {
                        app: "deck".to_owned(),
                        api_source: "deck".to_owned(),
                    });
                }
                Err(error) => return Err(error.into()),
            };
            let count = cards.len();
            json_value(DeckCardsOutput {
                profile: profile.name,
                server: profile.server.to_string(),
                board_id: args.board,
                include_archived: args.include_archived,
                cards,
                count,
            })
        }
    }
}

fn resolve_calendar_range(args: &CalendarEventsArgs) -> CliResult<CalendarRangeOutput> {
    if let Some(date) = &args.date {
        return resolve_calendar_date(date);
    }

    if let Some(range) = &args.range {
        return resolve_calendar_duration(range);
    }

    if args.from.is_some() || args.to.is_some() {
        let from = args
            .from
            .as_deref()
            .map(parse_calendar_bound)
            .transpose()?
            .unwrap_or_else(Utc::now);
        let to = args
            .to
            .as_deref()
            .map(parse_calendar_bound)
            .transpose()?
            .unwrap_or_else(|| from + chrono::Duration::days(7));
        return Ok(CalendarRangeOutput::new(from, to));
    }

    resolve_calendar_duration("7d")
}

fn resolve_calendar_date(value: &str) -> CliResult<CalendarRangeOutput> {
    let date = if value == "today" {
        Local::now().date_naive()
    } else {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
            CliError::InvalidCalendarBound {
                value: value.to_owned(),
            }
        })?
    };
    let from = local_midnight_utc(date);
    let to_date =
        date.checked_add_days(Days::new(1))
            .ok_or_else(|| CliError::InvalidCalendarBound {
                value: value.to_owned(),
            })?;
    let to = local_midnight_utc(to_date);
    Ok(CalendarRangeOutput::new(from, to))
}

fn local_midnight_utc(date: NaiveDate) -> DateTime<Utc> {
    let midnight = date.and_hms_opt(0, 0, 0).expect("valid midnight");
    match Local.from_local_datetime(&midnight) {
        LocalResult::Single(datetime) => datetime.with_timezone(&Utc),
        LocalResult::Ambiguous(earliest, _) => earliest.with_timezone(&Utc),
        LocalResult::None => midnight.and_utc(),
    }
}

fn resolve_calendar_duration(value: &str) -> CliResult<CalendarRangeOutput> {
    let days = value
        .strip_suffix('d')
        .and_then(|days| days.parse::<i64>().ok())
        .filter(|days| *days > 0)
        .ok_or_else(|| CliError::InvalidCalendarRange {
            value: value.to_owned(),
        })?;
    let from = Utc::now();
    let to = from + chrono::Duration::days(days);
    Ok(CalendarRangeOutput::new(from, to))
}

fn parse_calendar_bound(value: &str) -> CliResult<DateTime<Utc>> {
    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        return Ok(datetime.with_timezone(&Utc));
    }
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        CliError::InvalidCalendarBound {
            value: value.to_owned(),
        }
    })?;
    Ok(date.and_hms_opt(0, 0, 0).expect("valid midnight").and_utc())
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
struct ProfilePolicyOutput {
    profile: String,
    server: String,
    policy: nextcloud::ProfilePolicy,
    updated: bool,
}

#[derive(Debug, Serialize)]
struct ProfilePolicySetOutput {
    profile: String,
    server: String,
    policy: nextcloud::ProfilePolicy,
    updated: bool,
    changed: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ProfilePolicyResetOutput {
    profile: String,
    server: String,
    policy: nextcloud::ProfilePolicy,
    updated: bool,
    reset: bool,
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
struct AuthAppPasswordOutput {
    profile: String,
    server: String,
    username: String,
    auth_type: String,
    credential_backend: String,
    credential_stored: bool,
    password_source: String,
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
struct FilesUploadOutput {
    profile: String,
    server: String,
    remote: String,
    local: String,
    bytes_uploaded: u64,
    etag: Option<String>,
    overwritten: bool,
    entry: Option<nextcloud::WebDavEntry>,
}

#[derive(Debug, Serialize)]
struct FilesDownloadOutput {
    profile: String,
    server: String,
    remote: String,
    local: String,
    bytes_written: u64,
    content_length: Option<u64>,
    overwritten: bool,
}

#[derive(Debug, Serialize)]
struct FilesDeleteOutput {
    profile: String,
    server: String,
    path: String,
    dry_run: bool,
    deleted: bool,
    confirmed: bool,
    entry: Option<nextcloud::WebDavEntry>,
}

#[derive(Debug, Serialize)]
struct CalendarRangeOutput {
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

impl CalendarRangeOutput {
    fn new(from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        Self { from, to }
    }
}

#[derive(Debug, Serialize)]
struct CalendarEventsOutput {
    profile: String,
    server: String,
    calendar: Option<String>,
    range: CalendarRangeOutput,
    events: Vec<nextcloud::CalendarEvent>,
    count: usize,
}

#[derive(Debug, Serialize)]
struct CalendarCreateOutput {
    profile: String,
    server: String,
    dry_run: bool,
    created: bool,
    event: CalendarCreatePreview,
}

#[derive(Debug, Serialize)]
struct CalendarCreatePreview {
    uid: String,
    calendar: String,
    summary: String,
    starts_at: String,
    ends_at: String,
    all_day: bool,
    location: Option<String>,
    description_present: bool,
}

#[derive(Debug, Serialize)]
struct ContactsSearchOutput {
    profile: String,
    server: String,
    query: String,
    addressbook: Option<String>,
    limit: u32,
    contacts: Vec<nextcloud::Contact>,
    count: usize,
}

#[derive(Debug, Serialize)]
struct ContactsCreateOutput {
    profile: String,
    server: String,
    dry_run: bool,
    created: bool,
    contact: ContactsCreatePreview,
}

#[derive(Debug, Serialize)]
struct ContactsCreatePreview {
    uid: String,
    addressbook: String,
    full_name: String,
    email_count: usize,
    phone_count: usize,
    organization_present: bool,
}

#[derive(Debug, Serialize)]
struct ActivityRecentOutput {
    profile: String,
    server: String,
    limit: u32,
    activities: Vec<nextcloud::ActivityItem>,
    count: usize,
}

#[derive(Debug, Serialize)]
struct NotesListOutput {
    profile: String,
    server: String,
    category: Option<String>,
    exclude_content: bool,
    limit: u32,
    notes: Vec<nextcloud::Note>,
    count: usize,
}

#[derive(Debug, Serialize)]
struct NotesCreateOutput {
    profile: String,
    server: String,
    dry_run: bool,
    created: bool,
    note: NoteWriteSummary,
}

#[derive(Debug, Serialize)]
struct NoteWriteSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    etag: Option<String>,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified_at: Option<String>,
    content_present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_bytes: Option<usize>,
}

impl NoteWriteSummary {
    fn from_create_options(options: &nextcloud::NotesCreateOptions) -> Self {
        Self {
            id: None,
            etag: None,
            title: options.title.clone(),
            category: options.category.clone(),
            modified_at: None,
            content_present: options.content.is_some(),
            content_bytes: options.content.as_ref().map(|content| content.len()),
        }
    }

    fn from_note(note: &nextcloud::Note) -> Self {
        Self {
            id: Some(note.id.clone()),
            etag: note.etag.clone(),
            title: note.title.clone().unwrap_or_default(),
            category: note.category.clone(),
            modified_at: note.modified_at.clone(),
            content_present: note.content.is_some(),
            content_bytes: note.content.as_ref().map(|content| content.len()),
        }
    }
}

#[derive(Debug, Serialize)]
struct DeckBoardsOutput {
    profile: String,
    server: String,
    details: bool,
    boards: Vec<nextcloud::DeckBoard>,
    count: usize,
}

#[derive(Debug, Serialize)]
struct DeckCardsOutput {
    profile: String,
    server: String,
    board_id: String,
    include_archived: bool,
    cards: Vec<nextcloud::DeckCard>,
    count: usize,
}

#[derive(Debug, Serialize)]
struct DeleteObjectOutput {
    profile: String,
    server: String,
    command: String,
    object_type: String,
    collection: String,
    id: String,
    dry_run: bool,
    deleted: bool,
    confirmed: bool,
}

#[derive(Debug, Serialize)]
struct ShareDeleteOutput {
    profile: String,
    server: String,
    share_id: String,
    dry_run: bool,
    deleted: bool,
    confirmed: bool,
    command: String,
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
