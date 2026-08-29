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

    /// Add certificates from this PEM bundle to the system trust roots. Hostname
    /// verification remains enabled.
    #[arg(long, global = true, env = "NEXTCLOUD_CLI_CA_BUNDLE")]
    pub ca_bundle: Option<PathBuf>,

    /// Explicitly disable TLS certificate and hostname verification for this
    /// invocation. Rejected for agent-managed profiles.
    #[arg(long, global = true, default_value_t = false)]
    pub insecure: bool,

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
    Index(IndexCommand),

    #[command(subcommand)]
    Shares(SharesCommand),

    #[command(subcommand)]
    Calendar(CalendarCommand),

    #[command(subcommand)]
    Contacts(ContactsCommand),

    #[command(subcommand)]
    Activity(ActivityCommand),

    #[command(subcommand)]
    Notes(NotesCommand),

    #[command(subcommand)]
    Deck(DeckCommand),

    #[command(subcommand)]
    Smoke(SmokeCommand),

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
    SearchImage(FilesSearchImageArgs),
    Stat(FilesPathArgs),
    Mkdir(FilesMkdirArgs),
    Upload(FilesUploadArgs),
    Download(FilesDownloadArgs),
    Move(FilesTransferArgs),
    Rename(FilesRenameArgs),
    Copy(FilesTransferArgs),
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
pub struct FilesSearchImageArgs {
    pub query: PathBuf,

    #[arg(long, default_value = "/")]
    pub path: String,

    #[arg(long, value_enum, default_value_t = IndexMedia::All)]
    pub media: IndexMedia,

    #[arg(long, default_value_t = 25)]
    pub limit: u32,

    #[arg(long, default_value_t = 3)]
    pub video_candidates: u32,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum IndexMedia {
    Images,
    Videos,
    All,
}

#[derive(Debug, Subcommand)]
pub enum IndexCommand {
    Status,
    Build(IndexBuildArgs),
    Update(IndexUpdateArgs),
    Clear(IndexClearArgs),
}

#[derive(Debug, Args)]
pub struct IndexBuildArgs {
    #[arg(long, default_value = "/")]
    pub path: String,

    #[arg(long, value_enum, default_value_t = IndexMedia::All)]
    pub media: IndexMedia,

    #[arg(long, default_value_t = 10_000)]
    pub max_files: u32,

    #[arg(long, default_value_t = 1)]
    pub video_sample_rate: u32,
}

#[derive(Debug, Args)]
pub struct IndexUpdateArgs {
    #[arg(long, default_value = "/")]
    pub path: String,

    #[arg(long, value_enum, default_value_t = IndexMedia::All)]
    pub media: IndexMedia,

    #[arg(long, default_value_t = 10_000)]
    pub max_files: u32,

    #[arg(long, default_value_t = 1)]
    pub video_sample_rate: u32,
}

#[derive(Debug, Args)]
pub struct IndexClearArgs {
    #[arg(long, default_value_t = false)]
    pub yes: bool,
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
pub struct FilesTransferArgs {
    pub from: String,

    pub to: String,

    #[arg(long, default_value_t = false)]
    pub overwrite: bool,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct FilesRenameArgs {
    pub path: String,

    pub new_name: String,

    #[arg(long, default_value_t = false)]
    pub overwrite: bool,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
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
    Create(CalendarCreateArgs),
    Delete(CalendarDeleteArgs),
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

#[derive(Debug, Args)]
pub struct CalendarCreateArgs {
    #[arg(long)]
    pub calendar: String,

    #[arg(long)]
    pub summary: String,

    #[arg(long)]
    pub starts_at: String,

    #[arg(long)]
    pub ends_at: String,

    #[arg(long)]
    pub location: Option<String>,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(long, default_value_t = false)]
    pub all_day: bool,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct CalendarDeleteArgs {
    #[arg(long)]
    pub calendar: String,

    pub uid: String,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Debug, Subcommand)]
pub enum ContactsCommand {
    Search(ContactsSearchArgs),
    Create(ContactsCreateArgs),
    Delete(ContactsDeleteArgs),
}

#[derive(Debug, Args)]
pub struct ContactsSearchArgs {
    pub query: String,

    #[arg(long, default_value_t = 25)]
    pub limit: u32,

    #[arg(long)]
    pub addressbook: Option<String>,
}

#[derive(Debug, Args)]
pub struct ContactsCreateArgs {
    #[arg(long)]
    pub addressbook: String,

    #[arg(long)]
    pub full_name: String,

    #[arg(long)]
    pub email: Vec<String>,

    #[arg(long)]
    pub phone: Vec<String>,

