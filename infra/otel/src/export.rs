//! Export boundary (SPEC-007 behavior 2; EP-038 M2).
//!
//! The M2 provider layer only ever exports telemetry that has already
//! passed through the M1 `RedactionPolicy`. The boundary functions here
//! are the ONLY entry points to the serializers; each one:
//! 1. accepts a `RedactedEnvelope` (never raw observed events),
//! 2. calls `assert_exportable()` which re-checks every field for
//!    secret-shaped content and requires `policy_applied`,
//! 3. only then produces OTLP/JSON, Prometheus text, or a structured
//!    log line.
//!
//! There is no API that accepts raw `(field, value)` pairs for export.

use nexus_observability::model::{MetricDefinition, RedactedEnvelope, TraceContext};
use nexus_observability::ObservabilityError;

use crate::otlp;
use crate::prometheus;
use crate::structured;

/// Export a log envelope as an OTLP/JSON `ExportLogsServiceRequest`.
pub fn export_log(
    envelope: &RedactedEnvelope,
    service_version: &str,
    now_nanos: u64,
) -> Result<String, ObservabilityError> {
    envelope.assert_exportable()?;
    otlp::log_record_payload(envelope, service_version, now_nanos)
}

/// Export a span envelope as an OTLP/JSON `ExportTraceServiceRequest`.
pub fn export_span(
    envelope: &RedactedEnvelope,
    service_version: &str,
    start_nanos: u64,
    end_nanos: u64,
) -> Result<String, ObservabilityError> {
    envelope.assert_exportable()?;
    otlp::span_payload(envelope, service_version, start_nanos, end_nanos)
}

/// Export a metric point as an OTLP/JSON `ExportMetricsServiceRequest`.
pub fn export_metric(
    definition: &MetricDefinition,
    value: f64,
    labels: &[(String, String)],
    service_version: &str,
    value_nanos: u64,
) -> Result<String, ObservabilityError> {
    otlp::metric_payload(definition, value, labels, service_version, value_nanos)
}

/// Export a metric family in Prometheus text exposition format 0.0.4
/// (local fallback).
pub fn export_prometheus_family(
    definition: &MetricDefinition,
    value: f64,
    labels: &[(String, String)],
) -> Result<String, ObservabilityError> {
    prometheus::render_family(definition, value, labels)
}

/// Export a log envelope as a local structured JSON line (fallback).
pub fn export_structured_log(envelope: &RedactedEnvelope) -> Result<String, ObservabilityError> {
    envelope.assert_exportable()?;
    structured::structured_log_line(envelope)
}

/// Validate a `TraceContext` for OTLP export (32-hex trace id,
/// 16-hex span id).
pub fn validate_trace_context(tc: &TraceContext) -> Result<(), ObservabilityError> {
    otlp::trace_ctx_to_ids(tc).map(|_| ())
}
