use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use percent_encoding::{AsciiSet, CONTROLS};
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::client::NextcloudClient;
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct CalendarClient {
    client: NextcloudClient,
    username: String,
}

impl CalendarClient {
    pub fn new(client: NextcloudClient, username: String) -> Self {
        Self { client, username }
    }

    pub async fn events(&self, options: &CalendarEventsOptions) -> Result<Vec<CalendarEvent>> {
        let calendars = if let Some(calendar) = &options.calendar {
            vec![CalendarRef {
                name: calendar.clone(),
                display_name: Some(calendar.clone()),
            }]
        } else {
            self.discover_calendars().await?
        };

        let mut events = Vec::new();
        for calendar in calendars {
            let body = build_calendar_query(options.from, options.to);
            let text = self
                .client
                .request_text(
                    Method::from_bytes(b"REPORT").expect("valid method"),
                    &self.calendar_path(&calendar.name),
                    Some(("1", &body)),
                )
                .await?;
            events.extend(parse_calendar_multistatus(
                &text,
                &calendar.name,
                calendar.display_name.as_deref(),
            )?);
        }
        events.sort_by(|left, right| left.starts_at.cmp(&right.starts_at));
        Ok(events)
    }

    pub async fn create_event(&self, options: &CalendarCreateOptions) -> Result<CalendarEvent> {
        let uid = options.uid.clone();
        let ical = build_ical_event(options);
        let path = self.event_path(&options.calendar, &uid);
        let etag = self
            .client
            .put_bytes(
                &path,
                ical.into_bytes(),
                Some("text/calendar; charset=utf-8"),
            )
            .await?;

        Ok(CalendarEvent {
            uid,
            calendar: options.calendar.clone(),
            calendar_display_name: None,
            summary: Some(options.summary.clone()),
            description: options.description.clone(),
            location: options.location.clone(),
            starts_at: Some(options.starts_at.clone()),
            ends_at: Some(options.ends_at.clone()),
            all_day: options.all_day,
            status: Some("CONFIRMED".to_owned()),
            href: Some(format!("/{}", path)),
            etag,
        })
    }

    pub async fn delete_event(&self, calendar: &str, uid: &str) -> Result<()> {
        let path = self.event_path(calendar, uid);
        self.client
            .request_text(Method::DELETE, &path, None)
            .await?;
        Ok(())
    }

    async fn discover_calendars(&self) -> Result<Vec<CalendarRef>> {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:displayname />
    <d:resourcetype />
  </d:prop>
</d:propfind>"#;
        let text = self
            .client
            .request_text(
                Method::from_bytes(b"PROPFIND").expect("valid method"),
                &self.calendar_home_path(),
                Some(("1", body)),
            )
            .await?;
        parse_calendar_refs(&text, &self.username)
    }

    fn calendar_home_path(&self) -> String {
        format!(
            "remote.php/dav/calendars/{}/",
            encode_segment(&self.username)
        )
    }

    fn calendar_path(&self, calendar: &str) -> String {
        format!(
            "remote.php/dav/calendars/{}/{}/",
            encode_segment(&self.username),
            encode_segment(calendar)
        )
    }

