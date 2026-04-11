use percent_encoding::{AsciiSet, CONTROLS};
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::client::NextcloudClient;
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct ContactsClient {
    client: NextcloudClient,
    username: String,
}

impl ContactsClient {
    pub fn new(client: NextcloudClient, username: String) -> Self {
        Self { client, username }
    }

    pub async fn search(&self, options: &ContactSearchOptions) -> Result<Vec<Contact>> {
        let addressbooks = if let Some(addressbook) = &options.addressbook {
            vec![AddressBookRef {
                name: addressbook.clone(),
                display_name: Some(addressbook.clone()),
            }]
        } else {
            self.discover_addressbooks().await?
        };

        let mut contacts = Vec::new();
        for addressbook in addressbooks {
            let body = build_addressbook_query(&options.query);
            let text = self
                .client
                .request_text(
                    Method::from_bytes(b"REPORT").expect("valid method"),
                    &self.addressbook_path(&addressbook.name),
                    Some(("1", &body)),
                )
                .await?;
            contacts.extend(parse_addressbook_multistatus(
                &text,
                &addressbook.name,
                addressbook.display_name.as_deref(),
            )?);
            if contacts.len() >= options.limit as usize {
                contacts.truncate(options.limit as usize);
                break;
            }
        }

        Ok(contacts)
    }

    async fn discover_addressbooks(&self) -> Result<Vec<AddressBookRef>> {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:displayname />
    <d:resourcetype />
  </d:prop>
</d:propfind>"#;
        let text = self
            .client
            .request_text(
                Method::from_bytes(b"PROPFIND").expect("valid method"),
                &self.addressbook_home_path(),
                Some(("1", body)),
            )
            .await?;
        parse_addressbook_refs(&text, &self.username)
    }

    fn addressbook_home_path(&self) -> String {
        format!(
            "remote.php/dav/addressbooks/users/{}/",
            encode_segment(&self.username)
        )
    }

