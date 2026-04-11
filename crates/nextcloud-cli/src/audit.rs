use std::fs;
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use nextcloud::ConfigPaths;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event: String,
    pub profile: String,
    pub server: String,
    pub command: String,
    pub dry_run: bool,
    pub request: AuditRequest,
    pub target: Value,
    pub result: AuditResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditRequest {
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditResult {
    pub ok: bool,
}

pub fn command_executed(
    profile: &str,
    server: &str,
    command: &str,
    dry_run: bool,
    method: &str,
    path: &str,
    target: Value,
) -> AuditEvent {
    AuditEvent {
        timestamp: Utc::now(),
        event: "command.executed".to_owned(),
        profile: profile.to_owned(),
        server: server.to_owned(),
        command: command.to_owned(),
        dry_run,
        request: AuditRequest {
            method: method.to_owned(),
            path: path.to_owned(),
        },
        target,
        result: AuditResult { ok: true },
    }
}

pub fn record(paths: &ConfigPaths, event: &AuditEvent) {
    let Some(path) = audit_log_path(paths) else {
        return;
    };

    if let Err(error) = append_event(&path, event) {
        eprintln!(
            "warning: failed to write nextcloud-cli audit log at {}: {error}",
            path.display()
        );
    }
}

fn audit_log_path(paths: &ConfigPaths) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("NEXTCLOUD_CLI_LOG_FILE")
        && !path.is_empty()
    {
        return Some(PathBuf::from(path));
    }

    let enabled = std::env::var("NEXTCLOUD_CLI_AUDIT")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);
    if !enabled {
        return None;
    }

    let date = Utc::now().format("%Y-%m-%d");
    Some(paths.audit_dir.join(format!("nextcloud-cli-{date}.jsonl")))
}

fn append_event(path: &PathBuf, event: &AuditEvent) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut event = event.clone();
    redact_value(&mut event.target);
    serde_json::to_writer(&mut file, &event)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn redact_value(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if is_secret_key(key) {
                    *value = Value::String("[redacted]".to_owned());
                } else {
                    redact_value(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_value(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn is_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("password")
        || key.contains("secret")
        || key.contains("token")
        || key.contains("authorization")
        || key == "auth"
}

pub fn target(values: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut object = serde_json::Map::new();
    for (key, value) in values {
        object.insert(key.to_owned(), value);
    }
    Value::Object(object)
}

pub fn remote_path_target(path: &str) -> Value {
    json!({ "remote_path": path })
}

pub fn share_id_target(share_id: &str) -> Value {
    json!({ "share_id": share_id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn record_writes_jsonl_when_log_file_is_set() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let temp = TempDir::new()?;
        let paths = ConfigPaths::from_override(Some(temp.path().join("config")))?;
        let log = temp.path().join("audit.jsonl");

        unsafe {
            std::env::set_var("NEXTCLOUD_CLI_LOG_FILE", &log);
        }
        record(
            &paths,
            &command_executed(
                "personal",
                "https://cloud.example.com/",
                "shares.create",
                true,
                "POST",
                "/ocs/v2.php/apps/files_sharing/api/v1/shares",
                json!({
                    "remote_path": "/Documents/report.pdf",
                    "password": "should-not-be-passed-to-audit",
                }),
            ),
        );
        unsafe {
            std::env::remove_var("NEXTCLOUD_CLI_LOG_FILE");
        }

        let raw = fs::read_to_string(log)?;
        assert!(raw.contains("\"command\":\"shares.create\""));
        assert!(raw.contains("\"dry_run\":true"));
        assert!(!raw.contains("should-not-be-passed-to-audit"));
        assert!(raw.contains("[redacted]"));
        Ok(())
    }

    #[test]
    fn record_is_disabled_by_default() -> Result<(), Box<dyn std::error::Error>> {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let temp = TempDir::new()?;
        let paths = ConfigPaths::from_override(Some(temp.path().join("config")))?;
        unsafe {
            std::env::remove_var("NEXTCLOUD_CLI_LOG_FILE");
            std::env::remove_var("NEXTCLOUD_CLI_AUDIT");
        }

        record(
            &paths,
            &command_executed(
                "personal",
                "https://cloud.example.com/",
                "files.mkdir",
                true,
                "MKCOL",
                "/remote.php/dav/files/nicholai/example",
                remote_path_target("/example"),
            ),
        );

        assert!(!paths.audit_dir.exists());
        Ok(())
    }
}
