use nextcloud::models::{CommandOutputContract, CommandRef, CommandStability};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CommandSchema {
    pub schema_version: u32,
    pub default_format: String,
    pub error_shape: String,
    pub commands: Vec<CommandRef>,
}

pub fn command_schema() -> CommandSchema {
    CommandSchema {
        schema_version: 1,
        default_format: "json".to_owned(),
        error_shape: "{ error: { code, message, hint? } }".to_owned(),
        commands: vec![
            stable(
                "commands schema",
                "Print machine-readable command metadata.",
                "CommandSchema",
            ),
            stable(
                "config path",
                "Print resolved config, cache, audit, and credential paths.",
                "ConfigPaths",
            ),
            stable(
                "config show",
                "Print redacted CLI configuration.",
                "CliConfig",
            ),
            stable(
                "config doctor",
                "Verify local config directories and files.",
                "ConfigDoctor",
            ),
            stable(
                "auth login",
                "Run Nextcloud Login Flow v2 and store the returned app password.",
                "AuthLoginResult",
            ),
            stable(
                "auth app-password",
                "Create and store a Nextcloud app password from headless account-password auth.",
                "AuthAppPasswordResult",
            ),
            stable(
                "auth add",
                "Add an app-password authenticated profile.",
                "AuthAddResult",
            ),
            stable(
                "auth status",
                "Show selected profile authentication status.",
                "AuthStatus",
            ),
            stable("profiles list", "List configured profiles.", "ProfilesList"),
            stable("profiles show", "Show one configured profile.", "Profile"),
            stable(
                "profiles set-default",
                "Set the default profile.",
                "ProfileDefaultResult",
            ),
            stable(
                "profiles policy show",
                "Show local safety policy for one configured profile.",
                "ProfilePolicy",
            ),
            stable(
                "profiles policy set",
                "Update local safety policy for one configured profile.",
                "ProfilePolicySetResult",
            ),
            stable(
                "profiles policy reset",
                "Reset local safety policy for one configured profile to defaults.",
                "ProfilePolicyResetResult",
            ),
            stable(
                "server status",
                "Fetch Nextcloud status.php for the selected profile.",
                "ServerStatus",
            ),
            stable(
                "server capabilities",
                "Fetch OCS cloud capabilities for the selected profile.",
                "ServerCapabilities",
            ),
            stable(
                "files list",
                "List a remote Nextcloud folder through WebDAV.",
                "FilesList",
            ),
            stable(
                "files search",
                "Search remote Nextcloud file names through WebDAV SEARCH.",
                "FilesSearchResult",
            ),
            stable(
                "files stat",
                "Fetch metadata for one remote Nextcloud path through WebDAV.",
                "WebDavEntry",
            ),
            stable(
                "files mkdir",
                "Create a remote folder through WebDAV, with dry-run support.",
                "FilesMkdirResult",
            ),
            stable(
                "files upload",
                "Upload a local file to a remote Nextcloud path through WebDAV.",
                "FilesUploadResult",
            ),
            stable(
                "files download",
                "Download a remote Nextcloud file to a local path through WebDAV.",
                "FilesDownloadResult",
            ),
            stable(
                "files delete",
                "Delete a remote Nextcloud path through WebDAV with explicit confirmation.",
                "FilesDeleteResult",
            ),
            stable(
                "shares list",
                "List shares through the Nextcloud OCS files sharing API.",
                "SharesListResult",
            ),
            stable(
                "shares create",
                "Create a public link share through the Nextcloud OCS files sharing API.",
                "SharesCreateResult",
            ),
            stable(
                "shares delete",
                "Delete a share through the Nextcloud OCS files sharing API with explicit confirmation.",
                "SharesDeleteResult",
            ),
            stable(
                "shares revoke",
                "Alias for shares delete with explicit confirmation.",
                "SharesDeleteResult",
            ),
            stable(
                "calendar events",
                "List calendar events in a CalDAV time range.",
                "CalendarEventsResult",
            ),
            stable(
                "calendar create",
                "Create a calendar event through CalDAV PUT, with dry-run support.",
                "CalendarCreateResult",
            ),
            stable(
                "calendar delete",
                "Delete a calendar event through CalDAV with explicit confirmation.",
                "CalendarDeleteResult",
            ),
            stable(
                "contacts search",
                "Search contacts through CardDAV addressbook-query.",
                "ContactsSearchResult",
            ),
            stable(
                "contacts create",
                "Create a contact through CardDAV PUT, with dry-run support.",
                "ContactsCreateResult",
            ),
            stable(
                "contacts delete",
                "Delete a contact through CardDAV with explicit confirmation.",
                "ContactsDeleteResult",
            ),
            stable(
                "activity recent",
                "List recent activity through the Nextcloud Activity OCS API.",
                "ActivityRecentResult",
            ),
            stable(
                "update check",
                "Report current version and placeholder update metadata.",
                "UpdateCheck",
            ),
        ],
    }
}

fn stable(name: &str, summary: &str, success_shape: &str) -> CommandRef {
    CommandRef {
        name: name.to_owned(),
        summary: summary.to_owned(),
        stability: CommandStability::Stable,
        output: CommandOutputContract {
            default_format: "json".to_owned(),
            success_shape: success_shape.to_owned(),
            error_shape: "{ error: { code, message, hint? } }".to_owned(),
        },
    }
}
