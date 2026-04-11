use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "nextcloud-cli",
    bin_name = "nextcloud-cli",
    version,
    about = "Client-side Nextcloud CLI for humans, scripts, and AI agents.",
    long_about = "Client-side Nextcloud CLI for humans, scripts, and AI agents.\n\nQuick start:\n  nextcloud-cli auth add --server https://cloud.example.com --user you --app-password $NEXTCLOUD_APP_PASSWORD\n  nextcloud-cli files list /\n\nThis project is unofficial and talks to existing Nextcloud client APIs."
)]
pub struct Cli {
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,

    #[arg(long, global = true)]
    pub profile: Option<String>,

    #[arg(long, global = true, env = "NEXTCLOUD_CLI_CONFIG_DIR")]
    pub config_dir: Option<PathBuf>,

    #[arg(long, global = true, default_value_t = false)]
    pub no_art: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Human,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(subcommand)]
    Commands(CommandsCommand),

    #[command(subcommand)]
    Config(ConfigCommand),

    #[command(subcommand)]
    Profiles(ProfilesCommand),

    #[command(subcommand)]
    Auth(AuthCommand),

    #[command(subcommand)]
    Server(ServerCommand),

    #[command(subcommand)]
    Files(FilesCommand),

    #[command(subcommand)]
    Shares(SharesCommand),

    #[command(subcommand)]
    Calendar(CalendarCommand),

    #[command(subcommand)]
    Contacts(ContactsCommand),

    #[command(subcommand)]
    Update(UpdateCommand),
}

#[derive(Debug, Subcommand)]
pub enum CommandsCommand {
    Schema,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    Path,
    Show,
    Doctor,
}

#[derive(Debug, Subcommand)]
pub enum ProfilesCommand {
    List,
    Show(ProfileNameArgs),
    SetDefault(ProfileNameArgs),
    #[command(subcommand)]
    Policy(ProfilePolicyCommand),
}

#[derive(Debug, Args)]
pub struct ProfileNameArgs {
    pub name: String,
}

#[derive(Debug, Subcommand)]
pub enum ProfilePolicyCommand {
    Show(ProfileNameArgs),
    Set(ProfilePolicySetArgs),
    Reset(ProfilePolicyResetArgs),
}

#[derive(Debug, Args)]
pub struct ProfilePolicySetArgs {
    pub name: String,

    #[arg(long)]
    pub agent_mode: Option<bool>,

    #[arg(long)]
    pub default_dry_run: Option<bool>,

    #[arg(long)]
    pub allow_destructive: Option<bool>,

    #[arg(long)]
    pub allow_public_shares: Option<bool>,
}

#[derive(Debug, Args)]
pub struct ProfilePolicyResetArgs {
    pub name: String,

    #[arg(long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Login(AuthLoginArgs),
    AppPassword(AuthAppPasswordArgs),
    Add(AuthAddArgs),
    Status,
}

#[derive(Debug, Args)]
pub struct AuthLoginArgs {
    #[arg(long)]
    pub server: String,

    #[arg(long)]
    pub profile: Option<String>,

    #[arg(long, default_value_t = 600)]
    pub timeout_seconds: u64,

    #[arg(long, default_value_t = 2)]
    pub poll_interval_seconds: u64,

    #[arg(long, default_value_t = false)]
    pub no_open: bool,

    #[arg(long, default_value_t = true)]
    pub set_default: bool,
}

#[derive(Debug, Args)]
pub struct AuthAppPasswordArgs {
    #[arg(long)]
    pub server: String,

    #[arg(long, alias = "username")]
    pub user: String,

    #[arg(long)]
    pub profile: Option<String>,

    #[arg(long, default_value_t = false)]
    pub password_stdin: bool,

    #[arg(long)]
    pub password_env: Option<String>,

    #[arg(long, default_value_t = true)]
    pub set_default: bool,
}

#[derive(Debug, Args)]
pub struct AuthAddArgs {
    #[arg(long)]
    pub server: String,

    #[arg(long, alias = "username")]
    pub user: String,

    #[arg(long)]
    pub profile: Option<String>,

    #[arg(long, env = "NEXTCLOUD_APP_PASSWORD")]
    pub app_password: Option<String>,

    #[arg(long, default_value_t = true)]
    pub set_default: bool,
}

#[derive(Debug, Subcommand)]
pub enum ServerCommand {
    Status,
    Capabilities(ServerCapabilitiesArgs),
}

#[derive(Debug, Args)]
pub struct ServerCapabilitiesArgs {
    #[arg(long, default_value_t = false)]
    pub refresh: bool,
}

#[derive(Debug, Subcommand)]
pub enum FilesCommand {
    List(FilesPathArgs),
    Search(FilesSearchArgs),
    Stat(FilesPathArgs),
    Mkdir(FilesMkdirArgs),
    Upload(FilesUploadArgs),
    Download(FilesDownloadArgs),
    Delete(FilesDeleteArgs),
}

#[derive(Debug, Args)]
pub struct FilesPathArgs {
    #[arg(default_value = "/")]
    pub path: String,
}

#[derive(Debug, Args)]
pub struct FilesSearchArgs {
    pub query: String,

    #[arg(long, default_value = "/")]
    pub path: String,

    #[arg(long, default_value_t = 25)]
    pub limit: u32,

    #[arg(long, default_value = "name")]
    pub search_mode: String,
}

#[derive(Debug, Args)]
pub struct FilesMkdirArgs {
    pub path: String,

    #[arg(long, default_value_t = false)]
    pub parents: bool,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct FilesUploadArgs {
    pub local: PathBuf,

    pub remote: String,

    #[arg(long, default_value_t = false)]
    pub overwrite: bool,

    #[arg(long)]
    pub content_type: Option<String>,
}

#[derive(Debug, Args)]
pub struct FilesDownloadArgs {
    pub remote: String,

    pub local: PathBuf,

    #[arg(long, default_value_t = false)]
    pub overwrite: bool,
}

#[derive(Debug, Args)]
pub struct FilesDeleteArgs {
    pub path: String,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Debug, Subcommand)]
pub enum SharesCommand {
    List(SharesListArgs),
    Create(SharesCreateArgs),
    Delete(SharesDeleteArgs),
    Revoke(SharesDeleteArgs),
}

#[derive(Debug, Args)]
pub struct SharesListArgs {
    #[arg(long)]
    pub path: Option<String>,

    #[arg(long, default_value_t = false)]
    pub shared_with_me: bool,

    #[arg(long, default_value_t = false)]
    pub include_tags: bool,
}

#[derive(Debug, Args)]
pub struct SharesCreateArgs {
    pub path: String,

    #[arg(long, default_value_t = false)]
    pub public: bool,

    #[arg(long)]
    pub password: Option<String>,

    #[arg(long)]
    pub expire_date: Option<String>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct SharesDeleteArgs {
    pub share_id: String,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Debug, Subcommand)]
pub enum CalendarCommand {
    Events(CalendarEventsArgs),
}

#[derive(Debug, Args)]
pub struct CalendarEventsArgs {
    #[arg(long)]
    pub date: Option<String>,

    #[arg(long)]
    pub range: Option<String>,

    #[arg(long)]
    pub from: Option<String>,

    #[arg(long)]
    pub to: Option<String>,

    #[arg(long)]
    pub calendar: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum ContactsCommand {
    Search(ContactsSearchArgs),
}

#[derive(Debug, Args)]
pub struct ContactsSearchArgs {
    pub query: String,

    #[arg(long, default_value_t = 25)]
    pub limit: u32,

    #[arg(long)]
    pub addressbook: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum UpdateCommand {
    Check,
}
