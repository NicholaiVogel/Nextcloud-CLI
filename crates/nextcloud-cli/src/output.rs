use serde::Serialize;
use serde_json::{Value, json};

use crate::commands::OutputFormat;
use crate::error::{CliError, CliResult};

pub fn print_success<T>(value: &T, format: OutputFormat) -> CliResult<()>
where
    T: Serialize,
{
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        OutputFormat::Human => {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
    }
    Ok(())
}

pub fn print_error(error: &CliError) {
    let envelope = error_envelope(error);
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| {
            r#"{"error":{"code":"serialize_error_failed","message":"failed to serialize error"}}"#.to_owned()
        })
    );
}

pub fn error_envelope(error: &CliError) -> Value {
    json!({
        "error": {
            "code": error.code(),
            "message": error.to_string()
        }
    })
}
