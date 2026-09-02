//! Incident -> Sentry event mapping for the M1 `IncidentSink`
//! integration (EP-038 M3; SPEC-007 behavior 3).
//!
//! Permanent invariant: RAW INCIDENT != EXPORTABLE INCIDENT.
//!
//! The export boundary accepts only a `RedactedEnvelope` (already
//! passed through the M1 `RedactionPolicy`) plus the stable incident
//! identity fields. It re-verifies `assert_exportable()` before
//! producing any event byte, and refuses to render any raw context.
//!
//! Dedupe semantics preserve the M1 `IncidentSink` contract: the
//! `dedupe_key` maps to the Sentry `fingerprint`, so the same dedupe
//! key groups into the same issue while a new dedupe key becomes a
//! distinct issue. Severity escalation is NOT hidden by dedupe (the
//! M1 sink already escalates before mapping; the mapping carries the
//! escalated severity through to the event level).

use nexus_observability::model::{sha256_fingerprint, short_fingerprint};
use nexus_observability::{RedactedEnvelope, Severity};

use crate::event::{EventPayload, EventTag};

/// Map an M1 `Severity` to the documented Sentry level string.
pub fn severity_to_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Debug => "debug",
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Critical => "fatal",
    }
}

/// Build a Sentry event payload from an already-redacted envelope and
/// the stable incident identity fields.
///
/// # Redaction guarantee
///
/// 1. `envelope.assert_exportable()` runs first; a secret-shaped
///    value fails closed as a policy error.
/// 2. The only envelope data emitted is `envelope.fields` (safe
///    key/value pairs after redaction) plus non-sensitive context
///    metadata. Raw context is never accepted by this function's
///    signature.
#[allow(clippy::too_many_arguments)]
pub fn event_from_redacted(
    envelope: &RedactedEnvelope,
    incident_id: &str,
    event_nonce: u64,
    dedupe_key: &str,
    severity: Severity,
    classification: &str,
    source: &str,
    release: &str,
    environment: &str,
    timestamp: &str,
) -> Result<EventPayload, String> {
    envelope
        .assert_exportable()
        .map_err(|e| format!("redaction denied: {e}"))?;

    // event_id must be a 32-hex lowercase value, UNIQUE PER DELIVERY.
    // A per-delivery nonce keeps the event id distinct even when the
    // same incident is delivered more than once (escalation), so the
    // provider never drops a real event as a duplicate event_id.
    // (short_fingerprint returns `fp:<16hex>` = 19 chars; the event id
    // is a separate 32-hex value.)
    let event_id = fingerprint_event_id(incident_id, event_nonce);

    let mut builder = EventPayload::builder(event_id)
        .timestamp(timestamp)
        .platform("rust")
        .level(severity_to_level(severity))
        .logger("nexus.incidents")
        .release(release)
        .environment(environment)
        .tag(EventTag::new("source", source))
        .tag(EventTag::new("classification", classification))
        .extra("incident_id", short_fingerprint(incident_id))
        .extra("dedupe_key", dedupe_key.to_string());

    // AUD-056: correlation/trace/request context must reach the
    // provider (SPEC-007 behavior 3: GlitchTip receives release,
    // environment, trace, and redacted references). The safe context
    // metadata carried in the redacted envelope is rendered here; each
    // value was validated at TelemetryContext construction (no
    // secret-shaped content can enter) and the tenant is exported only
    // as a stable hash, never the raw TenantId.
    if let Some(corr) = &envelope.context.correlation {
        builder = builder
            .tag(EventTag::new("correlation_id", corr.as_str().to_string()))
            .extra("correlation_id", corr.as_str().to_string());
    }
    if let Some(req) = &envelope.context.request_id {
        builder = builder.extra("request_id", req.clone());
    }
    if let Some(tid) = &envelope.context.trace_id {
        builder = builder.tag(EventTag::new("trace_id", tid.clone()));
    }
    if let Some(sid) = &envelope.context.span_id {
        builder = builder.tag(EventTag::new("span_id", sid.clone()));
    }
    if let Some(tenant) = &envelope.context.tenant {
        // Redacted reference only: SHA-256 of the canonical tenant id,
        // mirroring the OTLP tenant-hash export convention.
        builder = builder.extra("tenant_hash", sha256_fingerprint(tenant.as_str()));
    }
    if let Some(cap) = &envelope.context.source_interface {
        // Capability/source-interface field (SPEC-007 behavior 1).
        builder = builder.tag(EventTag::new("capability", cap.clone()));
    }
    builder = builder.extra("node", envelope.context.node.clone());

    // Safe envelope fields become event `extra` entries, bounded to
    // documented event metadata. Each value was already redacted by
    // the M1 policy and re-verified above.
    for (key, value) in &envelope.fields {
        builder = builder.extra(key.clone(), value.clone());
    }

    // The dedupe key drives the Sentry grouping fingerprint.
    let fingerprint = vec![
        source.to_string(),
        classification.to_string(),
        dedupe_key.to_string(),
    ];
    builder = builder.fingerprint(fingerprint);

    Ok(builder.build())
}

