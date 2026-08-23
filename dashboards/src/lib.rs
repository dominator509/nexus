//! EP-038 M5 dashboards crate (SPEC-007 dashboards section; node contract).
//!
//! Real, validated Grafana-format dashboard configs backed by the M1
//! metric/health/SLO catalog semantics. The validator is the authority
//! that proves every dashboard:
//!
//! 1. is syntactically valid JSON with the required Grafana fields
//!    (uid, title, panels, datasource references, templating variables,
//!    query expressions),
//! 2. references ONLY canonical metric ids from the real M4 ops catalog
//!    (`nexus_observability_ops::ops_metric_definitions`), the M1
//!    canonical fixture metrics, and the rule/slo ids declared in the
//!    M1-owned `alerts/catalog.yaml` + `alerts/slo-catalog.yaml`,
//! 3. contains no secret-shaped literals (redaction-safe),
//! 4. never renders green from no data (threshold steps whose first
//!    bucket maps null/no-data to green are rejected),
//! 5. uses no unsafe high-cardinality raw labels in selectors.
//!
//! Honest boundary: these dashboards are VALIDATED CONFIG backed by the
//! M1/M4 catalog. No Grafana server, Prometheus server, or OTel
//! collector is exercised by this crate; runtime rendering is NOT
//! asserted here.

use nexus_domain::Privacy;
use nexus_observability::model::{contains_secret_shaped, MetricDefinition};
use nexus_observability::vocabulary::{CardinalityPolicy, MetricKind, StabilityLevel};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

/// A minimal Grafana dashboard document model. Only the fields the
/// validator actually reasons about are declared; unknown JSON fields
/// are ignored (Grafana accepts extra fields).
///
/// Field names mirror the Grafana wire format verbatim (camelCase);
/// they are the JSON contract, not style choices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct DashboardDocument {
    pub uid: String,
    pub title: String,
    #[serde(default)]
    pub schemaVersion: Option<u64>,
    #[serde(default)]
    pub panels: Vec<Panel>,
    #[serde(default)]
    pub templating: Option<Templating>,
    #[serde(default)]
    pub datasource: Option<Value>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Panel {
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub datasource: Option<Value>,
    #[serde(default)]
    pub targets: Vec<Target>,
    #[serde(default)]
    pub fieldConfig: Option<FieldConfig>,
    #[serde(default)]
    pub gridPos: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Target {
    #[serde(default)]
    pub expr: Option<String>,
    #[serde(default)]
    pub refId: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct FieldConfig {
    #[serde(default)]
    pub defaults: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Templating {
    #[serde(default)]
    pub list: Vec<TemplateVar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVar {
    pub name: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
}

/// One validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub code: String,
    pub detail: String,
}

impl Finding {
    fn new(code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            detail: detail.into(),
        }
    }
}

/// Build the canonical metric catalog the dashboards may reference.
///
/// Real sources, no invented ids:
/// - the M4 ops catalog (`ops_metric_definitions`),
/// - the M1 canonical fixture metrics (model.rs tests use these as the
///   canonical surface: `nexus.requests.total`, `nexus.workflow.duration`,
///   `nexus.api.availability`),
/// - every rule id in `alerts/catalog.yaml` and slo id in
///   `alerts/slo-catalog.yaml` (M1-owned contract configs).
pub fn canonical_catalog() -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for def in nexus_observability_ops::ops_metric_definitions() {
        ids.insert(def.id);
    }
    // M1 canonical fixture metrics (model.rs unit surface).
    for id in [
        "nexus.requests.total",
        "nexus.workflow.duration",
        "nexus.api.availability",
    ] {
        ids.insert(id.to_string());
    }
    // Alert rule ids + SLO ids from the M1 contract catalogs. Anchored
    // to the workspace root (parent of this crate) so the catalog is
    // identical no matter the caller's cwd.
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(Path::new("."));
    for path in ["alerts/catalog.yaml", "alerts/slo-catalog.yaml"] {
        if let Ok(text) = std::fs::read_to_string(repo_root.join(path)) {
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("- id:") {
                    let id = rest.trim().trim_matches('"').trim_matches('\'');
                    if !id.is_empty() {
                        ids.insert(id.to_string());
                    }
                }
            }
        }
    }
    ids
}

