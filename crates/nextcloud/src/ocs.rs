use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OcsEnvelope<T> {
    pub ocs: OcsBody<T>,
}

impl<T> OcsEnvelope<T> {
    pub fn into_data(self) -> Result<T> {
        let meta = self.ocs.meta;
        if meta.is_success() {
            Ok(self.ocs.data)
        } else {
            Err(Error::OcsStatus {
                status: meta.status,
                status_code: meta.statuscode,
                message: meta
                    .message
                    .unwrap_or_else(|| "OCS request failed".to_owned()),
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OcsBody<T> {
    pub meta: OcsMeta,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcsMeta {
    pub status: String,
    pub statuscode: i64,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub totalitems: Option<String>,
    #[serde(default)]
    pub itemsperpage: Option<String>,
}

impl OcsMeta {
    fn is_success(&self) -> bool {
        (100..300).contains(&self.statuscode) || self.status.eq_ignore_ascii_case("ok")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_success_envelope() -> Result<()> {
        let raw = r#"{
            "ocs": {
                "meta": {
                    "status": "ok",
                    "statuscode": 200,
                    "message": "OK"
                },
                "data": {"value": "hello"}
            }
        }"#;

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Data {
            value: String,
        }

        let envelope: OcsEnvelope<Data> = serde_json::from_str(raw).expect("valid OCS JSON");
        assert_eq!(
            envelope.into_data()?,
            Data {
                value: "hello".to_owned()
            }
        );
        Ok(())
    }

    #[test]
    fn maps_failure_envelope_to_error() {
        let raw = r#"{
            "ocs": {
                "meta": {
                    "status": "failure",
                    "statuscode": 404,
                    "message": "not found"
                },
                "data": []
            }
        }"#;

        let envelope: OcsEnvelope<Vec<serde_json::Value>> =
            serde_json::from_str(raw).expect("valid OCS JSON");
        let error = envelope.into_data().expect_err("failure maps to error");

        match error {
            Error::OcsStatus {
                status,
                status_code,
                message,
            } => {
                assert_eq!(status, "failure");
                assert_eq!(status_code, 404);
                assert_eq!(message, "not found");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
