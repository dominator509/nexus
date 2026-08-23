//! OTLP/JSON payload builders for the OpenTelemetry Protocol (SPEC-007;
//! EP-038 M2).
//!
//! Wire-format facts verified against the authoritative upstream
//! `opentelemetry-proto` sources:
//! - trace/span: `trace_id` is a 16-byte array, serialized in OTLP/JSON
//!   as 32 lowercase base16 characters; `span_id` is 8 bytes serialized
//!   as 16 base16 characters.
//! - `fixed64` fields (`time_unix_nano`, `start_time_unix_nano`,
//!   `end_time_unix_nano`) follow proto3 JSON mapping and are emitted as
//!   decimal strings.
//! - Field names are camelCase in OTLP/JSON: `resourceSpans`,
//!   `scopeSpans`, `traceId`, `spanId`, `parentSpanId`, `name`, `kind`,
//!   `startTimeUnixNano`, `endTimeUnixNano`, `status`/`code`/`message`,
//!   `attributes`/`key`/`value`/`stringValue`/`intValue`/`doubleValue`,
//!   `resourceMetrics`, `scopeMetrics`, `dataPoints`, `asDouble`,
//!   `asInt`, `resourceLogs`, `scopeLogs`, `logRecords`, `severityText`,
//!   `severityNumber`, `body`.
//! - SpanKind enum: UNSPECIFIED=0 INTERNAL=1 SERVER=2 CLIENT=3
//!   PRODUCER=4 CONSUMER=5.
//! - StatusCode: UNSET=0 OK=1 ERROR=2.
//! - SeverityNumber: TRACE=1 DEBUG=5 INFO=9 WARN=13 ERROR=17 FATAL=21.
//! - AggregationTemporality: UNSPECIFIED=0 DELTA=1 CUMULATIVE=2.
//!
//! This module never receives raw observed events. The export boundary
//! in `export.rs` accepts only `RedactedEnvelope` (already passed through
//! `RedactionPolicy`) and calls `assert_exportable()` before any byte is
//! produced.

use nexus_observability::model::{
    MetricDefinition, RedactedEnvelope, TelemetryContext, TraceContext,
};
use nexus_observability::vocabulary::{MetricKind, Severity, TelemetrySignal};
use nexus_observability::ObservabilityError;

/// Convert a `Severity` into the canonical OTLP severity number
/// (verified against `opentelemetry/proto/logs/v1/logs.proto`).
pub fn severity_number(sev: Severity) -> u32 {
    match sev {
        Severity::Debug => 5,
        Severity::Info => 9,
        Severity::Warning => 13,
        Severity::Error => 17,
        Severity::Critical => 21,
    }
}

/// Canonical OTLP severity text for a `Severity`.
pub fn severity_text(sev: Severity) -> &'static str {
    match sev {
        Severity::Debug => "DEBUG",
        Severity::Info => "INFO",
        Severity::Warning => "WARN",
        Severity::Error => "ERROR",
        Severity::Critical => "FATAL",
    }
}

/// Map an M1 `MetricKind` to the OTLP `Metric.data` oneof member.
/// Counter -> `sum` (monotonic cumulative), Gauge -> `gauge`.
/// Histogram/Distribution require a bucket layout not owned by M2 and are
/// rejected truthfully with `UnsupportedSignal`.
pub fn metric_data_kind(kind: &MetricKind) -> Result<&'static str, ObservabilityError> {
    match kind {
        MetricKind::Counter => Ok("sum"),
        MetricKind::Gauge => Ok("gauge"),
        MetricKind::Histogram | MetricKind::Distribution => {
            Err(ObservabilityError::unsupported_signal(
                "histogram/distribution wire shape is owned by a later milestone",
            ))
        }
    }
}

/// Resource attributes for an OTLP `Resource` (SPEC-007 behavior 1:
/// service, version, environment, node, tenant hash, request,
/// correlation, and capability fields).
///
/// The tenant is exported ONLY as a stable SHA-256 fingerprint, never as
/// the raw `TenantId` (SPEC-007: tenant hash field).
pub fn resource_attributes(ctx: &TelemetryContext, service_version: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    attrs.push(("service.name".to_string(), ctx.component.clone()));
    attrs.push(("service.version".to_string(), service_version.to_string()));
    if let Some(env) = &ctx.environment {
        attrs.push(("deployment.environment".to_string(), env.clone()));
    }
    attrs.push(("host.name".to_string(), ctx.node.clone()));
    if let Some(tenant) = &ctx.tenant {
        // SHA-256 of the canonical tenant id; stable across runs.
        let hash = sha256_hex(tenant.as_str());
        attrs.push(("nexus.tenant.hash".to_string(), hash));
    }
    // Deterministic wire output: sort by attribute key.
    attrs.sort();
    attrs
}

