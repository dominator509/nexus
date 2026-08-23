//! nexus-otel: OpenTelemetry provider layer for Nexus (SPEC-007; EP-038
//! M2).
//!
//! M2 owns the deterministic provider behavior on top of the M1
//! contract crate (`nexus-observability`):
//! - OTLP/JSON serialization for traces, metrics, and logs, hand-rolled
//!   against the authoritative `opentelemetry-proto` wire format
//!   (trace_id 32-hex base16, span_id 16-hex base16, camelCase field
//!   names, fixed64 timestamps as decimal strings, proto3 enum values).
//! - Local fallback per the EP-038 node contract: Prometheus text
//!   exposition format 0.0.4 and bounded structured JSON log lines.
//! - Redaction before egress: the `export` boundary accepts only
//!   `RedactedEnvelope` and re-verifies `assert_exportable()` before any
//!   byte is produced. No API accepts raw observed events.
//!
//! M2 does NOT claim: a Prometheus server, an OTel collector, Grafana,
//! GlitchTip, or any remote delivery. Those are later milestones.
//!
//! Permanent invariants proven by tests:
//! - RAW EVENT != EXPORTABLE TELEMETRY (redaction at the boundary)
//! - secret-shaped canaries never appear in OTLP/JSON, Prometheus text,
//!   or structured log output
//! - deterministic output for identical inputs (sorted attributes,
//!   canonical hex, fixed field order)

pub mod export;
pub mod otlp;
pub mod prometheus;
pub mod structured;

pub use export::{
    export_log, export_metric, export_prometheus_family, export_span, export_structured_log,
    validate_trace_context,
};
pub use otlp::{severity_number, severity_text, supported_signals};
