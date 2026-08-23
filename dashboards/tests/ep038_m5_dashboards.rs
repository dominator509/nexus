//! EP-038 M5 dashboards validation proofs (SPEC-007 dashboards).
//!
//! Every test here runs against the REAL validator and the REAL
//! canonical catalog (M4 ops metrics + M1 canonical metrics + M1
//! alert/slo catalogs). The negative tests prove the validator rejects
//! the exact anti-patterns the gate must stop: green-on-no-data,
//! unknown metric selectors, secret-shaped literals, missing required
//! Grafana fields, and raw high-cardinality label values.

use nexus_dashboards::{
    canonical_catalog, validate_dashboard, validate_dashboard_dir, DashboardDocument, Panel,
    Target, TemplateVar, Templating,
};
use nexus_observability::vocabulary::MetricKind;
use std::path::Path;

fn catalog() -> std::collections::BTreeSet<String> {
    canonical_catalog()
}

fn dashboard_with(panels: Vec<Panel>) -> DashboardDocument {
    DashboardDocument {
        uid: "nexus.test.dashboard".to_string(),
        title: "Test dashboard".to_string(),
        schemaVersion: Some(39),
        panels,
        templating: Some(Templating {
            list: vec![TemplateVar {
                name: "node".to_string(),
                query: Some("label_values(nexus.ops.health.composed, node)".to_string()),
                r#type: Some("query".to_string()),
            }],
        }),
        datasource: Some(serde_json::json!({"type": "prometheus", "uid": "prometheus"})),
        tags: vec!["test".to_string()],
    }
}

fn panel(id: u64, expr: &str) -> Panel {
    Panel {
        id,
        title: format!("panel {id}"),
        r#type: "timeseries".to_string(),
        datasource: Some(serde_json::json!({"type": "prometheus", "uid": "prometheus"})),
        targets: vec![Target {
            expr: Some(expr.to_string()),
            refId: Some("A".to_string()),
        }],
        fieldConfig: Some(nexus_dashboards::FieldConfig {
            defaults: Some(serde_json::json!({
                "thresholds": {
                    "mode": "absolute",
                    "steps": [
                        {"color": "grey", "value": null},
                        {"color": "green", "value": 1}
                    ]
                }
            })),
        }),
        gridPos: None,
    }
}

fn findings_for(doc: &DashboardDocument) -> Vec<nexus_dashboards::Finding> {
    validate_dashboard(doc, &catalog())
}

fn has_code(findings: &[nexus_dashboards::Finding], code: &str) -> bool {
    findings.iter().any(|f| f.code == code)
}

// ------------------------------------------------------------- catalog

#[test]
fn ep038_m5_catalog_contains_m4_ops_metrics() {
    let c = catalog();
    assert!(c.contains("nexus.ops.incidents.delivered"));
    assert!(c.contains("nexus.ops.incidents.failed"));
    assert!(c.contains("nexus.ops.health.composed"));
}

#[test]
fn ep038_m5_catalog_contains_m1_canonical_and_alert_slo_ids() {
    let c = catalog();
    assert!(c.contains("nexus.requests.total"));
    assert!(c.contains("nexus.workflow.duration"));
    assert!(c.contains("nexus.api.availability"));
    // From alerts/catalog.yaml
    assert!(c.contains("nexus.storage.provider_unavailable"));
    // From alerts/slo-catalog.yaml
    assert!(c.contains("nexus.slo.storage.put_success"));
}

// ------------------------------------------------------------- positive

#[test]
fn ep038_m5_valid_dashboard_passes() {
    let doc = dashboard_with(vec![panel(1, "nexus.ops.health.composed{node=\"$node\"}")]);
    assert!(findings_for(&doc).is_empty(), "{:?}", findings_for(&doc));
}

#[test]
fn ep038_m5_all_real_dashboards_validate() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let results = validate_dashboard_dir(&repo_root.join("dashboards"));
    assert!(!results.is_empty(), "no dashboard files found");
    for (name, findings) in &results {
        assert!(
            findings.is_empty(),
            "{name} failed validation: {findings:?}"
        );
    }
    // The gate must see exactly the three owned dashboards.
    let names: Vec<String> = results.iter().map(|(n, _)| n.clone()).collect();
    assert!(names.contains(&"nexus-health-overview.json".to_string()));
    assert!(names.contains(&"nexus-incidents-slo.json".to_string()));
    assert!(names.contains(&"nexus-metrics-ops.json".to_string()));
}

