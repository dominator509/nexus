//! EP-031 Zeek transport (M2): real JSON-lines transport over the
//! DOCUMENTED Zeek JSON log surface.
//!
//! Zeek is the Advanced profile network detection sensor (SPEC-013
//! behavior 3; COMPONENT_REGISTRY GPL-2.0 external sensor). Nexus
//! consumes its documented JSON log output (Zeek JSON Streaming Logs,
//! docs.zeek.org/en/current/log-formats.html) and normalizes provider
//! payloads at this infrastructure boundary - free-form Zeek JSON
//! never becomes a domain contract.
//!
//! Canonical transport surface (documented notice.log JSON fields):
//! - ts (epoch seconds), uid (connection UID)
//! - id.orig_h / id.orig_p / id.resp_h / id.resp_p
//! - proto, note (e.g. Scan::Port_Scan), msg, sub
//! - src / dst / p / n (notice parameters)
//! - peer_descr, actions, suppress_for, dropped
//!
//! The transport reads newline-delimited JSON records from an
//! arbitrary byte source (file, pipe, socket) and parses ONLY the
//! documented notice fields. Malformed or unknown records fail closed
//! (External / Vocabulary) and are never guessed.

use std::io::{BufRead, BufReader, Read};

use nexus_sentinel::{SentinelError, SentinelErrorCode};

/// A normalized Zeek notice.log record (documented JSON fields).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ZeekNotice {
    /// Connection UID (documented `uid`).
    pub uid: String,
    /// Originating host (documented `id.orig_h`).
    pub orig_h: Option<String>,
    /// Originating port (documented `id.orig_p`).
    pub orig_p: Option<u16>,
    /// Responding host (documented `id.resp_h`).
    pub resp_h: Option<String>,
    /// Responding port (documented `id.resp_p`).
    pub resp_p: Option<u16>,
    /// Transport protocol (documented `proto`).
    pub proto: Option<String>,
    /// Notice identifier (documented `note`, e.g. `Scan::Port_Scan`).
    pub note: String,
    /// Human message (documented `msg`).
    pub msg: Option<String>,
    /// Sub-message (documented `sub`).
    pub sub: Option<String>,
    /// Source address (documented `src`).
    pub src: Option<String>,
    /// Destination address (documented `dst`).
    pub dst: Option<String>,
    /// Port parameter (documented `p`).
    pub p: Option<u16>,
    /// Count parameter (documented `n`).
    pub n: Option<u64>,
    /// Notice actions (documented `actions`).
    pub actions: Vec<String>,
    /// Whether the notice was dropped (documented `dropped`).
    pub dropped: Option<bool>,
    /// RFC3339 observation timestamp derived from the documented
    /// epoch `ts` field.
    pub observed_at: String,
}

/// The Zeek transport port. Default implementations fail closed
/// (Unavailable) so an unbound transport never fabricates notices.
pub trait ZeekTransport {
    /// Read observed notices from the provider.
    fn read_notices(&mut self) -> Result<Vec<ZeekNotice>, SentinelError> {
        Err(SentinelError::unavailable(
            "zeek transport has no implementation bound",
        ))
    }
}

/// Unit transport: always fails closed (used for the unbound case and
/// as a default type parameter).
impl ZeekTransport for () {}

/// JSON-lines Zeek transport over an arbitrary byte source.
///
/// The provider surface is the documented Zeek JSON log record shape.
/// Records that are not notice records or that fail to parse are
/// skipped ONLY when they are provably non-notice control records;
/// malformed notice-shaped records fail closed (External).
pub struct JsonLinesZeekTransport<R> {
    source: R,
}

impl<R: Read> JsonLinesZeekTransport<R> {
    pub fn new(source: R) -> Self {
        Self { source }
    }
}

impl<R: Read> ZeekTransport for JsonLinesZeekTransport<R> {
    fn read_notices(&mut self) -> Result<Vec<ZeekNotice>, SentinelError> {
        // We read through a fresh BufReader per call; the source is
        // consumed once per call (log tail semantics: caller re-opens
        // the source for the next window).
        let mut reader = BufReader::new(&mut self.source);
        let mut notices = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "zeek log read failed",
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
                    "malformed zeek json record",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            // The Zeek JSON log format emits one leading header record
            // per stream ({"_path": ..., "_write_ts": ...}); it is not
            // a notice and carries no `note`. Anything with a `note`
            // field is parsed as a notice; anything else is skipped as
            // a control/header record.
            if value.get("note").is_none() {
                continue;
            }
            let rec: ZeekNoticeRaw = serde_json::from_value(value).map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "zeek notice record missing documented fields",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            notices.push(rec.into());
        }
        Ok(notices)
    }
}

/// Raw wire shape for the documented Zeek notice JSON record. The
/// dotted provider keys (`id.orig_h`, ...) are mapped to normalized
/// fields at this boundary.
#[derive(Debug, Clone, serde::Deserialize)]
struct ZeekNoticeRaw {
    #[serde(default)]
    uid: String,
    #[serde(rename = "id.orig_h", default)]
    orig_h: Option<String>,
    #[serde(rename = "id.orig_p", default)]
    orig_p: Option<u16>,
    #[serde(rename = "id.resp_h", default)]
    resp_h: Option<String>,
    #[serde(rename = "id.resp_p", default)]
    resp_p: Option<u16>,
    #[serde(default)]
    proto: Option<String>,
    note: String,
    #[serde(default)]
    msg: Option<String>,
    #[serde(default)]
    sub: Option<String>,
    #[serde(default)]
    src: Option<String>,
    #[serde(default)]
    dst: Option<String>,
    #[serde(default)]
    p: Option<u16>,
    #[serde(default)]
    n: Option<u64>,
    #[serde(default)]
    actions: Vec<String>,
    #[serde(default)]
    dropped: Option<bool>,
    #[serde(default)]
    ts: Option<f64>,
}

