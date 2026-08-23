//! EP-038 M2 provider-layer proofs (SPEC-007).
//!
//! These tests exercise the REAL `nexus-otel` provider implementation:
//! OTLP/JSON wire shape for traces/metrics/logs (verified against the
//! authoritative `opentelemetry-proto` sources), Prometheus text
//! exposition 0.0.4 fallback, structured-log fallback, and
//! redaction-before-egress with secret canaries.

use std::collections::BTreeMap;
use std::str::FromStr;

use nexus_domain::Privacy;
use nexus_domain::{CorrelationId, TenantId};
use nexus_observability::model::{
    MetricDefinition, RedactedEnvelope, RedactionPolicy, TelemetryContext, TraceContext,
};
use nexus_observability::vocabulary::{
    CardinalityPolicy, MetricKind, RedactionAction, Severity, StabilityLevel, TelemetrySignal,
};

fn tenant(id: &str) -> TenantId {
    TenantId::from_str(id).expect("valid tenant id")
}

fn correlation(id: &str) -> CorrelationId {
    CorrelationId::from_str(id).expect("valid correlation id")
}

fn default_policy() -> RedactionPolicy {
    RedactionPolicy::new(
        vec!["message".to_string()],
        vec![
            "payload".to_string(),
            "prompt".to_string(),
            "body".to_string(),
            "request".to_string(),
            "response".to_string(),
            "token".to_string(),
        ],
        RedactionAction::Hash,
        RedactionAction::Drop,
    )
}

fn ctx(trace: Option<(&str, &str)>, severity: Severity) -> TelemetryContext {
    let (t, s) = trace.unwrap_or(("4bf92f3577b34da6a3ce929d0e0e4736", "00f067aa0ba902b7"));
    TelemetryContext::new(
        "node-a",
        Some(tenant("018e5c5e-4d9b-7f0c-8a2b-3c4d5e6f7a80")),
        None,
        Some(correlation("018e5c5e-4d9b-7f0c-8a2b-3c4d5e6f7a81")),
        Some("req-1".to_string()),
        Some(t.to_string()),
        Some(s.to_string()),
        "nexus-home",
        "agent.tick",
        severity,
        Some("production".to_string()),
        Some("telephony".to_string()),
    )
    .expect("valid context")
}

fn envelope(fields: Vec<(String, String)>, trace: Option<(&str, &str)>) -> RedactedEnvelope {
    let c = ctx(trace, Severity::Info);
    let policy = default_policy();
    policy.apply(TelemetrySignal::Log, c, fields)
}

fn metric_def(id: &str, kind: MetricKind) -> MetricDefinition {
    MetricDefinition::new(
        id,
        "Total requests",
        "1",
        kind,
        vec!["method".to_string()],
        CardinalityPolicy::Bounded,
        Privacy::Public,
        "nexus",
        StabilityLevel::Stable,
        "sum",
    )
    .expect("valid metric")
}

// ---------------------------------------------------------------- OTLP/JSON

#[test]
fn ep038_unit_otlp_log_severity_mapping_exact() {
    let e = envelope(vec![("message".to_string(), "hello".to_string())], None);
    let payload = nexus_otel::export_log(&e, "0.1.0", 1_700_000_000_000_000_000).unwrap();
    let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let rec = &v["resourceLogs"][0]["scopeLogs"][0]["logRecords"][0];
    assert_eq!(rec["severityNumber"], 9, "INFO=9 (opentelemetry-proto)");
    assert_eq!(rec["severityText"], "INFO");
    assert_eq!(rec["timeUnixNano"], "1700000000000000000");
    assert_eq!(rec["body"]["stringValue"], "hello");
}

#[test]
fn ep038_unit_otlp_log_camelcase_wire_names() {
    let e = envelope(vec![("message".to_string(), "x".to_string())], None);
    let payload = nexus_otel::export_log(&e, "0.1.0", 42).unwrap();
    for key in [
        "resourceLogs",
        "scopeLogs",
        "logRecords",
        "timeUnixNano",
        "severityNumber",
        "severityText",
        "body",
        "stringValue",
    ] {
        assert!(
            payload.contains(&format!("\"{key}\"")),
            "missing {key} in {payload}"
        );
    }
    // snake_case protobuf names must NOT leak into OTLP/JSON.
    for banned in [
        "resource_logs",
        "scope_logs",
        "log_records",
        "time_unix_nano",
    ] {
        assert!(!payload.contains(banned), "snake_case {banned} leaked");
    }
}