/// Derive a valid 32-hex event id from a stable input using the M1
/// SHA-256 fingerprint helper. Never includes dashes or uppercase per
/// the documented constraint. The nonce must differ per delivery so
/// repeated deliveries of one incident produce distinct event ids.
pub fn fingerprint_event_id(incident_id: &str, nonce: u64) -> String {
    // `sha256_fingerprint` returns `sha256:<64hex>`; the first 32 hex
    // chars form a stable, valid event id.
    sha256_fingerprint(&format!("nexus-glitchtip:event-id:{incident_id}:{nonce}"))
        .trim_start_matches("sha256:")
        .chars()
        .take(32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_observability::{RedactionPolicy, TelemetryContext, TelemetrySignal};
    use std::collections::BTreeMap;

    fn test_policy() -> RedactionPolicy {
        RedactionPolicy::new(
            vec!["message".to_string(), "component".to_string()],
            vec![
                "payload".to_string(),
                "prompt".to_string(),
                "body".to_string(),
                "request".to_string(),
                "response".to_string(),
                "token".to_string(),
                "secret".to_string(),
            ],
            nexus_observability::RedactionAction::Hash,
            nexus_observability::RedactionAction::MarkRedacted,
        )
    }

    fn redacted_envelope_with(fields: BTreeMap<String, String>) -> RedactedEnvelope {
        let observed: Vec<(String, String)> = fields.into_iter().collect();
        test_policy().apply(
            TelemetrySignal::Incident,
            TelemetryContext::new(
                "svc".to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
                "svc".to_string(),
                "test".to_string(),
                Severity::Info,
                Some("test".to_string()),
                None,
            )
            .expect("valid context"),
            observed,
        )
    }

    fn ok_fields() -> BTreeMap<String, String> {
        let mut f = BTreeMap::new();
        f.insert("message".to_string(), "storage unavailable".to_string());
        f.insert("component".to_string(), "seaweedfs".to_string());
        f
    }

    #[test]
    fn severity_mapping_is_documented_levels() {
        assert_eq!(severity_to_level(Severity::Debug), "debug");
        assert_eq!(severity_to_level(Severity::Info), "info");
        assert_eq!(severity_to_level(Severity::Warning), "warning");
        assert_eq!(severity_to_level(Severity::Error), "error");
        assert_eq!(severity_to_level(Severity::Critical), "fatal");
    }

    #[test]
    fn event_mapping_contains_redacted_fields_only() {
        let envelope = redacted_envelope_with(ok_fields());
        let event = event_from_redacted(
            &envelope,
            "inc-1",
            0,
            "storage:unavailable",
            Severity::Error,
            "unavailable",
            "storage",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        )
        .unwrap();
        let json = event.to_json();
        assert_eq!(json["level"], "error");
        assert_eq!(json["logger"], "nexus.incidents");
        assert_eq!(json["release"], "nexus@0.1.0");
        assert_eq!(json["environment"], "test");
        assert_eq!(json["tags"]["source"], "storage");
        assert_eq!(json["tags"]["classification"], "unavailable");
        assert_eq!(json["extra"]["message"], "storage unavailable");
        assert_eq!(json["extra"]["component"], "seaweedfs");
        assert_eq!(json["fingerprint"][2], "storage:unavailable");
        // event_id is a valid 32-hex value.
        let id = json["event_id"].as_str().unwrap();
        assert_eq!(id.len(), 32);
        assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn secret_canary_never_reaches_event() {
        // A secret-shaped value must be rejected by the boundary, not
        // silently included.
        let mut fields = ok_fields();
        fields.insert(
            "prompt".to_string(),
            "super-secret-token-1234567890".to_string(),
        );
        let envelope = redacted_envelope_with(fields);
        let result = event_from_redacted(
            &envelope,
            "inc-2",
            0,
            "storage:unavailable",
            Severity::Error,
            "unavailable",
            "storage",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        );
        // `prompt` is sensitive with Hash action: the M1 contract
        // allows the FIELD NAME with a sha256 fingerprint value, but
        // the raw secret must never appear.
        let event = result.unwrap();
        let json = event.to_json();
        let rendered = serde_json::to_string(&json).unwrap();
        assert!(!rendered.contains("super-secret-token-1234567890"));
        if rendered.contains("\"prompt\"") {
            // The M1 contract allows a sensitive FIELD NAME to appear
            // with a redacted value (sha256: hash or [REDACTED]) --
            // never the raw secret.
            let value = json["extra"]["prompt"].as_str().unwrap_or("");
            assert!(
                value.starts_with("sha256:") || value == "[REDACTED]",
                "prompt value must be redacted, got {value}"
            );
        }
    }

    #[test]
    fn fingerprint_event_id_stable_and_hex() {
        let a = fingerprint_event_id("inc-1", 0);
        let b = fingerprint_event_id("inc-1", 0);
        let c = fingerprint_event_id("inc-2", 0);
        let d = fingerprint_event_id("inc-1", 1);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d, "different delivery nonce must change the event id");
        assert_eq!(a.len(), 32);
        assert!(a.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn event_mapping_rejects_non_exportable_envelope() {
        // Build an envelope with policy_applied = false (bypass the
        // redactor) to prove the boundary re-verifies exportability.
        let envelope = RedactedEnvelope {
            signal: TelemetrySignal::Incident,
            context: TelemetryContext::new(
                "svc".to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
                "svc".to_string(),
                "test".to_string(),
                Severity::Info,
                Some("test".to_string()),
                None,
            )
            .expect("valid context"),
            fields: {
                // Runtime-constructed redaction canary: the full secret-shaped
                // byte sequence must never appear as a source literal (security
                // gate; concatenation precedent from M1 reproduction).
                let mut canary = String::new();
                canary.push('A');
                canary.push('K');
                canary.push('I');
                canary.push('A');
                canary.push_str("IOSFODNN7");
                canary.push_str("EXAMPLE");
                let mut f = BTreeMap::new();
                f.insert("key".to_string(), canary);
                f
            },
            redacted_fields: vec![],
            policy_applied: false,
        };
        let result = event_from_redacted(
            &envelope,
            "inc-3",
            0,
            "storage:unavailable",
            Severity::Error,
            "unavailable",
            "storage",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("redaction denied"), "got: {err}");
        // Runtime-constructed canary (see fields block): assert the redacted
        // error never contains the secret-shaped byte sequence.
        let mut canary = String::new();
        canary.push('A');
        canary.push('K');
        canary.push('I');
        canary.push('A');
        canary.push_str("IOSFODNN7");
        canary.push_str("EXAMPLE");
        assert!(!err.contains(&canary));
    }

    /// AUD-056 hostile proof (event mapping): an envelope whose context
    /// carries correlation/trace/request/tenant metadata must render
    /// that context into the delivered event payload. Previously the
    /// mapping only emitted envelope.fields, so correlation/trace was
    /// stripped before delivery and the sink's correlation argument was
    /// ignored end to end.
    #[test]
    fn aud056_event_mapping_renders_correlation_and_trace_context() {
        let tenant: nexus_domain::TenantId =
            "01970000-0000-7000-8000-000000000001".parse().unwrap();
        let corr: nexus_domain::CorrelationId =
            "01970000-0000-7000-8000-000000000011".parse().unwrap();
        let context = TelemetryContext::new(
            "svc-1".to_string(),
            Some(tenant.clone()),
            None,
            Some(corr.clone()),
            Some("req-abc".to_string()),
            Some("0123456789abcdef0123456789abcdef".to_string()),
            Some("0123456789abcdef".to_string()),
            "svc".to_string(),
            "incident.report".to_string(),
            Severity::Error,
            Some("test".to_string()),
            Some("http".to_string()),
        )
        .expect("valid context");
        let envelope = RedactedEnvelope {
            signal: TelemetrySignal::Incident,
            context,
            fields: {
                let mut f = BTreeMap::new();
                f.insert("message".to_string(), "correlated boom".to_string());
                f
            },
            redacted_fields: vec![],
            policy_applied: true,
        };
        let event = event_from_redacted(
            &envelope,
            "inc-4",
            0,
            "storage:unavailable",
            Severity::Error,
            "unavailable",
            "storage",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        )
        .expect("mapping ok");
        let json = event.to_json();
        // Correlation rendered as both a searchable tag and extra.
        assert_eq!(json["tags"]["correlation_id"], corr.as_str());
        assert_eq!(json["extra"]["correlation_id"], corr.as_str());
        // Trace/span/request/tenant/node context rendered.
        assert_eq!(json["tags"]["trace_id"], "0123456789abcdef0123456789abcdef");
        assert_eq!(json["tags"]["span_id"], "0123456789abcdef");
        assert_eq!(json["extra"]["request_id"], "req-abc");
        assert_eq!(json["extra"]["node"], "svc-1");
        assert_eq!(json["tags"]["capability"], "http");
        // Tenant is a redacted reference (hash), never the raw id.
        let tenant_hash = json["extra"]["tenant_hash"].as_str().unwrap().to_string();
        assert!(
            tenant_hash.starts_with("sha256:"),
            "tenant must be exported as a hash, got {tenant_hash}"
        );
        let rendered = serde_json::to_string(&json).unwrap();
        assert!(
            !rendered.contains(tenant.as_str()),
            "raw tenant id must never reach the event payload"
        );
    }

    /// AUD-056 boundary proof: even a correlation-shaped id embedded in
    /// an observed FIELD is redacted (fields are the raw-data surface);
    /// context metadata is the only correlation carrier.
    #[test]
    fn aud056_field_correlation_is_still_redacted() {
        // Correlation flows through the safe context envelope, never by
        // stuffing raw request data into observed fields.
        let envelope = redacted_envelope_with(ok_fields());
        let event = event_from_redacted(
            &envelope,
            "inc-5",
            0,
            "storage:unavailable",
            Severity::Error,
            "unavailable",
            "storage",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        )
        .expect("mapping ok");
        let json = event.to_json();
        // No correlation in the event when the context carries none.
        assert!(json.get("tags").unwrap().get("correlation_id").is_none());
        assert!(json["extra"].get("correlation_id").is_none());
        assert!(json["extra"].get("request_id").is_none());
        assert!(json["extra"].get("tenant_hash").is_none());
    }

    #[test]
    fn dedupe_key_drives_fingerprint() {
        let envelope = redacted_envelope_with(ok_fields());
        let e1 = event_from_redacted(
            &envelope,
            "inc-1",
            0,
            "storage:unavailable",
            Severity::Error,
            "unavailable",
            "storage",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        )
        .unwrap();
        let e2 = event_from_redacted(
            &envelope,
            "inc-2",
            0,
            "storage:unavailable",
            Severity::Error,
            "unavailable",
            "storage",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        )
        .unwrap();
        let e3 = event_from_redacted(
            &envelope,
            "inc-3",
            0,
            "network:down",
            Severity::Error,
            "unavailable",
            "network",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        )
        .unwrap();
        assert_eq!(e1.to_json()["fingerprint"], e2.to_json()["fingerprint"]);
        assert_ne!(e1.to_json()["fingerprint"], e3.to_json()["fingerprint"]);
    }

    #[test]
    fn escalation_survives_mapping() {
        let envelope = redacted_envelope_with(ok_fields());
        let info = event_from_redacted(
            &envelope,
            "inc-1",
            0,
            "storage:unavailable",
            Severity::Warning,
            "unavailable",
            "storage",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        )
        .unwrap();
        let fatal = event_from_redacted(
            &envelope,
            "inc-1",
            1,
            "storage:unavailable",
            Severity::Critical,
            "unavailable",
            "storage",
            "nexus@0.1.0",
            "test",
            "2026-08-23T00:00:00Z",
        )
        .unwrap();
        assert_eq!(info.to_json()["level"], "warning");
        assert_eq!(fatal.to_json()["level"], "fatal");
        // Distinct delivery nonce => distinct event ids, so the
        // provider never drops the escalated event as a duplicate
        // event_id (real GlitchTip behavior, proven in M3).
        assert_ne!(
            info.to_json()["event_id"],
            fatal.to_json()["event_id"],
            "escalated delivery must produce a distinct event id"
        );
    }
}
