//! EP-031 Suricata transport (AUD-030): real JSON-lines transport over
//! the DOCUMENTED Suricata EVE JSON surface.
//!
//! Suricata is the Enhanced profile network detection sensor
//! (SPEC-013 behavior 3; COMPONENT_REGISTRY GPL-2.0 external sensor).
//! Nexus consumes its documented eve.json output
//! (docs.suricata.io/en/latest/output/eve/eve-json-format.html) and
//! normalizes provider payloads at this infrastructure boundary -
//! free-form EVE JSON never becomes a domain contract.
//!
//! Canonical transport surface (documented eve.json common section +
//! Event type: alert):
//! - timestamp (RFC3339 with subsecond + offset), flow_id
//! - event_type (the discriminator; only "alert" records are parsed)
//! - src_ip / src_port / dest_ip / dest_port / proto
//! - alert.action / alert.gid / alert.signature_id / alert.rev
//! - alert.signature / alert.category / alert.severity (1..=4)
//!
//! The transport reads newline-delimited JSON records from an
//! arbitrary byte source (file, pipe, socket) and parses ONLY the
//! documented alert fields. Malformed or unknown records fail closed
//! (External / Vocabulary) and are never guessed.

use std::io::{BufRead, BufReader, Read};

use nexus_sentinel::{SentinelError, SentinelErrorCode};

/// A normalized Suricata EVE alert record (documented JSON fields).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SuricataAlert {
    /// Documented `event_type` discriminator (always "alert" here).
    pub event_type: String,
    /// RFC3339 observation timestamp (documented `timestamp`).
    pub observed_at: String,
    /// Documented `flow_id` (correlates EVE records for one flow).
    pub flow_id: Option<u64>,
    /// Source address (documented `src_ip`).
    pub src_ip: Option<String>,
    /// Source port (documented `src_port`).
    pub src_port: Option<u16>,
    /// Destination address (documented `dest_ip`).
    pub dest_ip: Option<String>,
    /// Destination port (documented `dest_port`).
    pub dest_port: Option<u16>,
    /// Transport protocol (documented `proto`).
    pub proto: Option<String>,
    /// Alert signature id (documented `alert.signature_id`).
    pub signature_id: Option<u64>,
    /// Alert signature text (documented `alert.signature`).
    pub signature: String,
    /// Alert category (documented `alert.category`).
    pub category: String,
    /// Alert severity, documented bound 1..=4 (1 highest).
    pub severity: Option<u8>,
}

/// The Suricata transport port. Default implementations fail closed
/// (Unavailable) so an unbound transport never fabricates alerts.
pub trait SuricataTransport {
    /// Read observed alerts from the provider.
    fn read_alerts(&mut self) -> Result<Vec<SuricataAlert>, SentinelError> {
        Err(SentinelError::unavailable(
            "suricata transport has no implementation bound",
        ))
    }
}

/// Unit transport: always fails closed (used for the unbound case and
/// as a default type parameter).
impl SuricataTransport for () {}

/// JSON-lines Suricata transport over an arbitrary byte source.
///
/// The provider surface is the documented Suricata EVE JSON record
/// shape. Records that are not `event_type: alert` are skipped ONLY
/// when they are provably non-alert records (other documented event
/// types); malformed alert-shaped records fail closed (External).
pub struct JsonLinesSuricataTransport<R> {
    source: R,
}

impl<R: Read> JsonLinesSuricataTransport<R> {
    pub fn new(source: R) -> Self {
        Self { source }
    }
}

impl<R: Read> SuricataTransport for JsonLinesSuricataTransport<R> {
    fn read_alerts(&mut self) -> Result<Vec<SuricataAlert>, SentinelError> {
        let mut reader = BufReader::new(&mut self.source);
        let mut alerts = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "suricata eve log read failed",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "malformed suricata eve json record",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            // The EVE surface is a multi-event JSON stream: records
            // are discriminated by `event_type`. Anything without the
            // discriminator is not an EVE record and is skipped as
            // control noise; any non-alert event type is observed but
            // not an alert (never fabricated into one).
            let Some(event_type) = value.get("event_type").and_then(|v| v.as_str()) else {
                continue;
            };
            if event_type != "alert" {
                continue;
            }
            let rec: SuricataAlertRaw = serde_json::from_value(value).map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "suricata alert record missing documented fields",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            alerts.push(rec.into());
        }
        Ok(alerts)
    }
}