    fn event_path(&self, calendar: &str, uid: &str) -> String {
        format!(
            "remote.php/dav/calendars/{}/{}/{}.ics",
            encode_segment(&self.username),
            encode_segment(calendar),
            encode_segment(uid)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEventsOptions {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub calendar: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarCreateOptions {
    pub calendar: String,
    pub uid: String,
    pub summary: String,
    pub starts_at: String,
    pub ends_at: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub all_day: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalendarEvent {
    pub uid: String,
    pub calendar: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<String>,
    pub all_day: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CalendarRef {
    name: String,
    display_name: Option<String>,
}

#[derive(Debug, Default)]
struct CalendarRefBuilder {
    href: Option<String>,
    display_name: Option<String>,
    is_calendar: bool,
}

#[derive(Debug, Default)]
struct CalendarDataBuilder {
    href: Option<String>,
    etag: Option<String>,
    calendar_data: Option<String>,
}

fn parse_calendar_refs(xml: &str, username: &str) -> Result<Vec<CalendarRef>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut refs = Vec::new();
    let mut current: Option<CalendarRefBuilder> = None;
    let mut field: Option<CalendarRefField> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => current = Some(CalendarRefBuilder::default()),
                    "href" => field = Some(CalendarRefField::Href),
                    "displayname" => field = Some(CalendarRefField::DisplayName),
                    "calendar" => {
                        if let Some(current) = &mut current {
                            current.is_calendar = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(element)) => {
                if local_name(element.name().as_ref()) == "calendar"
                    && let Some(current) = &mut current
                {
                    current.is_calendar = true;
                }
            }
            Ok(Event::Text(text)) => {
                if let (Some(current), Some(field)) = (&mut current, field) {
                    let value = text
                        .decode()
                        .map(|cow| cow.into_owned())
                        .unwrap_or_default();
                    match field {
                        CalendarRefField::Href => current.href = Some(value),
                        CalendarRefField::DisplayName => current.display_name = Some(value),
                    }
                }
            }
            Ok(Event::End(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => {
                        if let Some(reference) = current
                            .take()
                            .and_then(|value| finish_calendar_ref(value, username))
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
                    context: "caldav calendar discovery".to_owned(),
                    message: source.to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(refs)
}

fn finish_calendar_ref(builder: CalendarRefBuilder, username: &str) -> Option<CalendarRef> {
    if !builder.is_calendar {
        return None;
    }
    let href = builder.href?;
    let marker = format!("/remote.php/dav/calendars/{}/", encode_segment(username));
    let calendar = href
        .split_once(&marker)
        .map(|(_, rest)| rest)
        .or_else(|| href.split_once("/calendars/").map(|(_, rest)| rest))?
        .trim_matches('/');
    if calendar.is_empty() || calendar.contains('/') {
        return None;
    }
    Some(CalendarRef {
        name: percent_decode(calendar),
        display_name: builder.display_name,
    })
}

fn parse_calendar_multistatus(
    xml: &str,
    calendar: &str,
    calendar_display_name: Option<&str>,
) -> Result<Vec<CalendarEvent>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut events = Vec::new();
    let mut current: Option<CalendarDataBuilder> = None;
    let mut field: Option<CalendarDataField> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => current = Some(CalendarDataBuilder::default()),
                    "href" => field = Some(CalendarDataField::Href),
                    "getetag" => field = Some(CalendarDataField::Etag),
                    "calendar-data" => field = Some(CalendarDataField::CalendarData),
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
                        CalendarDataField::Href => current.href = Some(value),
                        CalendarDataField::Etag => {
                            current.etag = Some(value.trim_matches('"').to_owned())
                        }
                        CalendarDataField::CalendarData => current.calendar_data = Some(value),
                    }
                }
            }
            Ok(Event::End(element)) => {
                let name = local_name(element.name().as_ref()).to_owned();
                match name.as_str() {
                    "response" => {
                        if let Some(current) = current.take()
                            && let Some(calendar_data) = current.calendar_data
                        {
                            events.extend(parse_ical_events(
                                &calendar_data,
                                calendar,
                                calendar_display_name,
                                current.href,
                                current.etag,
                            ));
                        }
                    }
                    "href" | "getetag" | "calendar-data" => field = None,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(source) => {
                return Err(Error::ParseXml {
                    context: "caldav calendar-query".to_owned(),
                    message: source.to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(events)
}

fn parse_ical_events(
    ical: &str,
    calendar: &str,
    calendar_display_name: Option<&str>,
    href: Option<String>,
    etag: Option<String>,
) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    let mut in_event = false;
    let mut fields = std::collections::BTreeMap::new();

    for line in unfold_ical_lines(ical) {
        if line == "BEGIN:VEVENT" {
            in_event = true;
            fields.clear();
            continue;
        }
        if line == "END:VEVENT" {
            if let Some(event) = finish_ical_event(
                &fields,
                calendar,
                calendar_display_name,
                href.clone(),
                etag.clone(),
            ) {
                events.push(event);
            }
            in_event = false;
            fields.clear();
            continue;
        }
        if in_event && let Some((key, value)) = line.split_once(':') {
            fields.insert(key.to_ascii_uppercase(), value.to_owned());
        }
    }

    events
}

fn finish_ical_event(
    fields: &std::collections::BTreeMap<String, String>,
    calendar: &str,
    calendar_display_name: Option<&str>,
    href: Option<String>,
    etag: Option<String>,
) -> Option<CalendarEvent> {
    let uid = field_value(fields, "UID")?;
    let dtstart_key = fields.keys().find(|key| key.starts_with("DTSTART"))?;
    let starts = parse_ical_datetime(dtstart_key, fields.get(dtstart_key)?)?;
    let ends = fields
        .keys()
        .find(|key| key.starts_with("DTEND"))
        .and_then(|key| parse_ical_datetime(key, fields.get(key)?));
    Some(CalendarEvent {
        uid: uid.to_owned(),
        calendar: calendar.to_owned(),
        calendar_display_name: calendar_display_name.map(str::to_owned),
        summary: field_value(fields, "SUMMARY").map(str::to_owned),
        description: field_value(fields, "DESCRIPTION").map(str::to_owned),
        location: field_value(fields, "LOCATION").map(str::to_owned),
        starts_at: Some(starts.value),
        ends_at: ends.map(|value| value.value),
        all_day: starts.all_day,
        status: field_value(fields, "STATUS").map(str::to_owned),
        href,
        etag,
    })
}

fn field_value<'a>(
    fields: &'a std::collections::BTreeMap<String, String>,
    name: &str,
) -> Option<&'a str> {
    fields
        .iter()
        .find(|(key, _)| key.split(';').next() == Some(name))
        .map(|(_, value)| value.as_str())
}

struct ParsedDateTime {
    value: String,
    all_day: bool,
}

fn parse_ical_datetime(key: &str, value: &str) -> Option<ParsedDateTime> {
    if key.contains("VALUE=DATE") || value.len() == 8 {
        let date = NaiveDate::parse_from_str(value, "%Y%m%d").ok()?;
        return Some(ParsedDateTime {
            value: date.to_string(),
            all_day: true,
        });
    }

    if let Some(utc_value) = value.strip_suffix('Z')
        && let Ok(naive) = NaiveDateTime::parse_from_str(utc_value, "%Y%m%dT%H%M%S")
    {
        return Some(ParsedDateTime {
            value: naive.and_utc().to_rfc3339(),
            all_day: false,
        });
    }

    let naive = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()?;
    Some(ParsedDateTime {
        value: naive.and_utc().to_rfc3339(),
        all_day: false,
    })
}

fn unfold_ical_lines(ical: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in ical.lines() {
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

fn build_calendar_query(from: DateTime<Utc>, to: DateTime<Utc>) -> String {
    let start = from.format("%Y%m%dT%H%M%SZ");
    let end = to.format("%Y%m%dT%H%M%SZ");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag />
    <c:calendar-data />
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT">
        <c:time-range start="{start}" end="{end}" />
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>"#
    )
}

pub fn build_ical_event(options: &CalendarCreateOptions) -> String {
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".to_owned(),
        "PRODID:-//nextcloud-cli//EN".to_owned(),
        "BEGIN:VEVENT".to_owned(),
        format!("UID:{}", escape_ical_text(&options.uid)),
        format!("SUMMARY:{}", escape_ical_text(&options.summary)),
        "STATUS:CONFIRMED".to_owned(),
    ];

    if options.all_day {
        lines.push(format!("DTSTART;VALUE=DATE:{}", options.starts_at));
        lines.push(format!("DTEND;VALUE=DATE:{}", options.ends_at));
    } else {
        lines.push(format!(
            "DTSTART:{}",
            rfc3339_to_ical_utc(&options.starts_at)
        ));
        lines.push(format!("DTEND:{}", rfc3339_to_ical_utc(&options.ends_at)));
    }

    if let Some(location) = &options.location {
        lines.push(format!("LOCATION:{}", escape_ical_text(location)));
    }
    if let Some(description) = &options.description {
        lines.push(format!("DESCRIPTION:{}", escape_ical_text(description)));
    }

    lines.push("END:VEVENT".to_owned());
    lines.push("END:VCALENDAR".to_owned());
    format!("{}\r\n", lines.join("\r\n"))
}

fn rfc3339_to_ical_utc(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
        .format("%Y%m%dT%H%M%SZ")
        .to_string()
}

fn escape_ical_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(';', "\\;")
        .replace(',', "\\,")
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

fn local_name(name: &[u8]) -> &str {
    let raw = std::str::from_utf8(name).unwrap_or_default();
    raw.rsplit(':').next().unwrap_or(raw)
}

#[derive(Debug, Clone, Copy)]
enum CalendarRefField {
    Href,
    DisplayName,
}

#[derive(Debug, Clone, Copy)]
enum CalendarDataField {
    Href,
    Etag,
    CalendarData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_calendar_refs() -> Result<()> {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav">
  <d:response>
    <d:href>/remote.php/dav/calendars/Nicholai%20Vogel/</d:href>
    <d:propstat><d:prop><d:resourcetype><d:collection /></d:resourcetype></d:prop></d:propstat>
  </d:response>
  <d:response>
    <d:href>/remote.php/dav/calendars/Nicholai%20Vogel/personal/</d:href>
    <d:propstat><d:prop>
      <d:displayname>Personal</d:displayname>
      <d:resourcetype><d:collection /><cal:calendar /></d:resourcetype>
    </d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;
        let refs = parse_calendar_refs(xml, "Nicholai Vogel")?;
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].name, "personal");
        assert_eq!(refs[0].display_name.as_deref(), Some("Personal"));
        Ok(())
    }

    #[test]
    fn parses_calendar_query_multistatus() -> Result<()> {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav">
  <d:response>
    <d:href>/remote.php/dav/calendars/nicholai/personal/event.ics</d:href>
    <d:propstat><d:prop>
      <d:getetag>"abc"</d:getetag>
      <cal:calendar-data>BEGIN:VCALENDAR
BEGIN:VEVENT
UID:event-1
SUMMARY:Meeting
DTSTART:20260410T160000Z
DTEND:20260410T170000Z
LOCATION:Office
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR</cal:calendar-data>
    </d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;
        let events = parse_calendar_multistatus(xml, "personal", Some("Personal"))?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].uid, "event-1");
        assert_eq!(events[0].summary.as_deref(), Some("Meeting"));
        assert_eq!(
            events[0].starts_at.as_deref(),
            Some("2026-04-10T16:00:00+00:00")
        );
        assert_eq!(events[0].etag.as_deref(), Some("abc"));
        Ok(())
    }

    #[test]
    fn serializes_ical_event() {
        let ical = build_ical_event(&CalendarCreateOptions {
            calendar: "personal".to_owned(),
            uid: "event-1".to_owned(),
            summary: "Meet, plan".to_owned(),
            starts_at: "2026-04-10T16:00:00Z".to_owned(),
            ends_at: "2026-04-10T17:00:00Z".to_owned(),
            location: Some("Office".to_owned()),
            description: Some("Bring notes".to_owned()),
            all_day: false,
        });

        assert!(ical.contains("BEGIN:VCALENDAR"));
        assert!(ical.contains("SUMMARY:Meet\\, plan"));
        assert!(ical.contains("DTSTART:20260410T160000Z"));
        assert!(ical.contains("DTEND:20260410T170000Z"));
        assert!(ical.contains("LOCATION:Office"));
    }
}
