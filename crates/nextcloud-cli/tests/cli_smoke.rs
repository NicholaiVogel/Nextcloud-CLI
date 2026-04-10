use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn commands_schema_is_json() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::cargo_bin("nextcloud-cli")?
        .args(["commands", "schema"])
        .output()?;
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["schema_version"], 1);
    assert!(value["commands"].as_array().expect("commands array").len() >= 8);
    Ok(())
}

#[test]
fn auth_add_persists_profile_without_printing_secret() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    Command::cargo_bin("nextcloud-cli")?
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
fn nxc_alias_runs_same_binary() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("nxc")?
        .args(["commands", "schema"])
        .assert()
        .success()
        .stdout(predicate::str::contains("commands schema"));
    Ok(())
}
