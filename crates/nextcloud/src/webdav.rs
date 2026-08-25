use percent_encoding::{AsciiSet, CONTROLS};
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;

use crate::client::{DownloadedBytes, NextcloudClient};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct WebDavClient {
    client: NextcloudClient,
    username: String,
}

impl WebDavClient {
    pub fn new(client: NextcloudClient, username: String) -> Self {
        Self { client, username }
    }

    pub async fn list(&self, path: &str) -> Result<Vec<WebDavEntry>> {
        let text = self.propfind(path, "1").await?;
        let mut entries = parse_multistatus(&text)?;
        let base = normalize_remote_path(path)?;
        entries.retain(|entry| entry.path != base);
        Ok(entries)
    }

    pub async fn walk(&self, path: &str) -> Result<Vec<WebDavEntry>> {
        let root = normalize_remote_path(path)?;
        let mut pending = vec![root];
        let mut visited = HashSet::new();
        let mut files = Vec::new();

        while let Some(directory) = pending.pop() {
            if !visited.insert(directory.clone()) {
                continue;
            }
            for entry in self.list(&directory).await? {
                if entry.is_dir {
                    pending.push(entry.path);
                } else {
                    files.push(entry);
                }
            }
        }

        Ok(files)
    }

    pub async fn preview(&self, file_id: &str, width: u32, height: u32) -> Result<Vec<u8>> {
        self.client.preview(file_id, width, height).await
    }

    pub async fn stat(&self, path: &str) -> Result<Option<WebDavEntry>> {
        let text = self.propfind(path, "0").await?;
        let base = normalize_remote_path(path)?;
        Ok(parse_multistatus(&text)?
            .into_iter()
            .find(|entry| entry.path == base))
    }

    pub async fn search(&self, query: &str, scope: &str, limit: u32) -> Result<Vec<WebDavEntry>> {
        let scope = normalize_remote_path(scope)?;
        let body = build_search_request(&self.username, query, &scope, limit)?;
        let text = self
            .client
            .request_xml_text(
                Method::from_bytes(b"SEARCH").expect("valid method"),
                "remote.php/dav",
                body,
            )
            .await?;
        parse_multistatus(&text)
    }