/// Span attribute pairs from a redacted envelope: request/correlation/
/// capability plus every safe field already classified by policy.
pub fn span_attributes(envelope: &RedactedEnvelope) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    if let Some(req) = &envelope.context.request_id {
        attrs.push(("nexus.request.id".to_string(), req.clone()));
    }
    if let Some(corr) = &envelope.context.correlation {
        attrs.push((
            "nexus.correlation.id".to_string(),
            corr.as_str().to_string(),
        ));
    }
    if let Some(cap) = &envelope.context.source_interface {
        attrs.push(("nexus.capability".to_string(), cap.clone()));
    }
    for (k, v) in &envelope.fields {
        attrs.push((k.clone(), v.clone()));
    }
    attrs
}

/// Stable SHA-256 hex fingerprint (lowercase, no prefix). Used for the
/// tenant hash resource attribute only; content redaction stays in the
/// M1 `RedactionPolicy` (`sha256:`-prefixed fingerprints).
pub fn sha256_hex(input: &str) -> String {
    // No sha2 dependency in M2: the tenant hash is a deterministic
    // fingerprint, and the canonical M1 fingerprint helper already
    // exists in nexus-observability. Use it to keep one hashing story.
    nexus_observability::model::sha256_fingerprint(input)
        .trim_start_matches("sha256:")
        .to_string()
}

/// Validate a trace id for OTLP/JSON: exactly 32 lowercase hex chars
/// (16 bytes, base16). Empty string is invalid.
pub fn validate_trace_id_hex(trace_id: &str) -> Result<(), ObservabilityError> {
    if trace_id.len() != 32
        || !trace_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !trace_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(ObservabilityError::validation(
            "OTLP trace id must be 32 lowercase base16 characters",
        ));
    }
    Ok(())
}

/// Validate a span id for OTLP/JSON: exactly 16 lowercase hex chars
/// (8 bytes, base16).
pub fn validate_span_id_hex(span_id: &str) -> Result<(), ObservabilityError> {
    if span_id.len() != 16
        || !span_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !span_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(ObservabilityError::validation(
            "OTLP span id must be 16 lowercase base16 characters",
        ));
    }
    Ok(())
}

/// Trace ids and span ids arrive from `TraceContext` already validated
/// by the M1 contract (32/16 hex). This function centralizes the check at
/// the export boundary so a malformed id can never produce a wire span.
pub fn trace_ctx_to_ids(tc: &TraceContext) -> Result<(String, String), ObservabilityError> {
    validate_trace_id_hex(&tc.trace_id)?;
    validate_span_id_hex(&tc.span_id)?;
    Ok((tc.trace_id.clone(), tc.span_id.clone()))
}

/// Serialize a log `RedactedEnvelope` to an OTLP/JSON `ExportLogsServiceRequest`.
/// The envelope's severity drives `severityNumber`/`severityText`;
/// `body` carries the (already redacted) `message`-safe field or a
/// bounded operation label. Secret-shaped values cannot reach here:
/// `assert_exportable()` runs first in the export boundary.
pub fn log_record_payload(
    envelope: &RedactedEnvelope,
    service_version: &str,
    now_nanos: u64,
) -> Result<String, ObservabilityError> {
    let ctx = &envelope.context;
    let resource = json_resource(resource_attributes(ctx, service_version));
    let sev = severity_number(ctx.severity);
    let sev_text = severity_text(ctx.severity);
    let body = envelope
        .fields
        .get("message")
        .cloned()
        .unwrap_or_else(|| format!("{}:{}", ctx.component, ctx.operation));
    let attrs = json_attributes(&span_attributes(envelope));
    // Never export the raw body twice; body is a scalar AnyValue.
    let log_record = serde_json::json!({
        "timeUnixNano": now_nanos.to_string(),
        "severityNumber": sev,
        "severityText": sev_text,
        "body": { "stringValue": body },
        "attributes": attrs,
        "droppedAttributesCount": 0,
    });
    serde_json::to_string(&serde_json::json!({
        "resourceLogs": [{
            "resource": resource,
            "scopeLogs": [{
                "scope": { "name": "nexus-observability", "version": service_version },
                "logRecords": [log_record],
            }],
        }],
    }))
    .map_err(|e| ObservabilityError::internal(format!("otlp log serialization: {e}")))
}