impl From<ZeekNoticeRaw> for ZeekNotice {
    fn from(raw: ZeekNoticeRaw) -> Self {
        // The documented `ts` field is epoch seconds. We keep it in
        // the normalized record as a stable RFC3339 rendering; when
        // the field is absent (malformed notice) the record still
        // carries the note evidence but a zero/absent timestamp is
        // preserved as a missing observation time (never invented).
        let observed_at = raw
            .ts
            .map(|t| {
                let secs = t.trunc() as i64;
                let frac = t.fract();
                let millis = (frac.abs() * 1000.0).round() as u32;
                format!("{}.{:03}Z", chrono_like_seconds_to_rfc3339(secs), millis)
            })
            .unwrap_or_default();
        Self {
            uid: raw.uid,
            orig_h: raw.orig_h,
            orig_p: raw.orig_p,
            resp_h: raw.resp_h,
            resp_p: raw.resp_p,
            proto: raw.proto,
            note: raw.note,
            msg: raw.msg,
            sub: raw.sub,
            src: raw.src,
            dst: raw.dst,
            p: raw.p,
            n: raw.n,
            actions: raw.actions,
            dropped: raw.dropped,
            observed_at,
        }
    }
}

/// Minimal RFC3339 UTC rendering without pulling a date crate: format
/// the epoch seconds as `YYYY-MM-DDTHH:MM:SS` in UTC.
fn chrono_like_seconds_to_rfc3339(secs: i64) -> String {
    // Days-from-epoch civil calendar (Howard Hinnant's algorithm).
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    // AUD-034: the month MUST NOT shadow the minute variable. The
    // previous code rebound `m` (the minute) to the month, so the
    // RFC3339 minute field carried the month value (e.g. 08 for
    // August) and the true minute was lost.
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep031_unit_zeek_transport_parses_documented_notice() {
        let json = r#"{"ts":1755650000.5,"uid":"C1","id.orig_h":"192.0.2.10","id.orig_p":54321,"id.resp_h":"198.51.100.7","id.resp_p":80,"proto":"tcp","note":"Scan::Port_Scan","msg":"Port scan detected","src":"192.0.2.10","dst":"198.51.100.7","p":80,"n":42,"actions":["Notice::ACTION_LOG"],"dropped":false}"#;
        let mut transport = JsonLinesZeekTransport::new(json.as_bytes());
        let notices = transport.read_notices().unwrap();
        assert_eq!(notices.len(), 1);
        let n = &notices[0];
        assert_eq!(n.uid, "C1");
        assert_eq!(n.orig_h.as_deref(), Some("192.0.2.10"));
        assert_eq!(n.resp_p, Some(80));
        assert_eq!(n.note, "Scan::Port_Scan");
        assert_eq!(n.n, Some(42));
        // AUD-034: the full timestamp must carry the TRUE minute, not
        // the month. ts=1755650000 is 2025-08-20T00:33:20Z; the
        // previous formatter corrupted the minute field to 08.
        assert_eq!(n.observed_at, "2025-08-20T00:33:20.500Z");
    }

    #[test]
    fn aud034_unit_zeek_rfc3339_keeps_minute_uncorrupted() {
        // AUD-034 hostile regression: the epoch formatter previously
        // shadowed the minute variable with the month, so the minute
        // field of the RFC3339 timestamp carried the month value. A
        // date/hour prefix check can never catch this - the full
        // timestamp must be exact.
        assert_eq!(
            chrono_like_seconds_to_rfc3339(1_755_650_000),
            "2025-08-20T00:33:20"
        );
        // Second sample with a different minute guards against a
        // month-equals-minute coincidence (minute 34 vs month 08).
        assert_eq!(
            chrono_like_seconds_to_rfc3339(1_755_650_099),
            "2025-08-20T00:34:59"
        );
        // Late-year sample: month 12 must not corrupt the minute.
        assert_eq!(
            chrono_like_seconds_to_rfc3339(1_765_972_800),
            "2025-12-17T12:00:00"
        );
    }

    #[test]
    fn ep031_unit_zeek_transport_skips_header_and_blank_lines() {
        let json = "{\"_path\":\"notice\",\"_write_ts\":1755650000}\n\n{\"uid\":\"C2\",\"note\":\"Weird::TCP_No_Data\",\"msg\":\"weird\"}\n";
        let mut transport = JsonLinesZeekTransport::new(json.as_bytes());
        let notices = transport.read_notices().unwrap();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].note, "Weird::TCP_No_Data");
    }

    #[test]
    fn ep031_unit_zeek_transport_fails_closed_on_malformed() {
        let json = "{\"uid\":\"C3\",\"note\":\nnot-json";
        let mut transport = JsonLinesZeekTransport::new(json.as_bytes());
        let err = transport.read_notices().unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
    }

    #[test]
    fn ep031_unit_zeek_transport_fails_closed_on_notice_missing_fields() {
        // A record with `note` but no uid/observed shape is still a
        // notice-shaped record; missing optional fields default, but a
        // record that cannot satisfy the documented shape fails
        // closed.
        let json = "{\"note\":12345}";
        let mut transport = JsonLinesZeekTransport::new(json.as_bytes());
        let err = transport.read_notices().unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
    }
}