/// Validate one dashboard document against the canonical catalog.
pub fn validate_dashboard(doc: &DashboardDocument, catalog: &BTreeSet<String>) -> Vec<Finding> {
    let mut findings = Vec::new();

    if doc.uid.trim().is_empty() {
        findings.push(Finding::new(
            "missing_uid",
            "dashboard uid must be non-empty",
        ));
    } else if !doc
        .uid
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' || c == '.')
    {
        findings.push(Finding::new(
            "invalid_uid",
            format!(
                "dashboard uid {:?} must be lowercase alnum/dash/underscore/dot",
                doc.uid
            ),
        ));
    }

    if doc.title.trim().is_empty() {
        findings.push(Finding::new(
            "missing_title",
            "dashboard title must be non-empty",
        ));
    }

    if doc.panels.is_empty() {
        findings.push(Finding::new(
            "no_panels",
            "dashboard must contain at least one panel",
        ));
    }

    // Datasource reference must exist at dashboard or panel level.
    let has_ds = doc.datasource.is_some() || doc.panels.iter().any(|p| p.datasource.is_some());
    if !has_ds {
        findings.push(Finding::new(
            "missing_datasource",
            "dashboard or every panel must declare a datasource reference",
        ));
    }

    for panel in &doc.panels {
        if panel.title.trim().is_empty() {
            findings.push(Finding::new(
                "panel_no_title",
                format!("panel {} has no title", panel.id),
            ));
        }
        if panel.r#type.is_empty() {
            findings.push(Finding::new(
                "panel_no_type",
                format!("panel {} has no type", panel.id),
            ));
        }
        if panel.targets.is_empty() {
            findings.push(Finding::new(
                "panel_no_targets",
                format!("panel {} has no query targets", panel.id),
            ));
        }
        for target in &panel.targets {
            let expr = match &target.expr {
                Some(e) if !e.trim().is_empty() => e.trim(),
                _ => {
                    findings.push(Finding::new(
                        "panel_empty_expr",
                        format!("panel {} has an empty query expression", panel.id),
                    ));
                    continue;
                }
            };
            validate_selector(panel.id, expr, catalog, &mut findings);
        }
        validate_no_green_on_nodata(panel, &mut findings);
        validate_redaction_of_json(panel, &mut findings);
    }

    if let Some(t) = &doc.templating {
        for var in &t.list {
            if var.name.trim().is_empty() {
                findings.push(Finding::new(
                    "template_var_no_name",
                    "templating variable must have a name",
                ));
            }
            if let Some(q) = &var.query {
                if contains_secret_shaped(q) {
                    findings.push(Finding::new(
                        "template_var_secret",
                        format!("templating variable {} query is secret-shaped", var.name),
                    ));
                }
            }
        }
    }

    findings
}

/// Validate one PromQL-style query expression: the first token must be
/// a canonical metric id and any label selectors must not carry raw
/// high-cardinality values (uuid/email/artifact-key shaped).
fn validate_selector(
    panel_id: u64,
    expr: &str,
    catalog: &BTreeSet<String>,
    findings: &mut Vec<Finding>,
) {
    // Strip wrapper functions to reach the metric name:
    // rate(metric[5m]), sum(metric), avg by (x) (metric). We skip
    // function-name '(' pairs, NOT alphabetic metric-name characters.
    let mut rest = expr.trim();
    loop {
        let bytes = rest.as_bytes();
        let mut i = 0;
        while i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            i += 1;
        }
        if i == 0 || i >= bytes.len() || bytes[i] != b'(' {
            break;
        }
        // Function call: skip the name and the opening paren.
        rest = rest[i + 1..].trim_start();
    }
    // The metric name is the leading dotted/underscore token before
    // `{`, `[`, whitespace, `(`, or `)`.
    let metric: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .collect();

    if metric.is_empty() {
        findings.push(Finding::new(
            "panel_unparseable_expr",
            format!(
                "panel {} expression {:?} has no metric token",
                panel_id, expr
            ),
        ));
        return;
    }

    if !catalog.contains(&metric) {
        findings.push(Finding::new(
            "unknown_metric",
            format!(
                "panel {} expression {:?} references metric {:?} not in the canonical catalog",
                panel_id, expr, metric
            ),
        ));
        return;
    }

    // Label selector scan: reject raw high-cardinality values.
    if let Some(start) = rest.find('{') {
        if let Some(end) = rest.find('}') {
            let labels = &rest[start + 1..end];
            for part in labels.split(',') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                if let Some((_k, v)) = part.split_once('=') {
                    let v = v.trim().trim_matches('"').trim_matches('\'');
                    if looks_high_cardinality(v) {
                        findings.push(Finding::new(
                            "high_cardinality_label",
                            format!(
                                "panel {} expression {:?} uses raw high-cardinality label value {:?}",
                                panel_id, expr, v
                            ),
                        ));
                    }
                }
            }
        }
    }
}

