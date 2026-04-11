use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use url::form_urlencoded;

use crate::client::NextcloudClient;
use crate::error::Result;
use crate::ocs::OcsEnvelope;
use crate::webdav::normalize_remote_path;

const SHARES_ENDPOINT: &str = "ocs/v2.php/apps/files_sharing/api/v1/shares";

#[derive(Debug, Clone)]
pub struct SharesClient {
    client: NextcloudClient,
}

impl SharesClient {
    pub fn new(client: NextcloudClient) -> Self {
        Self { client }
    }

    pub async fn list(&self, options: &ShareListOptions) -> Result<Vec<Share>> {
        let path = build_shares_list_path(options)?;
        let raw: Vec<RawShare> = self
            .client
            .get_ocs_json::<OcsEnvelope<Vec<RawShare>>>(&path)
            .await?
            .into_data()?;
        Ok(raw.into_iter().map(Share::from).collect())
    }

    pub async fn create_public(&self, options: &ShareCreatePublicOptions) -> Result<Share> {
        let path = normalize_remote_path(&options.path)?;
        let mut form = vec![
            ("path".to_owned(), path),
            ("shareType".to_owned(), "3".to_owned()),
            ("permissions".to_owned(), options.permissions.to_string()),
        ];
        if let Some(password) = &options.password {
            form.push(("password".to_owned(), password.clone()));
        }
        if let Some(expire_date) = &options.expire_date {
            form.push(("expireDate".to_owned(), expire_date.clone()));
        }

        let raw: RawShare = self
            .client
            .post_ocs_form_json::<_, OcsEnvelope<RawShare>>(
                &format!("{SHARES_ENDPOINT}?format=json"),
                &form,
            )
            .await?
            .into_data()?;
        Ok(Share::from(raw))
    }