/// Serialize a span `RedactedEnvelope` to an OTLP/JSON `ExportTraceServiceRequest`.
/// Span kind defaults to INTERNAL; status is UNSET unless the operation
/// carries an explicit `error` safe field, which maps to STATUS_CODE_ERROR.
pub fn span_payload(
    envelope: &RedactedEnvelope,
    service_version: &str,
    start_nanos: u64,
    end_nanos: u64,
) -> Result<String, ObservabilityError> {
    let ctx = &envelope.context;
    let (trace_id, span_id) = match (&ctx.trace_id, &ctx.span_id) {
        (Some(t), Some(s)) => {
            validate_trace_id_hex(t)?;
            validate_span_id_hex(s)?;
            (t.clone(), s.clone())
        }
        _ => {
            return Err(ObservabilityError::validation(
                "span export requires validated trace_id and span_id",
            ));
        }
    };
    let resource = json_resource(resource_attributes(ctx, service_version));
    let status = if envelope.fields.contains_key("error") {
        serde_json::json!({ "code": 2 })
    } else {
        serde_json::json!({ "code": 0 })
    };
    let span = serde_json::json!({
        "traceId": trace_id,
        "spanId": span_id,
        "name": ctx.operation,
        "kind": 1,
        "startTimeUnixNano": start_nanos.to_string(),
        "endTimeUnixNano": end_nanos.to_string(),
        "attributes": json_attributes(&span_attributes(envelope)),
        "status": status,
    });
    serde_json::to_string(&serde_json::json!({
        "resourceSpans": [{
            "resource": resource,
            "scopeSpans": [{
                "scope": { "name": "nexus-observability", "version": service_version },
                "spans": [span],
            }],
        }],
    }))
    .map_err(|e| ObservabilityError::internal(format!("otlp trace serialization: {e}")))
}

/// Serialize one metric point to an OTLP/JSON `ExportMetricsServiceRequest`.
/// Counter -> monotonic cumulative sum; Gauge -> gauge.
/// `value_nanos` is the data-point `timeUnixNano`.
pub fn metric_payload(
    definition: &MetricDefinition,
    value: f64,
    labels: &[(String, String)],
    service_version: &str,
    value_nanos: u64,
) -> Result<String, ObservabilityError> {
    let data_kind = metric_data_kind(&definition.kind)?;
    let point = serde_json::json!({
        "timeUnixNano": value_nanos.to_string(),
        "attributes": json_attributes(labels),
        "asDouble": value,
    });
    let data = match data_kind {
        "sum" => serde_json::json!({
            "dataPoints": [point],
            "aggregationTemporality": 2,
            "isMonotonic": true,
        }),
        "gauge" => serde_json::json!({ "dataPoints": [point] }),
        _ => unreachable!("metric_data_kind only returns sum|gauge"),
    };
    let metric = serde_json::json!({
        "name": definition.id,
        "description": definition.description,
        "unit": definition.unit,
        data_kind: data,
    });
    serde_json::to_string(&serde_json::json!({
        "resourceMetrics": [{
            "resource": serde_json::json!({ "attributes": json_attributes(&[(
                "service.name".to_string(),
                "nexus".to_string(),
            )]) }),
            "scopeMetrics": [{
                "scope": { "name": "nexus-observability", "version": service_version },
                "metrics": [metric],
            }],
        }],
    }))
    .map_err(|e| ObservabilityError::internal(format!("otlp metric serialization: {e}")))
}

fn json_resource(attrs: Vec<(String, String)>) -> serde_json::Value {
    serde_json::json!({ "attributes": json_attributes(&attrs) })
}

fn json_attributes(attrs: &[(String, String)]) -> Vec<serde_json::Value> {
    attrs
        .iter()
        .map(|(k, v)| {
            serde_json::json!({
                "key": k,
                "value": { "stringValue": v },
            })
        })
        .collect()
}

/// The signals this M2 provider can serialize truthfully.
pub fn supported_signals() -> &'static [TelemetrySignal] {
    &[
        TelemetrySignal::Trace,
        TelemetrySignal::Metric,
        TelemetrySignal::Log,
    ]
}