fn looks_high_cardinality(v: &str) -> bool {
    if v.is_empty() {
        return false;
    }
    // UUID / canonical nexus id shape: 0x or 0197... or 8-4-4-4-12 hex.
    let uuid_like = v.len() == 36
        && v.chars().filter(|c| *c == '-').count() == 4
        && v.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if uuid_like {
        return true;
    }
    // Long artifact/request shaped id: >= 32 chars of hex/alnum.
    if v.len() >= 32 && v.chars().all(|c| c.is_ascii_alphanumeric()) {
        return true;
    }
    // Email-shaped.
    v.contains('@') || v.starts_with("AKIA") || v.contains("sk-") || v.starts_with("ghp_")
}

/// Reject green-on-no-data: a panel whose threshold steps map the null
/// (no-data) bucket to a green color would render healthy from absence
/// of data. That is the exact SPEC-007 anti-pattern the gate must stop.
fn validate_no_green_on_nodata(panel: &Panel, findings: &mut Vec<Finding>) {
    let Some(fc) = &panel.fieldConfig else {
        return;
    };
    let Some(defaults) = &fc.defaults else {
        return;
    };
    let Some(thresholds) = defaults.get("thresholds") else {
        return;
    };
    let Some(steps) = thresholds.get("steps").and_then(Value::as_array) else {
        return;
    };
    // Grafana: the first step has value null and is the bucket for
    // values below the next threshold, including no-data rendering.
    if let Some(first) = steps.first() {
        let color = first
            .get("color")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let value_null = first.get("value").map(|v| v.is_null()).unwrap_or(false);
        if value_null && color == "green" {
            findings.push(Finding::new(
                "green_on_nodata",
                format!(
                    "panel {} thresholds map null/no-data bucket to green (no-data must never render healthy)",
                    panel.id
                ),
            ));
        }
    }
}

/// Scan every string in the panel JSON for secret-shaped literals.
fn validate_redaction_of_json(panel: &Panel, findings: &mut Vec<Finding>) {
    let Ok(value) = serde_json::to_value(panel) else {
        return;
    };
    scan_strings(&value, &mut |s| {
        if contains_secret_shaped(s) {
            findings.push(Finding::new(
                "secret_literal",
                format!("panel {} contains a secret-shaped literal", panel.id),
            ));
        }
    });
}

fn scan_strings(value: &Value, f: &mut impl FnMut(&str)) {
    match value {
        Value::String(s) => f(s),
        Value::Array(items) => {
            for item in items {
                scan_strings(item, f);
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                scan_strings(v, f);
            }
        }
        _ => {}
    }
}

/// Load every dashboard JSON under `dir` (top-level, `*.json`), parse,
/// and validate against the canonical catalog. Returns
/// `(file, findings)` pairs; only files that parse are reported, and a
/// parse failure is itself a finding with the `unparseable` code.
pub fn validate_dashboard_dir(dir: &Path) -> Vec<(String, Vec<Finding>)> {
    let catalog = canonical_catalog();
    let mut out = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                out.push((
                    name.clone(),
                    vec![Finding::new("unreadable", format!("cannot read: {e}"))],
                ));
                continue;
            }
        };
        let value: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                out.push((
                    name.clone(),
                    vec![Finding::new("unparseable", format!("invalid JSON: {e}"))],
                ));
                continue;
            }
        };
        // Redaction scan over the whole raw document (headers, notes).
        let mut findings = Vec::new();
        scan_strings(&value, &mut |s| {
            if contains_secret_shaped(s) {
                findings.push(Finding::new(
                    "secret_literal",
                    format!("{name} contains a secret-shaped literal"),
                ));
            }
        });
        let doc: DashboardDocument = match serde_json::from_value(value) {
            Ok(d) => d,
            Err(e) => {
                findings.push(Finding::new(
                    "model_reject",
                    format!("dashboard model rejected: {e}"),
                ));
                out.push((name.clone(), findings));
                continue;
            }
        };
        findings.extend(validate_dashboard(&doc, &catalog));
        out.push((name, findings));
    }
    out
}

/// Build a `MetricDefinition` from the M1 contract - used by the gate
/// and tests to prove the catalog itself is contract-valid.
pub fn contract_metric(
    id: &str,
    description: &str,
    kind: MetricKind,
    labels: Vec<String>,
) -> Result<MetricDefinition, nexus_observability::ObservabilityError> {
    MetricDefinition::new(
        id,
        description,
        "1",
        kind,
        labels,
        CardinalityPolicy::DenyHighCardinality,
        Privacy::Public,
        "ep-038",
        StabilityLevel::Stable,
        "sum",
    )
}
