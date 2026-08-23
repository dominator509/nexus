//! Local structured-log fallback (SPEC-007; node contract fallback:
//! "Use local structured logs and Prometheus metrics when external
//! collectors are unavailable").
//!
//! Emits one JSON object per line with bounded, already-redacted fields.
//! The export boundary (`export.rs`) only feeds this writer
//! `RedactedEnvelope` values whose `assert_exportable()` succeeded, so
//! secret-shaped values cannot appear here.

use nexus_observability::model::RedactedEnvelope;
use nexus_observability::ObservabilityError;

/// Render one envelope as a single JSON-lines record.
///
/// Shape (all values are the redacted, exportable ones):
/// `{"ts":<seconds>,"level":"INFO","service":"<component>","operation":"<op>","node":"<node>","fields":{...},"redacted":[...]}`
///
/// The raw body/payload fields are never included - the M1 policy
/// already dropped or hashed them; this writer only sees the safe
/// `fields` map.
pub fn structured_log_line(envelope: &RedactedEnvelope) -> Result<String, ObservabilityError> {
    let ctx = &envelope.context;
    let level = match ctx.severity {
        nexus_observability::vocabulary::Severity::Debug => "DEBUG",
        nexus_observability::vocabulary::Severity::Info => "INFO",
        nexus_observability::vocabulary::Severity::Warning => "WARN",
        nexus_observability::vocabulary::Severity::Error => "ERROR",
        nexus_observability::vocabulary::Severity::Critical => "CRITICAL",
    };
    let mut fields = serde_json::Map::new();
    for (k, v) in &envelope.fields {
        fields.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    if let Some(corr) = &ctx.correlation {
        fields.insert(
            "correlation_id".to_string(),
            serde_json::Value::String(corr.as_str().to_string()),
        );
    }
    if let Some(env) = &ctx.environment {
        fields.insert(
            "environment".to_string(),
            serde_json::Value::String(env.clone()),
        );
    }
    let record = serde_json::json!({
        "ts": ctx.timestamp,
        "level": level,
        "service": ctx.component,
        "operation": ctx.operation,
        "node": ctx.node,
        "fields": fields,
        "redacted": envelope.redacted_fields,
    });
    serde_json::to_string(&record)
        .map(|s| format!("{s}\n"))
        .map_err(|e| ObservabilityError::internal(format!("structured log serialization: {e}")))
}
