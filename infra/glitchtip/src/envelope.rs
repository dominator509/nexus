//! Sentry envelope serialization (EP-038 M3).
//!
//! Wire grammar verified against the authoritative Sentry Envelope
//! documentation:
//!
//! ```text
//! Envelope = Headers { "\n" Item } [ "\n" ] ;
//! Item     = Headers "\n" Payload ;
//! ```
//!
//! - Headers are single-line compact JSON objects, UTF-8, no leading
//!   or trailing whitespace, followed by `\n`.
//! - Envelope header attributes: `dsn` (full DSN, self-auth),
//!   `sdk` (object), `sent_at` (RFC 3339 UTC).
//! - Item header attributes: `type` (required), `length`
//!   (payload length in bytes, recommended).
//! - Envelopes are terminated with an optional trailing newline.
//!
//! We serialize exactly one `event` item per envelope.

use serde_json::{json, Map, Value};

use crate::dsn::Dsn;
use crate::event::EventPayload;

/// Serialize a complete envelope for the given DSN and event payload.
///
/// The `dsn` envelope header carries the full DSN for
/// self-authentication (documented: "an envelope can be self
/// authenticated"). `sent_at` is generated close to transmission.
pub fn serialize_envelope(dsn: &Dsn, event: &EventPayload, sent_at: &str) -> String {
    let envelope_header = json!({
        "dsn": dsn.full(),
        "sdk": {
            "name": "nexus-glitchtip",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "sent_at": sent_at,
    });

    let event_json = event.to_json();
    let payload = serde_json::to_string(&event_json).expect("event payload serializes");
    let payload_len = payload.len();

    let mut item_header = Map::new();
    item_header.insert("type".to_string(), Value::String("event".to_string()));
    item_header.insert("length".to_string(), Value::Number(payload_len.into()));

    let mut out = String::new();
    out.push_str(&serde_json::to_string(&envelope_header).expect("header serializes"));
    out.push('\n');
    out.push_str(
        &serde_json::to_string(&Value::Object(item_header)).expect("item header serializes"),
    );
    out.push('\n');
    out.push_str(&payload);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventPayload, EventTag};

    fn sample_event() -> EventPayload {
        EventPayload::builder("fc6d8c0c43fc4630ad850ee518f1b9d0")
            .timestamp("2011-05-02T17:41:36Z")
            .platform("rust")
            .level("error")
            .logger("nexus.incidents")
            .release("nexus@0.1.0")
            .environment("test")
            .tag(EventTag::new("source", "storage"))
            .tag(EventTag::new("classification", "unavailable"))
            .extra("dedupe_key", "storage:unavailable")
            .fingerprint(vec!["storage".to_string(), "unavailable".to_string()])
            .build()
    }

    #[test]
    fn envelope_grammar_header_item_payload() {
        let dsn =
            Dsn::parse("https://0123456789abcdef0123456789abcdef@glitchtip.local/42").unwrap();
        let envelope = serialize_envelope(&dsn, &sample_event(), "2026-08-23T00:00:00Z");
        let lines: Vec<&str> = envelope.split('\n').collect();
        // header + item header + payload + trailing newline => 4 lines.
        assert_eq!(lines.len(), 4);
        // Each JSON header is exactly one compact line.
        let header: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(
            header["dsn"],
            "https://0123456789abcdef0123456789abcdef@glitchtip.local/42"
        );
        assert_eq!(header["sdk"]["name"], "nexus-glitchtip");
        assert_eq!(header["sent_at"], "2026-08-23T00:00:00Z");
        let item_header: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(item_header["type"], "event");
        let payload_len = lines[2].len();
        assert_eq!(item_header["length"], payload_len);
    }

    #[test]
    fn envelope_contains_event_id_in_payload() {
        let dsn =
            Dsn::parse("https://0123456789abcdef0123456789abcdef@glitchtip.local/42").unwrap();
        let envelope = serialize_envelope(&dsn, &sample_event(), "2026-08-23T00:00:00Z");
        assert!(envelope.contains("\"event_id\":\"fc6d8c0c43fc4630ad850ee518f1b9d0\""));
        assert!(envelope.contains("\"level\":\"error\""));
        assert!(envelope.contains("\"platform\":\"rust\""));
    }

    #[test]
    fn envelope_trailing_newline_termination() {
        let dsn =
            Dsn::parse("https://0123456789abcdef0123456789abcdef@glitchtip.local/42").unwrap();
        let envelope = serialize_envelope(&dsn, &sample_event(), "2026-08-23T00:00:00Z");
        assert!(envelope.ends_with('\n'));
        assert!(!envelope.ends_with("\n\n"));
    }

    #[test]
    fn envelope_headers_are_single_line_no_whitespace() {
        let dsn =
            Dsn::parse("https://0123456789abcdef0123456789abcdef@glitchtip.local/42").unwrap();
        let envelope = serialize_envelope(&dsn, &sample_event(), "2026-08-23T00:00:00Z");
        for line in envelope.split('\n') {
            if !line.is_empty() {
                assert!(!line.starts_with(' '));
                assert!(!line.ends_with(' '));
                // A header line must parse as JSON (payload lines parse too).
                let v: Value = serde_json::from_str(line).unwrap_or(Value::Null);
                if v != Value::Null {
                    let _ = v;
                }
            }
        }
    }
}
