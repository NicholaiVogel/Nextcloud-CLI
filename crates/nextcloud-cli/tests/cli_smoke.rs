use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn commands_schema_is_json() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args(["commands", "schema"])
        .output()?;
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["schema_version"], 1);
    assert!(value["commands"].as_array().expect("commands array").len() >= 8);
    assert!(
        value["commands"]
            .as_array()
            .expect("commands array")
            .iter()
            .any(|command| command["name"] == "shares list")
    );
    Ok(())
}

#[test]
fn auth_add_persists_profile_without_printing_secret() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("personal"))
        .stdout(predicate::str::contains("super-secret").not());

    let config = std::fs::read_to_string(temp.path().join("config.json"))?;
    assert!(config.contains("personal"));
    assert!(!config.contains("super-secret"));

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "status",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("authenticated"));

    Ok(())
}

#[test]
fn auth_app_password_requires_password_source() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "app-password",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("missing_account_password"));

    Ok(())
}

#[test]
fn nxc_alias_runs_same_binary() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nxc")?
        .args(["commands", "schema"])
        .assert()
        .success()
        .stdout(predicate::str::contains("commands schema"));
    Ok(())
}

#[test]
fn files_rejects_dotdot_remote_paths_before_auth() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "files",
            "mkdir",
            "/Documents/../secret",
            "--dry-run",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid_remote_path"));

    Ok(())
}

#[test]
fn files_delete_requires_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "files",
            "delete",
            "/Documents/report.md",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("confirmation_required"));

    Ok(())
}

#[test]
fn files_search_rejects_unsupported_mode() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "files",
            "search",
            "report",
            "--search-mode",
            "content",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unsupported_search_mode"));

    Ok(())
}

#[test]
fn shares_list_help_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nextcloud-cli")?
        .args(["shares", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--shared-with-me"))
        .stdout(predicate::str::contains("--include-tags"));

    Ok(())
}

#[test]
fn shares_create_public_dry_run_does_not_print_password() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "shares",
            "create",
            "/Documents/report.pdf",
            "--public",
            "--password",
            "share-password",
            "--expire-date",
            "2026-05-01",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"dry_run\": true"))
        .stdout(predicate::str::contains("\"password_protected\": true"))
        .stdout(predicate::str::contains("share-password").not());

    Ok(())
}

#[test]
fn audit_log_records_write_dry_run_without_secret() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let config_dir = temp.path().join("config").display().to_string();
    let audit_log = temp.path().join("audit.jsonl");

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            &config_dir,
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .env("NEXTCLOUD_CLI_LOG_FILE", &audit_log)
        .args([
            "--config-dir",
            &config_dir,
            "--profile",
            "personal",
            "shares",
            "create",
            "/Documents/report.pdf",
            "--public",
            "--password",
            "share-password",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("share-password").not());

    let raw = std::fs::read_to_string(audit_log)?;
    assert!(raw.contains("\"command\":\"shares.create\""));
    assert!(raw.contains("\"dry_run\":true"));
    assert!(!raw.contains("share-password"));

    Ok(())
}

#[test]
fn shares_create_public_requires_confirmation_before_policy()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "shares",
            "create",
            "/Documents/report.pdf",
            "--public",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("confirmation_required"));

    Ok(())
}

#[test]
fn shares_create_public_enforces_profile_policy() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "shares",
            "create",
            "/Documents/report.pdf",
            "--public",
            "--yes",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("policy_denied"));

    Ok(())
}

#[test]
fn shares_delete_dry_run_does_not_require_network() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "shares",
            "delete",
            "123",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"share_id\": \"123\""))
        .stdout(predicate::str::contains("\"dry_run\": true"))
        .stdout(predicate::str::contains("\"deleted\": false"));

    Ok(())
}

#[test]
fn shares_delete_requires_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "shares",
            "delete",
            "123",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("confirmation_required"));

    Ok(())
}

#[test]
fn shares_revoke_requires_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "shares",
            "revoke",
            "123",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("confirmation_required"));

    Ok(())
}

#[test]
fn profile_policy_show_and_set_public_shares() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "profiles",
            "policy",
            "show",
            "personal",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"allow_public_shares\": false"));

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "profiles",
            "policy",
            "set",
            "personal",
            "--allow-public-shares",
            "true",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"allow_public_shares\": true"))
        .stdout(predicate::str::contains("allow_public_shares"));

    Ok(())
}

#[test]
fn profile_policy_reset_requires_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "profiles",
            "policy",
            "reset",
            "personal",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("confirmation_required"));

    Ok(())
}

#[test]
fn calendar_events_help_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nextcloud-cli")?
        .args(["calendar", "events", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--date"))
        .stdout(predicate::str::contains("--range"))
        .stdout(predicate::str::contains("--from"))
        .stdout(predicate::str::contains("--to"));

    Ok(())
}

#[test]
fn calendar_events_rejects_bad_range_before_auth() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "calendar",
            "events",
            "--range",
            "soon",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid_calendar_range"));

    Ok(())
}