#[test]
fn ep038_m5_contract_metric_builds_valid_definitions() {
    // Proves the catalog entries are contract-valid M1 MetricDefinitions.
    let def = nexus_dashboards::contract_metric(
        "nexus.ops.incidents.delivered",
        "incident deliveries recorded",
        MetricKind::Counter,
        vec!["source".to_string(), "classification".to_string()],
    )
    .unwrap();
    assert_eq!(def.id, "nexus.ops.incidents.delivered");
    assert!(nexus_dashboards::contract_metric(
        "nexus.invalid id",
        "x",
        MetricKind::Counter,
        vec![],
    )
    .is_err());
}

// ------------------------------------------------------- negative tests

#[test]
fn ep038_m5_rejects_green_on_nodata() {
    // The exact anti-pattern: first threshold step maps null (no-data)
    // to green. No-data must never render healthy.
    let mut doc = dashboard_with(vec![panel(1, "nexus.ops.health.composed")]);
    if let Some(fc) = &mut doc.panels[0].fieldConfig {
        fc.defaults = Some(serde_json::json!({
            "thresholds": {
                "mode": "absolute",
                "steps": [
                    {"color": "green", "value": null},
                    {"color": "red", "value": 1}
                ]
            }
        }));
    }
    let findings = findings_for(&doc);
    assert!(
        has_code(&findings, "green_on_nodata"),
        "expected green_on_nodata, got {findings:?}"
    );
}

#[test]
fn ep038_m5_rejects_unknown_metric() {
    let doc = dashboard_with(vec![panel(1, "nexus.nonexistent.metric")]);
    let findings = findings_for(&doc);
    assert!(
        has_code(&findings, "unknown_metric"),
        "expected unknown_metric, got {findings:?}"
    );
}

#[test]
fn ep038_m5_rejects_secret_shaped_literal() {
    let doc = dashboard_with(vec![panel(1, "nexus.ops.health.composed")]);
    // Inject a secret-shaped literal into the panel JSON via a custom
    // panel string field (target expr is the normal spot). The canary
    // is constructed at runtime (M1/EP-036 precedent) so the source
    // never contains a full secret-shaped literal.
    let mut doc = doc;
    // Construct a full AKIA-shaped token at runtime (>= 20 chars) so
    // the source never contains a full secret-shaped literal
    // (M1/EP-036 precedent).
    let mut canary = String::from("AKIA");
    for (i, c) in "IOSFODNN7EXAMPLE1234".chars().enumerate() {
        let _ = i;
        canary.push(c);
    }
    canary.push_str(" key");
    doc.panels[0].title = canary;
    let findings = findings_for(&doc);
    assert!(
        has_code(&findings, "secret_literal"),
        "expected secret_literal, got {findings:?}"
    );
}

#[test]
fn ep038_m5_rejects_missing_required_fields() {
    let mut doc = dashboard_with(vec![panel(1, "nexus.ops.health.composed")]);
    doc.uid = "".to_string();
    doc.title = "  ".to_string();
    let findings = findings_for(&doc);
    assert!(has_code(&findings, "missing_uid"));
    assert!(has_code(&findings, "missing_title"));
}

#[test]
fn ep038_m5_rejects_no_panels() {
    let doc = dashboard_with(vec![]);
    let findings = findings_for(&doc);
    assert!(has_code(&findings, "no_panels"));
}

#[test]
fn ep038_m5_rejects_high_cardinality_raw_label() {
    // Raw artifact/uuid-shaped value in a selector must be rejected.
    let doc = dashboard_with(vec![panel(
        1,
        "nexus.ops.health.composed{node=\"0197000000000000000000000000000000000000000000000001\"}",
    )]);
    let findings = findings_for(&doc);
    assert!(
        has_code(&findings, "high_cardinality_label"),
        "expected high_cardinality_label, got {findings:?}"
    );
}

#[test]
fn ep038_m5_rejects_empty_expr() {
    let doc = dashboard_with(vec![panel(1, "   ")]);
    let findings = findings_for(&doc);
    assert!(
        has_code(&findings, "panel_empty_expr"),
        "expected panel_empty_expr, got {findings:?}"
    );
}

// ------------------------------------------------- unparseable file

#[test]
fn ep038_m5_rejects_malformed_json_file() {
    // The validator must fail closed on a broken file, not skip it.
    let dir = std::env::temp_dir().join("nexus-ep038-m5-bad-dash");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("broken.json"), "{ not json").unwrap();
    let results = validate_dashboard_dir(&dir);
    assert_eq!(results.len(), 1);
    assert!(has_code(&results[0].1, "unparseable"));
    std::fs::remove_dir_all(&dir).unwrap();
}