    #[arg(long)]
    pub organization: Option<String>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct ContactsDeleteArgs {
    #[arg(long)]
    pub addressbook: String,

    pub uid: String,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Debug, Subcommand)]
pub enum ActivityCommand {
    Recent(ActivityRecentArgs),
}

#[derive(Debug, Args)]
pub struct ActivityRecentArgs {
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}

#[derive(Debug, Subcommand)]
pub enum NotesCommand {
    List(NotesListArgs),
    Create(NotesCreateArgs),
    Update(NotesUpdateArgs),
    Delete(NotesDeleteArgs),
}

#[derive(Debug, Args)]
pub struct NotesListArgs {
    #[arg(long)]
    pub category: Option<String>,

    #[arg(long, default_value_t = false)]
    pub exclude_content: bool,

    #[arg(long, default_value_t = 25)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct NotesCreateArgs {
    #[arg(long)]
    pub title: String,

    #[arg(long)]
    pub content: Option<String>,

    #[arg(long)]
    pub from_file: Option<PathBuf>,

    #[arg(long)]
    pub category: Option<String>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct NotesUpdateArgs {
    pub note_id: String,

    #[arg(long)]
    pub title: Option<String>,

    #[arg(long)]
    pub content: Option<String>,

    #[arg(long)]
    pub from_file: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct NotesDeleteArgs {
    pub note_id: String,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Debug, Subcommand)]
pub enum DeckCommand {
    Boards(DeckBoardsArgs),
    #[command(subcommand)]
    Stacks(DeckStacksCommand),
    Cards(DeckCardsArgs),
}

#[derive(Debug, Subcommand)]
pub enum SmokeCommand {
    Run(SmokeRunArgs),
}

#[derive(Debug, Args)]
pub struct SmokeRunArgs {
    #[arg(long, default_value = "/")]
    pub files_path: String,

    #[arg(long, default_value_t = false)]
    pub skip_optional: bool,

    #[arg(long, default_value_t = 7)]
    pub calendar_days: u32,

    #[arg(long, default_value = "zzzz-nextcloud-cli-smoke-no-match")]
    pub contacts_query: String,
}

#[derive(Debug, Args)]
pub struct DeckBoardsArgs {
    #[command(subcommand)]
    pub command: Option<DeckBoardsCommand>,

    #[arg(long, default_value_t = false)]
    pub details: bool,
}

#[derive(Debug, Subcommand)]
pub enum DeckBoardsCommand {
    Create(DeckBoardCreateArgs),
}

#[derive(Debug, Args)]
pub struct DeckBoardCreateArgs {
    #[arg(long)]
    pub title: String,

    #[arg(long)]
    pub color: Option<String>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Subcommand)]
pub enum DeckStacksCommand {
    Create(DeckStackCreateArgs),
}

#[derive(Debug, Args)]
pub struct DeckStackCreateArgs {
    #[arg(long)]
    pub board: String,

    #[arg(long)]
    pub title: String,

    #[arg(long)]
    pub order: Option<i64>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct DeckCardsArgs {
    #[command(subcommand)]
    pub command: Option<DeckCardsCommand>,

    #[arg(long)]
    pub board: Option<String>,

    #[arg(long, default_value_t = false)]
    pub include_archived: bool,
}

#[derive(Debug, Subcommand)]
pub enum DeckCardsCommand {
    Create(DeckCardCreateArgs),
    Update(DeckCardUpdateArgs),
    Move(DeckCardMoveArgs),
    Archive(DeckCardRefArgs),
    Delete(DeckCardRefArgs),
}

#[derive(Debug, Args)]
pub struct DeckCardCreateArgs {
    #[arg(long)]
    pub board: String,

    #[arg(long)]
    pub stack: String,

    #[arg(long)]
    pub title: String,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(long)]
    pub due_at: Option<String>,

    #[arg(long)]
    pub order: Option<i64>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct DeckCardUpdateArgs {
    pub card_id: String,

    #[arg(long)]
    pub board: String,

    #[arg(long)]
    pub stack: String,

    #[arg(long)]
    pub title: Option<String>,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(long)]
    pub due_at: Option<String>,

    #[arg(long)]
    pub order: Option<i64>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct DeckCardMoveArgs {
    pub card_id: String,

    #[arg(long)]
    pub board: String,

    #[arg(long)]
    pub from_stack: String,

    #[arg(long)]
    pub to_stack: String,

    #[arg(long)]
    pub order: Option<i64>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct DeckCardRefArgs {
    pub card_id: String,

    #[arg(long)]
    pub board: String,

    #[arg(long)]
    pub stack: String,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Debug, Subcommand)]
pub enum UpdateCommand {
    Check,
}