#[test]
fn calendar_create_dry_run_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "calendar",
            "create",
            "--calendar",
            "personal",
            "--summary",
            "Test",
            "--starts-at",
            "2026-04-10T16:00:00Z",
            "--ends-at",
            "2026-04-10T17:00:00Z",
            "--description",
            "private details",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"dry_run\": true"))
        .stdout(predicate::str::contains("\"created\": false"))
        .stdout(predicate::str::contains("\"description_present\": true"))
        .stdout(predicate::str::contains("private details").not());

    Ok(())
}

#[test]
fn calendar_create_rejects_inverted_range() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "calendar",
            "create",
            "--calendar",
            "personal",
            "--summary",
            "Test",
            "--starts-at",
            "2026-04-10T17:00:00Z",
            "--ends-at",
            "2026-04-10T16:00:00Z",
            "--dry-run",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid_calendar_event_range"));

    Ok(())
}

#[test]
fn contacts_search_help_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nextcloud-cli")?
        .args(["contacts", "search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--limit"))
        .stdout(predicate::str::contains("--addressbook"));

    Ok(())
}

#[test]
fn contacts_search_rejects_bad_limit_before_network() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "contacts",
            "search",
            "Ada",
            "--limit",
            "0",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid_limit"));

    Ok(())
}

#[test]
fn contacts_create_dry_run_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "contacts",
            "create",
            "--addressbook",
            "contacts",
            "--full-name",
            "Ada Lovelace",
            "--email",
            "ada@example.com",
            "--phone",
            "+15555550100",
            "--organization",
            "Analytical Engine",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"dry_run\": true"))
        .stdout(predicate::str::contains("\"created\": false"))
        .stdout(predicate::str::contains("\"email_count\": 1"))
        .stdout(predicate::str::contains("\"phone_count\": 1"));

    Ok(())
}

#[test]
fn calendar_and_contacts_delete_require_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    for args in [
        vec!["calendar", "delete", "--calendar", "personal", "event-1"],
        vec![
            "contacts",
            "delete",
            "--addressbook",
            "contacts",
            "contact-1",
        ],
    ] {
        let mut command = Command::cargo_bin("nextcloud-cli")?;
        command
            .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
            .args([
                "--config-dir",
                temp.path().to_str().expect("utf8 path"),
                "--profile",
                "personal",
            ])
            .args(args)
            .assert()
            .failure()
            .code(2)
            .stderr(predicate::str::contains("confirmation_required"));
    }

    Ok(())
}

#[test]
fn calendar_and_contacts_delete_dry_run() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "calendar",
            "delete",
            "--calendar",
            "personal",
            "event-1",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"deleted\": false"))
        .stdout(predicate::str::contains(
            "\"object_type\": \"calendar_event\"",
        ));

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "contacts",
            "delete",
            "--addressbook",
            "contacts",
            "contact-1",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"deleted\": false"))
        .stdout(predicate::str::contains("\"object_type\": \"contact\""));

    Ok(())
}

#[test]
fn activity_recent_help_and_limit_validation() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nextcloud-cli")?
        .args(["activity", "recent", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--limit"));

    let temp = TempDir::new()?;
    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "activity",
            "recent",
            "--limit",
            "0",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid_limit"));

    Ok(())
}

#[test]
fn notes_list_help_and_limit_validation() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nextcloud-cli")?
        .args(["notes", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--category"))
        .stdout(predicate::str::contains("--exclude-content"))
        .stdout(predicate::str::contains("--limit"));

    let temp = TempDir::new()?;
    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "add",
            "--server",
            "https://cloud.example.com",
            "--user",
            "nicholai",
            "--profile",
            "personal",
            "--app-password",
            "super-secret",
        ])
        .assert()
        .success();

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "--profile",
            "personal",
            "notes",
            "list",
            "--limit",
            "0",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid_limit"));

    Ok(())
}

#[test]
fn deck_boards_help_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nextcloud-cli")?
        .args(["deck", "boards", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--details"));

    Ok(())
}

#[test]
fn deck_cards_help_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nextcloud-cli")?
        .args(["deck", "cards", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--board"))
        .stdout(predicate::str::contains("--include-archived"));

    Ok(())
}

#[test]
fn env_profile_selects_profile_when_flag_is_absent() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    for profile in ["personal", "work"] {
        Command::cargo_bin("nextcloud-cli")?
            .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
            .args([
                "--config-dir",
                temp.path().to_str().expect("utf8 path"),
                "auth",
                "add",
                "--server",
                "https://cloud.example.com",
                "--user",
                profile,
                "--profile",
                profile,
                "--app-password",
                "super-secret",
            ])
            .assert()
            .success();
    }

    Command::cargo_bin("nextcloud-cli")?
        .env("NEXTCLOUD_CLI_KEYRING_BACKEND", "file")
        .env("NEXTCLOUD_CLI_PROFILE", "work")
        .args([
            "--config-dir",
            temp.path().to_str().expect("utf8 path"),
            "auth",
            "status",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"profile\": \"work\""));

    Ok(())
}
