//! Prometheus text exposition format 0.0.4 writer (SPEC-007; node
//! contract fallback: "Use local structured logs and Prometheus metrics
//! when external collectors are unavailable").
//!
//! Grammar facts verified against the Prometheus documentation:
//! - Line-oriented; each line ends with `\n`; the last line must end
//!   with a line-feed.
//! - `# HELP <metric> <docstring>` - backslash and line-feed in the
//!   docstring must be escaped as `\\` and `\n`.
//! - `# TYPE <metric> counter|gauge|histogram|summary|untyped`.
//! - Sample: `metric_name{label="value",...} value [timestamp]` -
//!   label values escape backslash, double-quote, and line-feed as
//!   `\\`, `\"`, `\n`.
//! - The TYPE line must precede the first sample for that metric.
//!
//! This writer consumes already-redacted values from the M1 export
//! boundary; it never receives raw secrets.

use nexus_observability::model::MetricDefinition;
use nexus_observability::vocabulary::MetricKind;

/// Escape a Prometheus docstring (backslash -> `\\`, newline -> `\n`).
pub fn escape_help(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n")
}

/// Escape a Prometheus label value (backslash, double-quote, newline).
pub fn escape_label_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Validate a metric name against Prometheus expression language
/// restrictions: `[a-zA-Z_:][a-zA-Z0-9_:]*`. Returns the name if valid.
pub fn validate_metric_name(name: &str) -> Result<&str, nexus_observability::ObservabilityError> {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == ':' => {}
        _ => {
            return Err(nexus_observability::ObservabilityError::validation(
                "prometheus metric name must start [a-zA-Z_:]",
            ));
        }
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') {
        return Err(nexus_observability::ObservabilityError::validation(
            "prometheus metric name must match [a-zA-Z_:][a-zA-Z0-9_:]*",
        ));
    }
    Ok(name)
}

/// Validate a label name: `[a-zA-Z_][a-zA-Z0-9_]*` (no colon allowed).
pub fn validate_label_name(name: &str) -> Result<&str, nexus_observability::ObservabilityError> {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => {
            return Err(nexus_observability::ObservabilityError::validation(
                "prometheus label name must start [a-zA-Z_]",
            ));
        }
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(nexus_observability::ObservabilityError::validation(
            "prometheus label name must match [a-zA-Z_][a-zA-Z0-9_]*",
        ));
    }
    Ok(name)
}

/// Render the `{label="value",...}` label segment for a sample line.
/// Labels are sorted by name for reproducible output.
fn render_labels(
    labels: &[(String, String)],
) -> Result<String, nexus_observability::ObservabilityError> {
    let mut sorted: Vec<_> = labels.to_vec();
    sorted.sort();
    let mut out = String::new();
    for (i, (k, v)) in sorted.iter().enumerate() {
        validate_label_name(k)?;
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("{}=\"{}\"", k, escape_label_value(v)));
    }
    Ok(out)
}

/// The Prometheus TYPE token for an M1 metric kind.
pub fn prometheus_type(kind: &MetricKind) -> &'static str {
    match kind {
        MetricKind::Counter => "counter",
        MetricKind::Gauge => "gauge",
        MetricKind::Histogram | MetricKind::Distribution => "histogram",
    }
}

/// Render one metric family (HELP + TYPE + one sample line) in text
/// exposition format 0.0.4. The output is newline-terminated.
///
/// Histogram/Distribution kinds render as a `histogram` TYPE but the
/// bucket layout is owned by a later milestone; M2 renders the family
/// header only for those kinds and returns an error for the sample.
/// Counter/Gauge render the full family with one sample.
pub fn render_family(
    definition: &MetricDefinition,
    value: f64,
    labels: &[(String, String)],
) -> Result<String, nexus_observability::ObservabilityError> {
    let name = validate_metric_name(&definition.id)?;
    let mut out = String::new();
    out.push_str(&format!(
        "# HELP {} {}\n",
        name,
        escape_help(&definition.description)
    ));
    out.push_str(&format!(
        "# TYPE {} {}\n",
        name,
        prometheus_type(&definition.kind)
    ));
    match definition.kind {
        MetricKind::Counter | MetricKind::Gauge => {
            let labels_seg = render_labels(labels)?;
            if labels_seg.is_empty() {
                out.push_str(&format!("{} {}\n", name, format_value(value)));
            } else {
                out.push_str(&format!(
                    "{}{{{}}} {}\n",
                    name,
                    labels_seg,
                    format_value(value)
                ));
            }
            Ok(out)
        }
        MetricKind::Histogram | MetricKind::Distribution => {
            Err(nexus_observability::ObservabilityError::unsupported_signal(
                "histogram bucket layout is owned by a later milestone",
            ))
        }
    }
}

/// Format a float per Go `strconv` semantics used by Prometheus:
/// finite values print with enough digits; `NaN`, `+Inf`, `-Inf` are
/// emitted as `NaN`, `+Inf`, `-Inf`.
pub fn format_value(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v == f64::INFINITY {
        return "+Inf".to_string();
    }
    if v == f64::NEG_INFINITY {
        return "-Inf".to_string();
    }
    // Deterministic shortest representation with trailing .0 for whole
    // numbers so the value parses as a float (Go strconv style).
    let s = format!("{v}");
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        format!("{s}.0")
    } else {
        s
    }
}
