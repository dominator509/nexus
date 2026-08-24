//! Redacted deterministic evidence for the real inventory (EP-039 M3).
//!
//! Preserves the M1/M2 redaction guarantee: the evidence JSON never
//! contains secret-shaped values (sk-, ghp_, AKIA, Bearer, credentials,
//! private URLs with credentials). The M2 evidence document is used;
//! every string is scrubbed through the shared redaction boundary.

use nexus_supply_chain_policy::evidence::{redact_secret_shaped, EvidenceDocument};

use crate::inventory::InventoryReport;

/// Build a redacted evidence document for the real inventory.
pub fn inventory_evidence(report: &InventoryReport) -> EvidenceDocument {
    let body = serde_json::json!({
        "run_id": report.run_id,
        "package_count": report.package_count,
        "transitive_count": report.transitive_count,
        "workspace_count": report.workspace_count,
        "resolved_license_count": report.resolved_license_count,
        "missing_license_count": report.missing_license_count,
        "green_count": report.green_count,
        "review_count": report.review_count,
        "sidecar_count": report.sidecar_count,
        "external_count": report.external_count,
        "prohibited_count": report.prohibited_count,
        "unknown_count": report.unknown_count,
        "permitted_default_count": report.permitted_default_count,
        "packages": report.packages.iter().map(|p| {
            serde_json::json!({
                "name": p.name,
                "version": p.version,
                "license_spdx": p.license_spdx,
                "class": p.class,
                "license_clear": p.license_clear,
                "permitted_default": p.permitted_default,
                "reason": p.reason,
            })
        }).collect::<Vec<_>>(),
    })
    .to_string();
    EvidenceDocument {
        run_id: report.run_id.clone(),
        owner: "ep039-m3".to_string(),
        body,
        generated_at_ts: 1_700_000_000,
    }
}

/// Assert no secret-shaped value survives in a rendered evidence JSON.
pub fn assert_redacted(json: &str) -> bool {
    for marker in [
        "sk-",
        "pk-",
        "rk-",
        "ghp_",
        "gho_",
        "ghs_",
        "github_pat_",
        "AKIA",
        "Bearer ",
        "token=",
        "password=",
        "secret=",
        "xoxb-",
        "glpat-",
    ] {
        if json.contains(marker) {
            return false;
        }
    }
    true
}

/// Re-export the canonical redaction primitive for tests and the gate.
pub fn redact(value: &str) -> String {
    redact_secret_shaped(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_canary_never_survives() {
        let prefix = format!("sk{}", "-live");
        let secret = format!("{prefix}{}", "1234567890abcdef");
        let redacted = redact(&secret);
        assert!(!redacted.contains(&prefix));
        assert!(redacted.contains("REDACTED") || redacted.contains("***"));
    }

    #[test]
    fn evidence_document_redacts() {
        let body = serde_json::json!({
            "credential": format!("ghp_{}", "abc123"),
        })
        .to_string();
        let doc = EvidenceDocument {
            run_id: "run-1".to_string(),
            owner: "ep039-m3".to_string(),
            body,
            generated_at_ts: 1_700_000_000,
        };
        let json = doc.to_redacted_json();
        assert!(assert_redacted(&json));
    }
}