    fn addressbook_path(&self, addressbook: &str) -> String {
        format!(
            "remote.php/dav/addressbooks/users/{}/{}/",
            encode_segment(&self.username),
            encode_segment(addressbook)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactSearchOptions {
    pub query: String,
    pub limit: u32,
    pub addressbook: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contact {
    pub uid: String,
    pub addressbook: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addressbook_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    pub emails: Vec<String>,
    pub phones: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AddressBookRef {
    name: String,
    display_name: Option<String>,
}

#[derive(Debug, Default)]
struct AddressBookRefBuilder {
    href: Option<String>,
    display_name: Option<String>,
    is_addressbook: bool,
}

#[derive(Debug, Default)]
struct ContactDataBuilder {
    href: Option<String>,
    etag: Option<String>,
    address_data: Option<String>,
}

fn parse_addressbook_refs(xml: &str, username: &str) -> Result<Vec<AddressBookRef>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut refs = Vec::new();
    let mut current: Option<AddressBookRefBuilder> = None;
    let mut field: Option<AddressBookRefField> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => current = Some(AddressBookRefBuilder::default()),
                    "href" => field = Some(AddressBookRefField::Href),
                    "displayname" => field = Some(AddressBookRefField::DisplayName),
                    "addressbook" => {
                        if let Some(current) = &mut current {
                            current.is_addressbook = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(element)) => {
                if local_name(element.name().as_ref()) == "addressbook"
                    && let Some(current) = &mut current
                {
                    current.is_addressbook = true;
                }
            }
            Ok(Event::Text(text)) => {
                if let (Some(current), Some(field)) = (&mut current, field) {
                    let value = text
                        .decode()
                        .map(|cow| cow.into_owned())
                        .unwrap_or_default();
                    match field {
                        AddressBookRefField::Href => current.href = Some(value),
                        AddressBookRefField::DisplayName => current.display_name = Some(value),
                    }
                }
            }
            Ok(Event::End(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => {
                        if let Some(reference) = current
                            .take()
                            .and_then(|value| finish_addressbook_ref(value, username))
                        {
                            refs.push(reference);
                        }
                    }
                    "href" | "displayname" => field = None,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                return Err(Error::ParseXml {
                    context: "carddav addressbook discovery".to_owned(),
                    message: source.to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(refs)
}

fn finish_addressbook_ref(
    builder: AddressBookRefBuilder,
    username: &str,
) -> Option<AddressBookRef> {
    if !builder.is_addressbook {
        return None;
    }
    let href = builder.href?;
    let marker = format!(
        "/remote.php/dav/addressbooks/users/{}/",
        encode_segment(username)
    );
    let addressbook = href
        .split_once(&marker)
        .map(|(_, rest)| rest)
        .or_else(|| {
            href.split_once("/addressbooks/users/")
                .map(|(_, rest)| rest)
        })?
        .trim_matches('/');
    if addressbook.is_empty() || addressbook.contains('/') {
        return None;
    }
    Some(AddressBookRef {
        name: percent_decode(addressbook),
        display_name: builder.display_name,
    })
}

fn parse_addressbook_multistatus(
    xml: &str,
    addressbook: &str,
    addressbook_display_name: Option<&str>,
) -> Result<Vec<Contact>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut contacts = Vec::new();
    let mut current: Option<ContactDataBuilder> = None;
    let mut field: Option<ContactDataField> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => current = Some(ContactDataBuilder::default()),
                    "href" => field = Some(ContactDataField::Href),
                    "getetag" => field = Some(ContactDataField::Etag),
                    "address-data" => field = Some(ContactDataField::AddressData),
                    _ => {}
                }
            }
            Ok(Event::Text(text)) => {
                if let (Some(current), Some(field)) = (&mut current, field) {
                    let value = text
                        .decode()
                        .map(|cow| cow.into_owned())
                        .unwrap_or_default();
                    match field {
                        ContactDataField::Href => current.href = Some(value),
                        ContactDataField::Etag => {
                            current.etag = Some(value.trim_matches('"').to_owned());
                        }
                        ContactDataField::AddressData => current.address_data = Some(value),
                    }
                }
            }
            Ok(Event::End(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => {
                        if let Some(current) = current.take()
                            && let Some(address_data) = current.address_data
                            && let Some(contact) = parse_vcard_contact(
                                &address_data,
                                addressbook,
                                addressbook_display_name,
                                current.href,
                                current.etag,
                            )
                        {
                            contacts.push(contact);
                        }
                    }
                    "href" | "getetag" | "address-data" => field = None,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                return Err(Error::ParseXml {
                    context: "carddav addressbook-query".to_owned(),
                    message: source.to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(contacts)
}

fn parse_vcard_contact(
    vcard: &str,
    addressbook: &str,
    addressbook_display_name: Option<&str>,
    href: Option<String>,
    etag: Option<String>,
) -> Option<Contact> {
    let mut uid = None;
    let mut full_name = None;
    let mut emails = Vec::new();
    let mut phones = Vec::new();
    let mut organization = None;
    let mut title = None;

    for line in unfold_lines(vcard) {
        let Some((raw_key, raw_value)) = line.split_once(':') else {
            continue;
        };
        let key = raw_key
            .split(';')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        let value = unescape_vcard_value(raw_value);
        match key.as_str() {
            "UID" => uid = non_empty(value),
            "FN" => full_name = non_empty(value),
            "EMAIL" => {
                if let Some(value) = non_empty(value) {
                    emails.push(value);
                }
            }
            "TEL" => {
                if let Some(value) = non_empty(value) {
                    phones.push(value);
                }
            }
            "ORG" => organization = non_empty(value.replace(';', " ")),
            "TITLE" => title = non_empty(value),
            _ => {}
        }
    }

    Some(Contact {
        uid: uid.or_else(|| href.as_deref().and_then(uid_from_href))?,
        addressbook: addressbook.to_owned(),
        addressbook_display_name: addressbook_display_name.map(str::to_owned),
        full_name,
        emails,
        phones,
        organization,
        title,
        href,
        etag,
    })
}

fn build_addressbook_query(query: &str) -> String {
    let query = escape_xml_text(query);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<card:addressbook-query xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:getetag />
    <card:address-data />
  </d:prop>
  <card:filter test="anyof">
    <card:prop-filter name="FN">
      <card:text-match collation="i;unicode-casemap" match-type="contains">{query}</card:text-match>
    </card:prop-filter>
    <card:prop-filter name="EMAIL">
      <card:text-match collation="i;unicode-casemap" match-type="contains">{query}</card:text-match>
    </card:prop-filter>
    <card:prop-filter name="N">
      <card:text-match collation="i;unicode-casemap" match-type="contains">{query}</card:text-match>
    </card:prop-filter>
  </card:filter>
</card:addressbook-query>"#
    )
}

fn uid_from_href(href: &str) -> Option<String> {
    href.trim_end_matches('/')
        .rsplit('/')
        .next()
        .map(|value| value.trim_end_matches(".vcf").to_owned())
        .and_then(non_empty)
}

fn unfold_lines(value: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in value.lines() {
        let line = raw.trim_end_matches('\r');
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(previous) = lines.last_mut() {
                previous.push_str(line.trim_start());
            }
        } else {
            lines.push(line.to_owned());
        }
    }
    lines
}

fn unescape_vcard_value(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
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

fn percent_decode(path: &str) -> String {
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

#[derive(Debug, Clone, Copy)]
enum AddressBookRefField {
    Href,
    DisplayName,
}

#[derive(Debug, Clone, Copy)]
enum ContactDataField {
    Href,
    Etag,
    AddressData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_addressbook_refs() -> Result<()> {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:response>
    <d:href>/remote.php/dav/addressbooks/users/Nicholai%20Vogel/contacts/</d:href>
    <d:propstat><d:prop>
      <d:displayname>Contacts</d:displayname>
      <d:resourcetype><d:collection /><card:addressbook /></d:resourcetype>
    </d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;
        let refs = parse_addressbook_refs(xml, "Nicholai Vogel")?;
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].name, "contacts");
        assert_eq!(refs[0].display_name.as_deref(), Some("Contacts"));
        Ok(())
    }

    #[test]
    fn parses_addressbook_query_multistatus() -> Result<()> {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:response>
    <d:href>/remote.php/dav/addressbooks/users/nicholai/contacts/ada.vcf</d:href>
    <d:propstat><d:prop>
      <d:getetag>"abc"</d:getetag>
      <card:address-data>BEGIN:VCARD
VERSION:4.0
UID:ada
FN:Ada Lovelace
EMAIL;TYPE=work:ada@example.com
TEL;TYPE=cell:+15555550100
ORG:Analytical Engine;Research
TITLE:Countess
END:VCARD</card:address-data>
    </d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;
        let contacts = parse_addressbook_multistatus(xml, "contacts", Some("Contacts"))?;
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].uid, "ada");
        assert_eq!(contacts[0].full_name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(contacts[0].emails, vec!["ada@example.com"]);
        assert_eq!(contacts[0].phones, vec!["+15555550100"]);
        assert_eq!(
            contacts[0].organization.as_deref(),
            Some("Analytical Engine Research")
        );
        assert_eq!(contacts[0].etag.as_deref(), Some("abc"));
        Ok(())
    }
}