    pub async fn mkdir(&self, path: &str) -> Result<()> {
        let dav_path = self.dav_path(path)?;
        self.client
            .request_text(
                Method::from_bytes(b"MKCOL").expect("valid method"),
                &dav_path,
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn mkdir_parents(&self, path: &str) -> Result<Vec<String>> {
        let normalized = normalize_remote_path(path)?;
        reject_root_path(&normalized)?;
        let mut created = Vec::new();
        let mut current = String::new();

        for segment in normalized.trim_start_matches('/').split('/') {
            current.push('/');
            current.push_str(segment);
            if let Some(entry) = self.stat_if_exists(&current).await? {
                if entry.is_dir {
                    continue;
                }
                return Err(Error::InvalidRemotePath {
                    path: current,
                    reason: "path segment exists and is not a folder".to_owned(),
                });
            }

            self.mkdir(&current).await?;
            created.push(current.clone());
        }

        Ok(created)
    }

    pub async fn upload(
        &self,
        remote_path: &str,
        bytes: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<Option<String>> {
        let dav_path = self.dav_path(remote_path)?;
        self.client.put_bytes(&dav_path, bytes, content_type).await
    }

    pub async fn download(&self, remote_path: &str) -> Result<Vec<u8>> {
        let dav_path = self.dav_path(remote_path)?;
        self.client.request_bytes(Method::GET, &dav_path).await
    }

    pub async fn download_to_writer<W>(
        &self,
        remote_path: &str,
        writer: &mut W,
    ) -> Result<DownloadedBytes>
    where
        W: Write,
    {
        let dav_path = self.dav_path(remote_path)?;
        self.client
            .request_to_writer(Method::GET, &dav_path, writer)
            .await
    }

    pub async fn delete(&self, remote_path: &str) -> Result<()> {
        let normalized = normalize_remote_path(remote_path)?;
        reject_root_path(&normalized)?;
        let dav_path = self.dav_path(&normalized)?;
        self.client
            .request_text(Method::DELETE, &dav_path, None)
            .await?;
        Ok(())
    }

    pub async fn exists(&self, remote_path: &str) -> Result<bool> {
        Ok(self.stat_if_exists(remote_path).await?.is_some())
    }

    async fn stat_if_exists(&self, path: &str) -> Result<Option<WebDavEntry>> {
        match self.stat(path).await {
            Ok(entry) => Ok(entry),
            Err(Error::HttpStatus { status, .. }) if status == StatusCode::NOT_FOUND => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn propfind(&self, path: &str, depth: &str) -> Result<String> {
        let body = r#"<?xml version="1.0"?>
<d:propfind xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
  <d:prop>
    <d:getlastmodified />
    <d:getcontentlength />
    <d:getcontenttype />
    <d:resourcetype />
    <d:getetag />
    <oc:fileid />
    <oc:permissions />
  </d:prop>
</d:propfind>"#;
        let dav_path = self.dav_path(path)?;
        self.client
            .request_text(
                Method::from_bytes(b"PROPFIND").expect("valid method"),
                &dav_path,
                Some((depth, body)),
            )
            .await
    }

    fn dav_path(&self, path: &str) -> Result<String> {
        let encoded_user = encode_segment(&self.username);
        let remote_path = encode_remote_path(path)?;
        if remote_path == "/" {
            Ok(format!("remote.php/dav/files/{encoded_user}/"))
        } else {
            Ok(format!("remote.php/dav/files/{encoded_user}{remote_path}"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebDavEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
}

#[derive(Debug, Default)]
struct EntryBuilder {
    href: Option<String>,
    is_dir: bool,
    content_type: Option<String>,
    size: Option<u64>,
    etag: Option<String>,
    file_id: Option<String>,
    permissions: Option<String>,
    modified_at: Option<String>,
}

pub fn parse_multistatus(xml: &str) -> Result<Vec<WebDavEntry>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut entries = Vec::new();
    let mut current: Option<EntryBuilder> = None;
    let mut current_field: Option<Field> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => current = Some(EntryBuilder::default()),
                    "href" => current_field = Some(Field::Href),
                    "getcontentlength" => current_field = Some(Field::Size),
                    "getcontenttype" => current_field = Some(Field::ContentType),
                    "getetag" => current_field = Some(Field::Etag),
                    "fileid" => current_field = Some(Field::FileId),
                    "permissions" => current_field = Some(Field::Permissions),
                    "getlastmodified" => current_field = Some(Field::ModifiedAt),
                    "collection" => {
                        if let Some(entry) = &mut current {
                            entry.is_dir = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(element)) => {
                if local_name(element.name().as_ref()) == "collection"
                    && let Some(entry) = &mut current
                {
                    entry.is_dir = true;
                }
            }
            Ok(Event::Text(text)) => {
                if let (Some(entry), Some(field)) = (&mut current, current_field) {
                    let value = text
                        .decode()
                        .map(|cow| cow.into_owned())
                        .unwrap_or_default();
                    apply_field(entry, field, value);
                }
            }
            Ok(Event::End(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => {
                        if let Some(entry) = current.take().and_then(finish_entry) {
                            entries.push(entry);
                        }
                    }
                    "href" | "getcontentlength" | "getcontenttype" | "getetag" | "fileid"
                    | "permissions" | "getlastmodified" => current_field = None,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                return Err(crate::Error::ParseXml {
                    context: "webdav multistatus".to_owned(),
                    message: source.to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(entries)
}

#[derive(Debug, Clone, Copy)]
enum Field {
    Href,
    Size,
    ContentType,
    Etag,
    FileId,
    Permissions,
    ModifiedAt,
}

fn apply_field(entry: &mut EntryBuilder, field: Field, value: String) {
    match field {
        Field::Href => entry.href = Some(value),
        Field::Size => entry.size = value.parse().ok(),
        Field::ContentType => entry.content_type = Some(value),
        Field::Etag => entry.etag = Some(value.trim_matches('"').to_owned()),
        Field::FileId => entry.file_id = Some(value),
        Field::Permissions => entry.permissions = Some(value),
        Field::ModifiedAt => entry.modified_at = Some(value),
    }
}

fn finish_entry(entry: EntryBuilder) -> Option<WebDavEntry> {
    let path = href_to_remote_path(&entry.href?)?;
    let name = if path == "/" {
        "/".to_owned()
    } else {
        path.trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_owned()
    };
    Some(WebDavEntry {
        path,
        name,
        is_dir: entry.is_dir,
        content_type: entry.content_type,
        size: entry.size,
        etag: entry.etag,
        file_id: entry.file_id,
        permissions: entry.permissions,
        modified_at: entry.modified_at,
    })
}

fn href_to_remote_path(href: &str) -> Option<String> {
    let after_marker = if let Some((_, after_marker)) = href.split_once("/remote.php/dav/files/") {
        after_marker
    } else if let Some((_, after_marker)) = href.split_once("/files/") {
        after_marker
    } else {
        return None;
    };
    let mut parts = after_marker.split('/');
    parts.next()?;
    let rest = parts.collect::<Vec<_>>().join("/");
    if rest.is_empty() {
        Some("/".to_owned())
    } else {
        normalize_remote_path(&percent_decode_path(&rest)).ok()
    }
}

pub fn normalize_remote_path(path: &str) -> Result<String> {
    if path.contains('\0') {
        return Err(Error::InvalidRemotePath {
            path: path.to_owned(),
            reason: "path contains a NUL byte".to_owned(),
        });
    }

    if path == "/" || path.is_empty() {
        return Ok("/".to_owned());
    }

    let mut segments = Vec::new();
    for segment in path.trim_matches('/').split('/') {
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." {
            return Err(Error::InvalidRemotePath {
                path: path.to_owned(),
                reason: "path segments `.` and `..` are not allowed".to_owned(),
            });
        }
        segments.push(segment);
    }

    if segments.is_empty() {
        Ok("/".to_owned())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

fn encode_remote_path(path: &str) -> Result<String> {
    let normalized = normalize_remote_path(path)?;
    if normalized == "/" {
        return Ok("/".to_owned());
    }
    let encoded = normalized
        .trim_start_matches('/')
        .split('/')
        .map(encode_segment)
        .collect::<Vec<_>>()
        .join("/");
    Ok(format!("/{encoded}"))
}

pub fn reject_root_path(path: &str) -> Result<()> {
    if normalize_remote_path(path)? == "/" {
        return Err(Error::InvalidRemotePath {
            path: path.to_owned(),
            reason: "root path is not valid for this operation".to_owned(),
        });
    }
    Ok(())
}

pub fn build_search_request(
    username: &str,
    query: &str,
    scope: &str,
    limit: u32,
) -> Result<String> {
    let normalized_scope = normalize_remote_path(scope)?;
    let limit = limit.clamp(1, 100);
    let encoded_user = encode_segment(username);
    let encoded_scope = encode_remote_path(&normalized_scope)?;
    let href = if encoded_scope == "/" {
        format!("/files/{encoded_user}/")
    } else {
        format!("/files/{encoded_user}{encoded_scope}")
    };
    let pattern = format!("%{}%", escape_like_literal(query));

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<d:searchrequest xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns" xmlns:ns="https://github.com/icewind1991/SearchDAV/ns">
  <d:basicsearch>
    <d:select>
      <d:prop>
        <d:getlastmodified />
        <d:getcontentlength />
        <d:getcontenttype />
        <d:resourcetype />
        <d:getetag />
        <oc:fileid />
        <oc:permissions />
      </d:prop>
    </d:select>
    <d:from>
      <d:scope>
        <d:href>{}</d:href>
        <d:depth>infinity</d:depth>
      </d:scope>
    </d:from>
    <d:where>
      <d:like>
        <d:prop>
          <d:displayname />
        </d:prop>
        <d:literal>{}</d:literal>
      </d:like>
    </d:where>
    <d:orderby>
      <d:order>
        <d:prop>
          <d:displayname />
        </d:prop>
        <d:ascending />
      </d:order>
    </d:orderby>
    <d:limit>
      <d:nresults>{}</d:nresults>
      <ns:firstresult>0</ns:firstresult>
    </d:limit>
  </d:basicsearch>
</d:searchrequest>"#,
        escape_xml_text(&href),
        escape_xml_text(&pattern),
        limit,
    ))
}

fn escape_like_literal(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn encode_segment(segment: &str) -> String {
    percent_encoding::utf8_percent_encode(segment, PATH_SEGMENT_ENCODE_SET).to_string()
}

const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

fn percent_decode_path(path: &str) -> String {
    percent_encoding::percent_decode_str(path)
        .decode_utf8_lossy()
        .into_owned()
}

fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn local_name(name: &[u8]) -> &str {
    let raw = std::str::from_utf8(name).unwrap_or_default();
    raw.rsplit(':').next().unwrap_or(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;
    use url::Url;

    #[test]
    fn parses_basic_multistatus() -> Result<()> {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
  <d:response>
    <d:href>/remote.php/dav/files/nicholai/Documents/report.md</d:href>
    <d:propstat><d:prop>
      <d:getcontentlength>42</d:getcontentlength>
      <d:getcontenttype>text/markdown</d:getcontenttype>
      <d:getetag>"abc"</d:getetag>
      <oc:fileid>99</oc:fileid>
      <oc:permissions>RGDNVW</oc:permissions>
    </d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;
        let entries = parse_multistatus(xml)?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/Documents/report.md");
        assert_eq!(entries[0].size, Some(42));
        assert_eq!(entries[0].etag.as_deref(), Some("abc"));
        Ok(())
    }

    #[test]
    fn normalizes_and_rejects_remote_paths() -> Result<()> {
        assert_eq!(normalize_remote_path("")?, "/");
        assert_eq!(normalize_remote_path("/")?, "/");
        assert_eq!(
            normalize_remote_path("Documents/report.md")?,
            "/Documents/report.md"
        );
        assert_eq!(
            normalize_remote_path("//Documents///Q1//")?,
            "/Documents/Q1"
        );
        assert!(normalize_remote_path("/Documents/../secret").is_err());
        assert!(normalize_remote_path("/Documents/./report.md").is_err());
        Ok(())
    }

    #[test]
    fn encodes_remote_path_by_segment() -> Result<()> {
        assert_eq!(
            encode_remote_path("/Documents/a report #1.md")?,
            "/Documents/a%20report%20%231.md"
        );
        Ok(())
    }

    #[test]
    fn directory_hrefs_have_names_and_normalized_paths() -> Result<()> {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
  <d:response>
    <d:href>/remote.php/dav/files/nicholai/Documents/</d:href>
    <d:propstat><d:prop>
      <d:resourcetype><d:collection /></d:resourcetype>
    </d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;

        let entries = parse_multistatus(xml)?;
        assert_eq!(entries[0].path, "/Documents");
        assert_eq!(entries[0].name, "Documents");
        assert!(entries[0].is_dir);
        Ok(())
    }

    #[test]
    fn parses_search_hrefs_from_dav_arbiter() -> Result<()> {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
  <d:response>
    <d:href>/files/nicholai/Documents/report%20Q1.md</d:href>
    <d:propstat><d:prop>
      <d:getcontentlength>42</d:getcontentlength>
      <d:getcontenttype>text/markdown</d:getcontenttype>
    </d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;

        let entries = parse_multistatus(xml)?;
        assert_eq!(entries[0].path, "/Documents/report Q1.md");
        assert_eq!(entries[0].name, "report Q1.md");
        Ok(())
    }

    #[test]
    fn builds_display_name_search_request() -> Result<()> {
        let body = build_search_request("Nicholai Vogel", "hello #1", "/", 25)?;
        assert!(body.contains("<d:href>/files/Nicholai%20Vogel/</d:href>"));
        assert!(body.contains("<d:like>"));
        assert!(body.contains("<d:displayname />"));
        assert!(body.contains("<d:literal>%hello #1%</d:literal>"));
        assert!(body.contains("<d:nresults>25</d:nresults>"));
        Ok(())
    }

    #[tokio::test]
    async fn list_sends_propfind_and_filters_base_entry() -> Result<()> {
        let server = OneShotServer::spawn(MockResponse::xml(
            207,
            r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
  <d:response>
    <d:href>/remote.php/dav/files/nicholai/Documents/</d:href>
    <d:propstat><d:prop><d:resourcetype><d:collection /></d:resourcetype></d:prop></d:propstat>
  </d:response>
  <d:response>
    <d:href>/remote.php/dav/files/nicholai/Documents/report.md</d:href>
    <d:propstat><d:prop>
      <d:getcontentlength>42</d:getcontentlength>
      <d:getcontenttype>text/markdown</d:getcontenttype>
      <d:getetag>"abc"</d:getetag>
    </d:prop></d:propstat>
  </d:response>
</d:multistatus>"#,
        ));

        let entries = mock_webdav(server.base_url())?.list("/Documents").await?;
        let request = server.join();

        assert_eq!(request.method, "PROPFIND");
        assert_eq!(request.path, "/remote.php/dav/files/nicholai/Documents");
        assert_eq!(request.header("depth"), Some("1"));
        assert!(request.header("authorization").is_some());
        assert!(request.body.contains("<d:propfind"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/Documents/report.md");
        assert_eq!(entries[0].size, Some(42));
        Ok(())
    }

    #[tokio::test]
    async fn stat_sends_depth_zero_propfind() -> Result<()> {
        let server = OneShotServer::spawn(MockResponse::xml(
            207,
            r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
  <d:response>
    <d:href>/remote.php/dav/files/nicholai/Documents/report.md</d:href>
    <d:propstat><d:prop>
      <d:getcontentlength>42</d:getcontentlength>
      <d:getcontenttype>text/markdown</d:getcontenttype>
    </d:prop></d:propstat>
  </d:response>
</d:multistatus>"#,
        ));

        let entry = mock_webdav(server.base_url())?
            .stat("/Documents/report.md")
            .await?;
        let request = server.join();

        assert_eq!(request.method, "PROPFIND");
        assert_eq!(
            request.path,
            "/remote.php/dav/files/nicholai/Documents/report.md"
        );
        assert_eq!(request.header("depth"), Some("0"));
        assert!(request.header("authorization").is_some());
        assert_eq!(
            entry.map(|entry| entry.path),
            Some("/Documents/report.md".to_owned())
        );
        Ok(())
    }

    #[tokio::test]
    async fn mkdir_maps_webdav_failure_status() -> Result<()> {
        let server = OneShotServer::spawn(MockResponse::text(409, "Conflict"));

        let error = mock_webdav(server.base_url())?
            .mkdir("/Documents/existing-parent/new")
            .await
            .expect_err("MKCOL should fail");
        let request = server.join();

        assert_eq!(request.method, "MKCOL");
        assert_eq!(
            request.path,
            "/remote.php/dav/files/nicholai/Documents/existing-parent/new"
        );
        assert!(request.header("authorization").is_some());
        match error {
            Error::HttpStatus { status, body } => {
                assert_eq!(status, StatusCode::CONFLICT);
                assert_eq!(body, "Conflict");
            }
            other => panic!("unexpected error: {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn search_sends_dav_search_xml() -> Result<()> {
        let server = OneShotServer::spawn(MockResponse::xml(
            207,
            r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
  <d:response>
    <d:href>/files/nicholai/Documents/report.md</d:href>
    <d:propstat><d:prop><d:getcontentlength>42</d:getcontentlength></d:prop></d:propstat>
  </d:response>
</d:multistatus>"#,
        ));

        let entries = mock_webdav(server.base_url())?
            .search("report", "/Documents", 10)
            .await?;
        let request = server.join();

        assert_eq!(request.method, "SEARCH");
        assert_eq!(request.path, "/remote.php/dav");
        assert_eq!(
            request.header("content-type"),
            Some("application/xml; charset=utf-8")
        );
        assert!(request.header("authorization").is_some());
        assert!(
            request
                .body
                .contains("<d:href>/files/nicholai/Documents</d:href>")
        );
        assert!(request.body.contains("<d:literal>%report%</d:literal>"));
        assert!(request.body.contains("<d:nresults>10</d:nresults>"));
        assert_eq!(entries[0].path, "/Documents/report.md");
        Ok(())
    }

    #[tokio::test]
    async fn upload_sends_put_body_and_returns_etag() -> Result<()> {
        let server = OneShotServer::spawn(
            MockResponse::text(201, "").with_header("etag", r#""etag-value""#),
        );

        let etag = mock_webdav(server.base_url())?
            .upload(
                "/Documents/report.md",
                b"hello".to_vec(),
                Some("text/markdown"),
            )
            .await?;
        let request = server.join();

        assert_eq!(request.method, "PUT");
        assert_eq!(
            request.path,
            "/remote.php/dav/files/nicholai/Documents/report.md"
        );
        assert_eq!(request.header("content-type"), Some("text/markdown"));
        assert!(request.header("authorization").is_some());
        assert_eq!(request.body, "hello");
        assert_eq!(etag.as_deref(), Some("etag-value"));
        Ok(())
    }

    #[tokio::test]
    async fn download_streams_response_to_writer() -> Result<()> {
        let server = OneShotServer::spawn(MockResponse::text(200, "hello"));

        let mut output = Vec::new();
        let downloaded = mock_webdav(server.base_url())?
            .download_to_writer("/Documents/report.md", &mut output)
            .await?;
        let request = server.join();

        assert_eq!(request.method, "GET");
        assert_eq!(
            request.path,
            "/remote.php/dav/files/nicholai/Documents/report.md"
        );
        assert!(request.header("authorization").is_some());
        assert_eq!(output, b"hello");
        assert_eq!(downloaded.bytes_written, 5);
        assert_eq!(downloaded.content_length, Some(5));
        Ok(())
    }

    fn mock_webdav(base_url: &str) -> Result<WebDavClient> {
        let base = Url::parse(base_url).expect("valid mock URL");
        let client = NextcloudClient::new(
            base,
            Some(crate::ClientAuth {
                username: "nicholai".to_owned(),
                app_password: "app-secret".to_owned(),
            }),
        )?;
        Ok(WebDavClient::new(client, "nicholai".to_owned()))
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

    struct MockResponse {
        status: u16,
        content_type: &'static str,
        headers: Vec<(&'static str, &'static str)>,
        body: &'static str,
    }

    impl MockResponse {
        fn text(status: u16, body: &'static str) -> Self {
            Self {
                status,
                content_type: "text/plain",
                headers: Vec::new(),
                body,
            }
        }

        fn xml(status: u16, body: &'static str) -> Self {
            Self {
                status,
                content_type: "application/xml; charset=utf-8",
                headers: Vec::new(),
                body,
            }
        }

        fn with_header(mut self, name: &'static str, value: &'static str) -> Self {
            self.headers.push((name, value));
            self
        }
    }

    struct OneShotServer {
        base_url: String,
        handle: thread::JoinHandle<RecordedRequest>,
    }

    impl OneShotServer {
        fn spawn(response: MockResponse) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
            let address = listener.local_addr().expect("read test server address");
            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept one request");
                let request = read_request(&mut stream);
                write_response(&mut stream, response);
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
        let headers_text = String::from_utf8_lossy(&buffer[..header_end]);
        let mut lines = headers_text.lines();
        let request_line = lines.next().expect("request line");
        let mut request_parts = request_line.split_whitespace();
        let method = request_parts.next().expect("method").to_owned();
        let path = request_parts.next().expect("path").to_owned();
        let mut headers = HashMap::new();
        for line in lines {
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

    fn write_response(stream: &mut TcpStream, response: MockResponse) {
        let mut headers = format!(
            "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            response.status,
            response.content_type,
            response.body.len()
        );
        for (name, value) in response.headers {
            headers.push_str(name);
            headers.push_str(": ");
            headers.push_str(value);
            headers.push_str("\r\n");
        }
        headers.push_str("\r\n");
        stream
            .write_all(headers.as_bytes())
            .expect("write response headers");
        stream
            .write_all(response.body.as_bytes())
            .expect("write response body");
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
