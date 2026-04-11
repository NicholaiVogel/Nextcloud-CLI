use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};

use crate::client::NextcloudClient;
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

    pub async fn stat(&self, path: &str) -> Result<Option<WebDavEntry>> {
        let text = self.propfind(path, "0").await?;
        let base = normalize_remote_path(path)?;
        Ok(parse_multistatus(&text)?
            .into_iter()
            .find(|entry| entry.path == base))
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
    let marker = "/remote.php/dav/files/";
    let after_marker = href.split(marker).nth(1)?;
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

fn encode_segment(segment: &str) -> String {
    percent_encoding::utf8_percent_encode(segment, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn percent_decode_path(path: &str) -> String {
    percent_encoding::percent_decode_str(path)
        .decode_utf8_lossy()
        .into_owned()
}

fn local_name(name: &[u8]) -> &str {
    let raw = std::str::from_utf8(name).unwrap_or_default();
    raw.rsplit(':').next().unwrap_or(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "/Documents/a%20report%20%231%2Emd"
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
}