/// Raw wire shape for the documented Suricata EVE alert record. The
/// nested provider keys (`alert.signature`, ...) are mapped to
/// normalized fields at this boundary.
#[derive(Debug, Clone, serde::Deserialize)]
struct SuricataAlertRaw {
    #[serde(default)]
    event_type: String,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    flow_id: Option<u64>,
    #[serde(default)]
    src_ip: Option<String>,
    #[serde(default)]
    src_port: Option<u16>,
    #[serde(default)]
    dest_ip: Option<String>,
    #[serde(default)]
    dest_port: Option<u16>,
    #[serde(default)]
    proto: Option<String>,
    /// Documented nested alert object. REQUIRED for an alert record:
    /// an `event_type: alert` record without the alert object cannot
    /// satisfy the documented shape and fails closed.
    alert: SuricataAlertObjectRaw,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SuricataAlertObjectRaw {
    #[serde(default)]
    signature_id: Option<u64>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    severity: Option<u8>,
}

impl From<SuricataAlertRaw> for SuricataAlert {
    fn from(raw: SuricataAlertRaw) -> Self {
        let observed_at = raw.timestamp.unwrap_or_default();
        Self {
            event_type: raw.event_type,
            observed_at,
            flow_id: raw.flow_id,
            src_ip: raw.src_ip,
            src_port: raw.src_port,
            dest_ip: raw.dest_ip,
            dest_port: raw.dest_port,
            proto: raw.proto,
            signature_id: raw.alert.signature_id,
            signature: raw.alert.signature.unwrap_or_default(),
            category: raw.alert.category.unwrap_or_default(),
            severity: raw.alert.severity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aud030_unit_suricata_transport_parses_documented_alert() {
        let json = r#"{"timestamp":"2026-08-20T00:00:01.123456+0000","flow_id":1676750115612680,"event_type":"alert","src_ip":"192.168.40.77","src_port":40000,"dest_ip":"192.168.40.1","dest_port":22,"proto":"TCP","alert":{"action":"allowed","gid":1,"signature_id":2018358,"rev":10,"signature":"ET SCAN Potential SSH Scan","category":"Attempted Information Leak","severity":2}}"#;
        let mut transport = JsonLinesSuricataTransport::new(json.as_bytes());
        let alerts = transport.read_alerts().unwrap();
        assert_eq!(alerts.len(), 1);
        let a = &alerts[0];
        assert_eq!(a.event_type, "alert");
        assert_eq!(a.src_ip.as_deref(), Some("192.168.40.77"));
        assert_eq!(a.dest_port, Some(22));
        assert_eq!(a.signature_id, Some(2018358));
        assert_eq!(a.signature, "ET SCAN Potential SSH Scan");
        assert_eq!(a.category, "Attempted Information Leak");
        assert_eq!(a.severity, Some(2));
        assert!(a.observed_at.starts_with("2026-08-20T00:00:01"));
    }

    #[test]
    fn aud030_unit_suricata_transport_skips_non_alert_events() {
        // DNS / flow / http records share the stream but are never
        // alerts; blank lines are skipped.
        let json = "{\"event_type\":\"dns\",\"dns\":{\"type\":\"query\"}}\n\n{\"event_type\":\"flow\",\"flow\":{\"pkts_toserver\":1}}\n{\"timestamp\":\"2026-08-20T00:00:02Z\",\"event_type\":\"alert\",\"alert\":{\"signature\":\"ET SCAN\",\"category\":\"Scan\",\"severity\":3}}\n";
        let mut transport = JsonLinesSuricataTransport::new(json.as_bytes());
        let alerts = transport.read_alerts().unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].signature, "ET SCAN");
    }

    #[test]
    fn aud030_unit_suricata_transport_fails_closed_on_malformed() {
        let json = "{\"event_type\":\"alert\",\"alert\":\nnot-json";
        let mut transport = JsonLinesSuricataTransport::new(json.as_bytes());
        let err = transport.read_alerts().unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
    }

    #[test]
    fn aud030_unit_suricata_transport_fails_closed_on_alert_missing_shape() {
        // A record with event_type=alert but no alert object cannot
        // satisfy the documented alert shape; it fails closed.
        let json = "{\"event_type\":\"alert\"}";
        let mut transport = JsonLinesSuricataTransport::new(json.as_bytes());
        let err = transport.read_alerts().unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
    }
}