    pub async fn delete(&self, share_id: &str) -> Result<()> {
        let share_id = share_id.trim();
        if share_id.is_empty() {
            return Err(crate::Error::InvalidRemotePath {
                path: share_id.to_owned(),
                reason: "share id must not be empty".to_owned(),
            });
        }

        let encoded_id: String = form_urlencoded::byte_serialize(share_id.as_bytes()).collect();
        self.client
            .delete_ocs_json::<OcsEnvelope<Value>>(&format!(
                "{SHARES_ENDPOINT}/{encoded_id}?format=json"
            ))
            .await?
            .into_data()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShareListOptions {
    pub path: Option<String>,
    pub shared_with_me: bool,
    pub include_tags: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareCreatePublicOptions {
    pub path: String,
    pub password: Option<String>,
    pub expire_date: Option<String>,
    pub permissions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Share {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub share_type: String,
    pub share_type_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub permissions: i64,
    pub password_protected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub displayname_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mimetype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Value>,
}

impl From<RawShare> for Share {
    fn from(raw: RawShare) -> Self {
        let share_type_id = raw.i64("share_type").unwrap_or_default();
        let password_protected = raw
            .bool("password_protected")
            .unwrap_or_else(|| raw.string("password").is_some());

        Self {
            id: raw
                .string("id")
                .unwrap_or_else(|| raw.i64("id").unwrap_or_default().to_string()),
            path: raw.string("path").and_then(non_empty),
            share_type: share_type_name(share_type_id).to_owned(),
            share_type_id,
            share_with: raw.string("share_with").and_then(non_empty),
            url: raw.string("url").and_then(non_empty),
            token: raw.string("token").and_then(non_empty),
            permissions: raw.i64("permissions").unwrap_or_default(),
            password_protected,
            expiration: raw.string("expiration").and_then(non_empty),
            created_at: raw.i64("stime").and_then(timestamp_to_rfc3339),
            uid_owner: raw.string("uid_owner").and_then(non_empty),
            displayname_owner: raw.string("displayname_owner").and_then(non_empty),
            item_type: raw.string("item_type").and_then(non_empty),
            mimetype: raw.string("mimetype").and_then(non_empty),
            tags: raw.value("tags").cloned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct RawShare {
    #[serde(flatten)]
    fields: Map<String, Value>,
}

impl RawShare {
    fn value(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    fn string(&self, key: &str) -> Option<String> {
        match self.value(key)? {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }

    fn i64(&self, key: &str) -> Option<i64> {
        match self.value(key)? {
            Value::Number(value) => value.as_i64().or_else(|| value.as_u64().map(|n| n as i64)),
            Value::String(value) => value.parse().ok(),
            Value::Bool(value) => Some(i64::from(*value)),
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }

    fn bool(&self, key: &str) -> Option<bool> {
        match self.value(key)? {
            Value::Bool(value) => Some(*value),
            Value::Number(value) => value.as_i64().map(|number| number != 0),
            Value::String(value) => match value.as_str() {
                "true" | "1" => Some(true),
                "false" | "0" | "" => Some(false),
                _ => None,
            },
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }
}

fn build_shares_list_path(options: &ShareListOptions) -> Result<String> {
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    serializer.append_pair("format", "json");
    if options.shared_with_me {
        serializer.append_pair("shared_with_me", "true");
    }
    if options.include_tags {
        serializer.append_pair("include_tags", "true");
    }
    if let Some(path) = &options.path {
        serializer.append_pair("path", &normalize_remote_path(path)?);
    }

    Ok(format!("{SHARES_ENDPOINT}?{}", serializer.finish()))
}

fn share_type_name(value: i64) -> &'static str {
    match value {
        0 => "user",
        1 => "group",
        3 => "public_link",
        4 => "email",
        6 => "federated_cloud",
        7 => "circle",
        10 => "talk_room",
        _ => "unknown",
    }
}

fn timestamp_to_rfc3339(value: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp(value, 0)
        .map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;
    use url::Url;

    #[test]
    fn builds_list_path_with_filters() -> Result<()> {
        let path = build_shares_list_path(&ShareListOptions {
            path: Some("/Documents/report.md".to_owned()),
            shared_with_me: true,
            include_tags: true,
        })?;

        assert_eq!(
            path,
            "ocs/v2.php/apps/files_sharing/api/v1/shares?format=json&shared_with_me=true&include_tags=true&path=%2FDocuments%2Freport.md"
        );
        Ok(())
    }

    #[test]
    fn normalizes_public_link_share() -> Result<()> {
        let raw = r#"{
            "ocs": {
                "meta": {
                    "status": "ok",
                    "statuscode": 200,
                    "message": "OK"
                },
                "data": [{
                    "id": "123",
                    "path": "/Documents/report.pdf",
                    "share_type": 3,
                    "share_with": null,
                    "url": "https://cloud.example.com/s/abc123",
                    "token": "abc123",
                    "permissions": 1,
                    "password": null,
                    "expiration": null,
                    "stime": 1775833445,
                    "uid_owner": "nicholai",
                    "displayname_owner": "Nicholai",
                    "item_type": "file",
                    "mimetype": "application/pdf"
                }]
            }
        }"#;

        let shares: Vec<RawShare> = serde_json::from_str::<OcsEnvelope<Vec<RawShare>>>(raw)
            .expect("valid share response")
            .into_data()?;
        let share = Share::from(shares.into_iter().next().expect("one share"));

        assert_eq!(share.id, "123");
        assert_eq!(share.share_type, "public_link");
        assert_eq!(share.share_type_id, 3);
        assert_eq!(share.path.as_deref(), Some("/Documents/report.pdf"));
        assert_eq!(share.token.as_deref(), Some("abc123"));
        assert!(!share.password_protected);
        assert_eq!(share.created_at.as_deref(), Some("2026-04-10T15:04:05Z"));
        Ok(())
    }

    #[tokio::test]
    async fn list_sends_ocs_request_and_normalizes_response() -> Result<()> {
        let server = OneShotServer::spawn(
            r#"{
                "ocs": {
                    "meta": {
                        "status": "ok",
                        "statuscode": 200,
                        "message": "OK"
                    },
                    "data": [{
                        "id": "1",
                        "path": "/Documents",
                        "share_type": 0,
                        "share_with": "avery",
                        "permissions": "31",
                        "password": "",
                        "stime": "1775833445"
                    }]
                }
            }"#,
        );

        let shares = mock_shares(server.base_url())?
            .list(&ShareListOptions {
                path: Some("/Documents".to_owned()),
                shared_with_me: true,
                include_tags: false,
            })
            .await?;
        let request = server.join();

        assert_eq!(request.method, "GET");
        assert_eq!(
            request.path,
            "/ocs/v2.php/apps/files_sharing/api/v1/shares?format=json&shared_with_me=true&path=%2FDocuments"
        );
        assert_eq!(request.header("ocs-apirequest"), Some("true"));
        assert!(request.header("authorization").is_some());
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].share_type, "user");
        assert_eq!(shares[0].permissions, 31);
        Ok(())
    }

    #[tokio::test]
    async fn create_public_sends_ocs_form_and_normalizes_response() -> Result<()> {
        let server = OneShotServer::spawn(
            r#"{
                "ocs": {
                    "meta": {
                        "status": "ok",
                        "statuscode": 200,
                        "message": "OK"
                    },
                    "data": {
                        "id": "123",
                        "path": "/Documents/report.pdf",
                        "share_type": 3,
                        "url": "https://cloud.example.com/s/abc123",
                        "token": "abc123",
                        "permissions": 1,
                        "password": "secret",
                        "expiration": "2026-05-01",
                        "stime": 1775833445
                    }
                }
            }"#,
        );

        let share = mock_shares(server.base_url())?
            .create_public(&ShareCreatePublicOptions {
                path: "/Documents/report.pdf".to_owned(),
                password: Some("super-secret".to_owned()),
                expire_date: Some("2026-05-01".to_owned()),
                permissions: 1,
            })
            .await?;
        let request = server.join();

        assert_eq!(request.method, "POST");
        assert_eq!(
            request.path,
            "/ocs/v2.php/apps/files_sharing/api/v1/shares?format=json"
        );
        assert_eq!(request.header("ocs-apirequest"), Some("true"));
        assert_eq!(
            request.header("content-type"),
            Some("application/x-www-form-urlencoded")
        );
        assert!(request.header("authorization").is_some());
        assert!(request.body.contains("path=%2FDocuments%2Freport.pdf"));
        assert!(request.body.contains("shareType=3"));
        assert!(request.body.contains("permissions=1"));
        assert!(request.body.contains("password=super-secret"));
        assert!(request.body.contains("expireDate=2026-05-01"));
        assert_eq!(share.share_type, "public_link");
        assert!(share.password_protected);
        assert_eq!(share.expiration.as_deref(), Some("2026-05-01"));
        Ok(())
    }

    #[tokio::test]
    async fn delete_sends_ocs_delete_request() -> Result<()> {
        let server = OneShotServer::spawn(
            r#"{
                "ocs": {
                    "meta": {
                        "status": "ok",
                        "statuscode": 200,
                        "message": "OK"
                    },
                    "data": []
                }
            }"#,
        );

        mock_shares(server.base_url())?.delete("123/456").await?;
        let request = server.join();

        assert_eq!(request.method, "DELETE");
        assert_eq!(
            request.path,
            "/ocs/v2.php/apps/files_sharing/api/v1/shares/123%2F456?format=json"
        );
        assert_eq!(request.header("ocs-apirequest"), Some("true"));
        assert!(request.header("authorization").is_some());
        assert!(request.body.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn delete_maps_ocs_failure_envelope() -> Result<()> {
        let server = OneShotServer::spawn_with_status(
            404,
            r#"{
                "ocs": {
                    "meta": {
                        "status": "failure",
                        "statuscode": 404,
                        "message": "share not found"
                    },
                    "data": []
                }
            }"#,
        );

        let error = mock_shares(server.base_url())?
            .delete("404")
            .await
            .expect_err("missing share maps to OCS error");
        let request = server.join();

        assert_eq!(request.method, "DELETE");
        match error {
            crate::Error::OcsStatus {
                status,
                status_code,
                message,
            } => {
                assert_eq!(status, "failure");
                assert_eq!(status_code, 404);
                assert_eq!(message, "share not found");
            }
            other => panic!("unexpected error: {other:?}"),
        }
        Ok(())
    }

    fn mock_shares(base_url: &str) -> Result<SharesClient> {
        let base = Url::parse(base_url).expect("valid mock URL");
        let client = NextcloudClient::new(
            base,
            Some(crate::ClientAuth {
                username: "nicholai".to_owned(),
                app_password: "app-secret".to_owned(),
            }),
        )?;
        Ok(SharesClient::new(client))
    }

    #[derive(Debug)]
    struct RecordedRequest {
        method: String,
        path: String,
        headers: HashMap<String, String>,
        body: String,
    }

    impl RecordedRequest {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .get(&name.to_ascii_lowercase())
                .map(String::as_str)
        }
    }

    struct OneShotServer {
        base_url: String,
        handle: thread::JoinHandle<RecordedRequest>,
    }

    impl OneShotServer {
        fn spawn(body: &'static str) -> Self {
            Self::spawn_with_status(200, body)
        }

        fn spawn_with_status(status: u16, body: &'static str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
            let address = listener.local_addr().expect("read test server address");
            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept one request");
                let request = read_request(&mut stream);
                write_response(&mut stream, status, body);
                request
            });

            Self {
                base_url: format!("http://{address}/"),
                handle,
            }
        }

        fn base_url(&self) -> &str {
            &self.base_url
        }

        fn join(self) -> RecordedRequest {
            self.handle.join().expect("mock server thread")
        }
    }

    fn read_request(stream: &mut TcpStream) -> RecordedRequest {
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");

        let mut buffer = Vec::new();
        let mut header_end = None;
        let mut content_length = 0_usize;

        loop {
            let mut chunk = [0_u8; 1024];
            let read = stream.read(&mut chunk).expect("read request");
            assert!(read != 0, "connection closed before request completed");
            buffer.extend_from_slice(&chunk[..read]);

            if header_end.is_none()
                && let Some(position) = find_header_end(&buffer)
            {
                header_end = Some(position);
                let headers = String::from_utf8_lossy(&buffer[..position]);
                content_length = parse_content_length(&headers);
            }

            if let Some(position) = header_end
                && buffer.len() >= position + 4 + content_length
            {
                break;
            }
        }

        let header_end = header_end.expect("request headers");
        let request = String::from_utf8_lossy(&buffer[..header_end]);
        let mut lines = request.lines();
        let request_line = lines.next().expect("request line");
        let mut request_parts = request_line.split_whitespace();
        let method = request_parts.next().expect("method").to_owned();
        let path = request_parts.next().expect("path").to_owned();
        let mut headers = HashMap::new();
        for line in lines {
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
            }
        }

        let body_start = header_end + 4;
        let body_end = body_start + content_length;
        let body = String::from_utf8_lossy(&buffer[body_start..body_end]).into_owned();

        RecordedRequest {
            method,
            path,
            headers,
            body,
        }
    }

    fn write_response(stream: &mut TcpStream, status: u16, body: &str) {
        let reason = match status {
            200 => "OK",
            404 => "Not Found",
            403 => "Forbidden",
            401 => "Unauthorized",
            _ => "Unknown",
        };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    }

    fn find_header_end(buffer: &[u8]) -> Option<usize> {
        buffer.windows(4).position(|window| window == b"\r\n\r\n")
    }

    fn parse_content_length(headers: &str) -> usize {
        headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse().ok())
            .unwrap_or(0)
    }
}
