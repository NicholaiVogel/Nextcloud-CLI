use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::client::NextcloudClient;
use crate::error::Result;

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
        let base = normalize_remote_path(path);
        entries.retain(|entry| entry.path != base);
        Ok(entries)
    }

    pub async fn stat(&self, path: &str) -> Result<Option<WebDavEntry>> {
        let text = self.propfind(path, "0").await?;
        let base = normalize_remote_path(path);
        Ok(parse_multistatus(&text)?
            .into_iter()
            .find(|entry| entry.path == base))
    }

    pub async fn mkdir(&self, path: &str) -> Result<()> {
        let dav_path = self.dav_path(path);
        self.client
            .request_text(
                Method::from_bytes(b"MKCOL").expect("valid method"),
                &dav_path,
                None,
            )
            .await?;
        Ok(())
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
        let dav_path = self.dav_path(path);
        self.client
            .request_text(
                Method::from_bytes(b"PROPFIND").expect("valid method"),
                &dav_path,
                Some((depth, body)),
            )
            .await
    }

    fn dav_path(&self, path: &str) -> String {
        let encoded_user = encode_segment(&self.username);
        let remote_path = encode_remote_path(path);
        if remote_path == "/" {
            format!("remote.php/dav/files/{encoded_user}/")
        } else {
            format!("remote.php/dav/files/{encoded_user}{remote_path}")
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
        path.rsplit('/').next().unwrap_or_default().to_owned()
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
        Some(format!("/{}", percent_decode_path(&rest)))
    }
}

fn normalize_remote_path(path: &str) -> String {
    if path == "/" || path.is_empty() {
        "/".to_owned()
    } else {
        format!("/{}", path.trim_matches('/'))
    }
}

fn encode_remote_path(path: &str) -> String {
    let normalized = normalize_remote_path(path);
    if normalized == "/" {
        return "/".to_owned();
    }
    let encoded = normalized
        .trim_start_matches('/')
        .split('/')
        .map(encode_segment)
        .collect::<Vec<_>>()
        .join("/");
    format!("/{encoded}")
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
}