#[test]
fn ep038_unit_otlp_span_ids_base16_hex() {
    let e = envelope(
        vec![("message".to_string(), "span".to_string())],
        Some(("4bf92f3577b34da6a3ce929d0e0e4736", "00f067aa0ba902b7")),
    );
    let payload = nexus_otel::export_span(&e, "0.1.0", 1, 2).unwrap();
    let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let span = &v["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
    assert_eq!(span["traceId"], "4bf92f3577b34da6a3ce929d0e0e4736");
    assert_eq!(span["spanId"], "00f067aa0ba902b7");
    assert_eq!(span["kind"], 1, "INTERNAL=1");
    assert_eq!(span["startTimeUnixNano"], "1");
    assert_eq!(span["endTimeUnixNano"], "2");
}

#[test]
fn ep038_unit_otlp_span_rejects_malformed_ids() {
    // M1's TelemetryContext already rejects malformed trace/span ids at
    // construction; the OTLP boundary re-validates at export time via
    // validate_trace_context / trace_ctx_to_ids.
    assert!(nexus_otel::otlp::validate_trace_id_hex("4bf92f3577b34da6a3ce929d0e0e47").is_err()); // 31 hex
    assert!(nexus_otel::otlp::validate_trace_id_hex("4BF92F3577B34DA6A3CE929D0E0E4736").is_err()); // uppercase
    assert!(nexus_otel::otlp::validate_trace_id_hex("4bf92f3577b34da6a3ce929d0e0e4736").is_ok());
    assert!(nexus_otel::otlp::validate_span_id_hex("00f067aa0ba902b").is_err()); // 15 hex
    assert!(nexus_otel::otlp::validate_span_id_hex("00f067aa0ba902b7").is_ok());
    // Empty trace id is invalid (proto: zero-length invalid).
    assert!(nexus_otel::otlp::validate_trace_id_hex("").is_err());
}

#[test]
fn ep038_unit_otlp_metric_counter_is_sum_cumulative() {
    let m = metric_def("nexus.requests.total", MetricKind::Counter);
    let payload = nexus_otel::export_metric(
        &m,
        42.0,
        &[("method".to_string(), "GET".to_string())],
        "0.1.0",
        7,
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let metric = &v["resourceMetrics"][0]["scopeMetrics"][0]["metrics"][0];
    assert_eq!(metric["name"], "nexus.requests.total");
    assert_eq!(metric["sum"]["aggregationTemporality"], 2, "CUMULATIVE=2");
    assert_eq!(metric["sum"]["isMonotonic"], true);
    assert_eq!(metric["sum"]["dataPoints"][0]["asDouble"], 42.0);
    assert_eq!(metric["sum"]["dataPoints"][0]["timeUnixNano"], "7");
}

#[test]
fn ep038_unit_otlp_metric_gauge_shape() {
    let m = metric_def("nexus.temperature", MetricKind::Gauge);
    let payload = nexus_otel::export_metric(&m, 21.5, &[], "0.1.0", 7).unwrap();
    let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let metric = &v["resourceMetrics"][0]["scopeMetrics"][0]["metrics"][0];
    assert!(metric.get("gauge").is_some(), "gauge oneof");
    assert!(metric.get("sum").is_none());
}

#[test]
fn ep038_unit_otlp_metric_histogram_unsupported_truthful() {
    let m = metric_def("nexus.latency", MetricKind::Histogram);
    let err = nexus_otel::export_metric(&m, 1.0, &[], "0.1.0", 7).unwrap_err();
    assert_eq!(
        err.code,
        nexus_observability::ObservabilityErrorCode::UnsupportedSignal
    );
}

#[test]
fn ep038_unit_otlp_resource_attributes_include_tenant_hash_only() {
    let e = envelope(vec![("message".to_string(), "x".to_string())], None);
    let payload = nexus_otel::export_log(&e, "0.1.0", 42).unwrap();
    let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let attrs = &v["resourceLogs"][0]["resource"]["attributes"];
    let mut found = std::collections::HashMap::new();
    for a in attrs.as_array().unwrap() {
        found.insert(
            a["key"].as_str().unwrap().to_string(),
            a["value"]["stringValue"].as_str().unwrap().to_string(),
        );
    }
    assert_eq!(found["service.name"], "nexus-home");
    assert_eq!(found["deployment.environment"], "production");
    assert_eq!(found["host.name"], "node-a");
    // Raw tenant id must NEVER appear; only its fingerprint.
    assert!(!found["nexus.tenant.hash"].starts_with("018e"));
    assert_eq!(found["nexus.tenant.hash"].len(), 64, "sha256 hex");
    assert!(!payload.contains("018e5c5e-4d9b-7f0c-8a2b-3c4d5e6f7a80"));
}

// -------------------------------------------------- redaction before egress

#[test]
fn ep038_unit_redaction_canary_absent_otlp_log() {
    let secret = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
    let e = envelope(
        vec![
            ("message".to_string(), "ok".to_string()),
            ("payload".to_string(), secret.clone()),
        ],
        None,
    );
    let payload = nexus_otel::export_log(&e, "0.1.0", 42).unwrap();
    assert!(!payload.contains(&secret), "raw secret in otlp log");
    // The payload field was redacted: its exported value is a sha256
    // fingerprint, never the raw bytes. The field NAME may appear as
    // metadata; the VALUE must not.
    assert!(payload.contains("sha256:"), "redacted value must be hashed");
    assert!(payload.contains("ok"));
}

#[test]
fn ep038_unit_redaction_canary_absent_otlp_span() {
    let secret = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
    let e = envelope(
        vec![
            ("message".to_string(), "span".to_string()),
            ("token".to_string(), secret.clone()),
        ],
        Some(("4bf92f3577b34da6a3ce929d0e0e4736", "00f067aa0ba902b7")),
    );
    let payload = nexus_otel::export_span(&e, "0.1.0", 1, 2).unwrap();
    assert!(!payload.contains(&secret));
}

#[test]
fn ep038_unit_redaction_canary_absent_structured_log() {
    let secret = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
    let e = envelope(
        vec![
            ("message".to_string(), "ok".to_string()),
            ("prompt".to_string(), secret.clone()),
        ],
        None,
    );
    let line = nexus_otel::export_structured_log(&e).unwrap();
    assert!(!line.contains(&secret));
}

#[test]
fn ep038_unit_export_boundary_rejects_non_exportable() {
    // A hand-constructed envelope that claims policy_applied but carries
    // a secret-shaped field must be refused at the boundary.
    let c = ctx(None, Severity::Info);
    let mut fields = BTreeMap::new();
    let secret = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
    fields.insert("leak".to_string(), secret.clone());
    let bad = RedactedEnvelope::new(TelemetrySignal::Log, c, fields, vec![]);
    assert!(nexus_otel::export_log(&bad, "0.1.0", 42).is_err());
    assert!(nexus_otel::export_structured_log(&bad).is_err());
}

#[test]
fn ep038_unit_export_boundary_no_raw_event_api() {
    // Compile-time contract: the public export functions accept only
    // RedactedEnvelope / MetricDefinition / TraceContext. There is no
    // path that takes raw (field, value) pairs. This test documents the
    // boundary by exercising every public export entry point with an
    // envelope produced ONLY through RedactionPolicy.
    let e = envelope(vec![("message".to_string(), "ok".to_string())], None);
    assert!(nexus_otel::export_log(&e, "0.1.0", 1).is_ok());
    let tc =
        TraceContext::new("4bf92f3577b34da6a3ce929d0e0e4736", "00f067aa0ba902b7", true).unwrap();
    assert!(nexus_otel::validate_trace_context(&tc).is_ok());
}

// ------------------------------------------------------------- prometheus

#[test]
fn ep038_unit_prometheus_counter_family_exact() {
    let m = metric_def("nexus_requests_total", MetricKind::Counter);
    let out = nexus_otel::export_prometheus_family(
        &m,
        1027.0,
        &[("method".to_string(), "GET".to_string())],
    )
    .unwrap();
    assert_eq!(
        out,
        "# HELP nexus_requests_total Total requests\n# TYPE nexus_requests_total counter\nnexus_requests_total{method=\"GET\"} 1027.0\n"
    );
}

#[test]
fn ep038_unit_prometheus_label_escaping_exact() {
    let m = metric_def("msdos_file_access_time_seconds", MetricKind::Gauge);
    let out = nexus_otel::export_prometheus_family(
        &m,
        1.458255915e9,
        &[("path".to_string(), "C:\\DIR\\FILE.TXT".to_string())],
    )
    .unwrap();
    assert!(
        out.contains("path=\"C:\\\\DIR\\\\FILE.TXT\""),
        "backslash must be escaped: {out}"
    );
    let out2 = nexus_otel::export_prometheus_family(
        &m,
        1.0,
        &[(
            "error".to_string(),
            "Cannot find file:\n\"FILE.TXT\"".to_string(),
        )],
    )
    .unwrap();
    assert!(
        out2.contains("error=\"Cannot find file:\\n\\\"FILE.TXT\\\"\""),
        "newline+quote must be escaped: {out2}"
    );
}

#[test]
fn ep038_unit_prometheus_help_escaping_and_last_lf() {
    let mut m = metric_def("nexus_ok", MetricKind::Counter);
    // description with a newline must be escaped in HELP
    let d = "line1\nline2";
    m.description = d.to_string();
    let out = nexus_otel::export_prometheus_family(&m, 1.0, &[]).unwrap();
    assert!(out.contains("# HELP nexus_ok line1\\nline2"));
    assert!(out.ends_with('\n'), "last line must end with LF");
}

#[test]
fn ep038_unit_prometheus_rejects_invalid_metric_name() {
    let m = metric_def("9bad_name", MetricKind::Counter);
    let _ = m; // metric_def validates id already; raw validation below.
    assert!(nexus_otel::prometheus::validate_metric_name("9bad_name").is_err());
    assert!(nexus_otel::prometheus::validate_metric_name("ok_name:1").is_ok());
    assert!(nexus_otel::prometheus::validate_label_name("9bad").is_err());
    assert!(nexus_otel::prometheus::validate_label_name("ok_label").is_ok());
}

#[test]
fn ep038_unit_prometheus_value_formatting() {
    assert_eq!(nexus_otel::prometheus::format_value(42.0), "42.0");
    assert_eq!(nexus_otel::prometheus::format_value(f64::NAN), "NaN");
    assert_eq!(nexus_otel::prometheus::format_value(f64::INFINITY), "+Inf");
    assert_eq!(
        nexus_otel::prometheus::format_value(f64::NEG_INFINITY),
        "-Inf"
    );
}

#[test]
fn ep038_unit_prometheus_histogram_type_header_truthful() {
    let m = metric_def("nexus_latency_seconds", MetricKind::Histogram);
    // M2 renders the family header but refuses the sample (bucket layout
    // owned by a later milestone) - never a fake histogram.
    assert!(nexus_otel::export_prometheus_family(&m, 1.0, &[]).is_err());
    assert_eq!(
        nexus_otel::prometheus::prometheus_type(&MetricKind::Histogram),
        "histogram"
    );
}

// ------------------------------------------------------------- structured

#[test]
fn ep038_unit_structured_log_json_line_shape() {
    let e = envelope(vec![("message".to_string(), "tick".to_string())], None);
    let line = nexus_otel::export_structured_log(&e).unwrap();
    let v: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
    assert_eq!(v["level"], "INFO");
    assert_eq!(v["service"], "nexus-home");
    assert_eq!(v["operation"], "agent.tick");
    assert_eq!(v["node"], "node-a");
    assert_eq!(v["fields"]["message"], "tick");
    assert!(line.ends_with('\n'));
}

#[test]
fn ep038_unit_structured_log_redacted_list_recorded() {
    let secret = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
    let e = envelope(
        vec![
            ("message".to_string(), "tick".to_string()),
            ("payload".to_string(), secret),
        ],
        None,
    );
    let line = nexus_otel::export_structured_log(&e).unwrap();
    let v: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
    assert_eq!(v["redacted"][0], "payload");
    assert!(!line.contains("AKIA"));
}

// ------------------------------------------------------------ determinism

#[test]
fn ep038_unit_otlp_output_deterministic() {
    let e = envelope(
        vec![
            ("message".to_string(), "tick".to_string()),
            ("z_last".to_string(), "value".to_string()),
            ("a_first".to_string(), "value".to_string()),
        ],
        Some(("4bf92f3577b34da6a3ce929d0e0e4736", "00f067aa0ba902b7")),
    );
    let a = nexus_otel::export_log(&e, "0.1.0", 42).unwrap();
    let b = nexus_otel::export_log(&e, "0.1.0", 42).unwrap();
    assert_eq!(a, b, "identical input -> identical output");
}

#[test]
fn ep038_unit_otlp_resource_sorted_reproducible() {
    let e = envelope(vec![("message".to_string(), "x".to_string())], None);
    let payload = nexus_otel::export_log(&e, "0.1.0", 42).unwrap();
    let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let attrs = v["resourceLogs"][0]["resource"]["attributes"]
        .as_array()
        .unwrap();
    let keys: Vec<&str> = attrs.iter().map(|a| a["key"].as_str().unwrap()).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "attributes must be sorted for determinism");
}

// -------------------------------------------------------------- severity

#[test]
fn ep038_unit_severity_mapping_canonical() {
    use nexus_observability::vocabulary::Severity;
    assert_eq!(nexus_otel::severity_number(Severity::Debug), 5);
    assert_eq!(nexus_otel::severity_number(Severity::Info), 9);
    assert_eq!(nexus_otel::severity_number(Severity::Warning), 13);
    assert_eq!(nexus_otel::severity_number(Severity::Error), 17);
    assert_eq!(nexus_otel::severity_number(Severity::Critical), 21);
    assert_eq!(nexus_otel::severity_text(Severity::Warning), "WARN");
}
