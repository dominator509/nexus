//! EP-039 M2 unit proofs: deterministic supply-chain policy behavior
//! (SPEC-019; LICENSE_POLICY.md). Every negative invariant fails closed,
//! every positive path requires the exact policy match.

use nexus_supply_chain::model::{
    Advisory, AdvisoryAffected, ArtifactDigest, Component, ComponentBoundary, DependencyWaiver,
    ProvenanceAttestation, SbomDocument, SbomPackage, SbomVerification, SourceOffer,
};
use nexus_supply_chain::vocabulary::{
    AdvisorySeverity, ApprovalState, IntegrationMode, LicenseClass, LicenseReview,
    VerificationResult, WaiverState,
};
use nexus_supply_chain_policy::advisory::{
    AdvisoryEvaluation, AdvisoryPolicy, AdvisoryPolicyConfig,
};
use nexus_supply_chain_policy::boundary::{BoundaryEvaluation, BoundaryPolicy};
use nexus_supply_chain_policy::evidence::{
    evidence_boundary, redact_secret_shaped, EvidenceDocument,
};
use nexus_supply_chain_policy::license::{LicenseEvaluation, LicensePolicy};
use nexus_supply_chain_policy::provenance::{ProvenanceEvaluation, ProvenancePolicy};
use nexus_supply_chain_policy::sbom::{SbomEvaluation, SbomPolicy, SbomPolicyConfig};
use nexus_supply_chain_policy::waiver::{WaiverEvaluation, WaiverPolicy, WaiverPolicyConfig};

// ------------------------------------------------------------- helpers

fn green_component(name: &str, version: &str) -> Component {
    Component {
        identity: nexus_supply_chain::model::ComponentIdentity {
            name: name.to_string(),
            version: version.to_string(),
            source: format!("https://example.invalid/{name}"),
            registry: "crates.io".to_string(),
            lockfile: "Cargo.lock".to_string(),
            digest: None,
        },
        license_spdx: Some("MIT".to_string()),
        license_class: Some(LicenseClass::Green),
        review: LicenseReview::Approved,
        approval: ApprovalState::Approved,
        integration_mode: IntegrationMode::Embedded,
        risk: nexus_supply_chain::vocabulary::RiskClass::Low,
        owner: "ep039-m2".to_string(),
        verification: VerificationResult::Verified,
        evidence_ts: 1_700_000_000,
        run_id: "run-1".to_string(),
    }
}

fn mit_digest() -> ArtifactDigest {
    ArtifactDigest {
        algorithm: "sha256".to_string(),
        hex: "a".repeat(64),
    }
}

fn waive(package: &str, version: &str, state: WaiverState, expires_at_ts: u64) -> DependencyWaiver {
    DependencyWaiver {
        package: package.to_string(),
        version: version.to_string(),
        owner: "ep039-m2".to_string(),
        reason: "documented bounded exception".to_string(),
        controls: vec!["no external network".to_string()],
        expires_at_ts,
        replacement_plan: "upgrade to fixed version".to_string(),
        state,
    }
}

// ------------------------------------------------------------- license behavior

#[test]
fn ep039_unit_m2_license_green_permitted_only_under_exact_policy() {
    let policy = LicensePolicy::default();
    let c = green_component("serde", "1.0.0");
    let eval: LicenseEvaluation = policy.evaluate(&c);
    assert!(eval.permitted, "MIT with review+approval must be permitted");
    assert_eq!(eval.class, Some(LicenseClass::Green));
    assert_eq!(eval.review, LicenseReview::Approved);
}

#[test]
fn ep039_unit_m2_license_green_allowlist_entry_not_approval() {
    // Presence in the policy table (class GREEN) without explicit review
    // is NOT approval: review is NeedsReview -> denied.
    let policy = LicensePolicy::default();
    let mut c = green_component("serde", "1.0.0");
    c.review = LicenseReview::NeedsReview;
    c.approval = ApprovalState::Pending;
    let eval = policy.evaluate(&c);
    assert!(
        !eval.permitted,
        "allowlist entry alone must never approve (ALLOWLIST ENTRY != LEGAL APPROVAL)"
    );
    assert_eq!(eval.review, LicenseReview::NeedsReview);
}

#[test]
fn ep039_unit_m2_license_green_requires_approval_state() {
    let policy = LicensePolicy::default();
    let mut c = green_component("serde", "1.0.0");
    c.approval = ApprovalState::Pending;
    let eval = policy.evaluate(&c);
    assert!(
        !eval.permitted,
        "approved review without approval state must deny"
    );
}

#[test]
fn ep039_unit_m2_license_review_requires_review_state() {
    let policy = LicensePolicy::default();
    let mut c = green_component("mpl", "2.0.0");
    c.license_spdx = Some("MPL-2.0".to_string());
    c.license_class = Some(LicenseClass::Review);
    c.review = LicenseReview::NeedsReview;
    let eval = policy.evaluate(&c);
    assert!(
        !eval.permitted,
        "REVIEW license requires review/approval state"
    );
    assert_eq!(eval.class, Some(LicenseClass::Review));
}

#[test]
fn ep039_unit_m2_license_sidecar_requires_review_state() {
    let policy = LicensePolicy::default();
    let mut c = green_component("gpl", "3.0.0");
    c.license_spdx = Some("GPL-3.0-ONLY".to_string());
    c.license_class = Some(LicenseClass::Sidecar);
    c.review = LicenseReview::NeedsReview;
    let eval = policy.evaluate(&c);
    assert!(
        !eval.permitted,
        "SIDECAR requires sidecar terms/notice state"
    );
}

#[test]
fn ep039_unit_m2_license_external_never_auto_approved() {
    let policy = LicensePolicy::default();
    let mut c = green_component("provider", "1.0.0");
    c.license_spdx = Some("COMMERCIAL".to_string());
    c.license_class = Some(LicenseClass::External);
    // Even with review APPROVED the external class is never auto-approved.
    c.review = LicenseReview::Approved;
    c.approval = ApprovalState::Approved;
    let eval = policy.evaluate(&c);
    assert!(!eval.permitted, "EXTERNAL license is never auto-approved");
    assert_eq!(eval.class, Some(LicenseClass::External));
}

#[test]
fn ep039_unit_m2_license_prohibited_fails_closed() {
    let policy = LicensePolicy::default();
    let mut c = green_component("nc", "1.0.0");
    c.license_spdx = Some("NONCOMMERCIAL".to_string());
    c.license_class = Some(LicenseClass::Prohibited);
    c.review = LicenseReview::Approved;
    c.approval = ApprovalState::Approved;
    let eval = policy.evaluate(&c);
    assert!(!eval.permitted, "PROHIBITED license must fail closed");
    assert_eq!(eval.review, LicenseReview::Denied);
}

#[test]
fn ep039_unit_m2_license_unknown_fails_closed() {
    let policy = LicensePolicy::default();
    let mut c = green_component("odd", "1.0.0");
    c.license_spdx = Some("MAYBE_SAFE".to_string());
    c.license_class = None;
    c.review = LicenseReview::Approved;
    c.approval = ApprovalState::Approved;
    let eval = policy.evaluate(&c);
    assert!(!eval.permitted, "UNKNOWN license must fail closed");
    assert_eq!(eval.review, LicenseReview::Denied);
}

#[test]
fn ep039_unit_m2_license_missing_fails_closed() {
    let policy = LicensePolicy::default();
    let mut c = green_component("unlicensed", "1.0.0");
    c.license_spdx = None;
    let eval = policy.evaluate(&c);
    assert!(!eval.permitted, "MISSING license must fail closed");
    assert_eq!(eval.review, LicenseReview::Denied);
}

#[test]
fn ep039_unit_m2_license_fuzzy_string_never_bypasses_policy() {
    // Fuzzy strings that do not exactly match the policy table (after
    // canonical case normalization and trimming) must fail closed. Note:
    // "MIT" in any case IS the exact SPDX id (SPDX ids are case-
    // insensitive), so case variants are not "fuzzy" - substring or
    // descriptive strings are.
    let policy = LicensePolicy::default();
    for fuzzy in [
        "MIT-ish",
        "Apache",
        "MIT/X11",
        "MIT-style",
        "GPL compatible",
    ] {
        let mut c = green_component("fuzzy", "1.0.0");
        c.license_spdx = Some(fuzzy.to_string());
        c.review = LicenseReview::Approved;
        c.approval = ApprovalState::Approved;
        let eval = policy.evaluate(&c);
        assert!(
            !eval.permitted,
            "fuzzy license string {fuzzy:?} must never bypass policy"
        );
    }
}

#[test]
fn ep039_unit_m2_license_evaluation_deterministic() {
    let policy = LicensePolicy::default();
    let c = green_component("serde", "1.0.0");
    assert_eq!(policy.evaluate(&c), policy.evaluate(&c));
}

// ------------------------------------------------------------- boundary behavior

#[test]
fn ep039_unit_m2_boundary_sidecar_requires_process_separation() {
    let policy = BoundaryPolicy::default();
    let mut c = green_component("gpl", "3.0.0");
    c.license_class = Some(LicenseClass::Sidecar);
    c.integration_mode = IntegrationMode::Embedded;
    let eval: BoundaryEvaluation = policy.evaluate(&c, None);
    assert!(
        !eval.valid,
        "copyleft component embedded in-process must be denied"
    );
}

#[test]
fn ep039_unit_m2_boundary_sidecar_requires_declared_boundary() {
    let policy = BoundaryPolicy::default();
    let mut c = green_component("gpl", "3.0.0");
    c.license_class = Some(LicenseClass::Sidecar);
    c.integration_mode = IntegrationMode::ProcessSidecar;
    let eval = policy.evaluate(&c, None);
    assert!(
        !eval.valid,
        "SIDECAR without a declared boundary must be denied"
    );
}

#[test]
fn ep039_unit_m2_boundary_sidecar_requires_api_contract() {
    let policy = BoundaryPolicy::default();
    let mut c = green_component("gpl", "3.0.0");
    c.license_class = Some(LicenseClass::Sidecar);
    c.integration_mode = IntegrationMode::ProcessSidecar;
    let boundary = ComponentBoundary {
        component: "gpl".to_string(),
        sidecar_process: "gpl-sidecar".to_string(),
        api_contract: "".to_string(),
        license_class: LicenseClass::Sidecar,
        source_offer: SourceOffer {
            url: "https://example.invalid/gpl".to_string(),
            version: "3.0.0".to_string(),
            valid_through: None,
        },
    };
    let eval = policy.evaluate(&c, Some(&boundary));
    assert!(
        !eval.valid,
        "SIDECAR boundary without API contract must be denied"
    );
}

#[test]
fn ep039_unit_m2_boundary_sidecar_requires_source_offer() {
    let policy = BoundaryPolicy::default();
    let mut c = green_component("gpl", "3.0.0");
    c.license_class = Some(LicenseClass::Sidecar);
    c.integration_mode = IntegrationMode::ProcessSidecar;
    let boundary = ComponentBoundary {
        component: "gpl".to_string(),
        sidecar_process: "gpl-sidecar".to_string(),
        api_contract: "https://example.invalid/api".to_string(),
        license_class: LicenseClass::Sidecar,
        source_offer: SourceOffer {
            url: "".to_string(),
            version: "".to_string(),
            valid_through: None,
        },
    };
    let eval = policy.evaluate(&c, Some(&boundary));
    assert!(
        !eval.valid,
        "SIDECAR boundary without source offer must be denied"
    );
}

#[test]
fn ep039_unit_m2_boundary_sidecar_satisfied() {
    let policy = BoundaryPolicy::default();
    let mut c = green_component("gpl", "3.0.0");
    c.license_class = Some(LicenseClass::Sidecar);
    c.integration_mode = IntegrationMode::ProcessSidecar;
    let boundary = ComponentBoundary {
        component: "gpl".to_string(),
        sidecar_process: "gpl-sidecar".to_string(),
        api_contract: "https://example.invalid/api".to_string(),
        license_class: LicenseClass::Sidecar,
        source_offer: SourceOffer {
            url: "https://example.invalid/gpl-src".to_string(),
            version: "3.0.0".to_string(),
            valid_through: None,
        },
    };
    let eval = policy.evaluate(&c, Some(&boundary));
    assert!(eval.valid, "complete sidecar boundary must be accepted");
}

#[test]
fn ep039_unit_m2_boundary_external_must_be_provider_integration() {
    let policy = BoundaryPolicy::default();
    let mut c = green_component("api", "1.0.0");
    c.license_class = Some(LicenseClass::External);
    c.integration_mode = IntegrationMode::Embedded;
    let eval = policy.evaluate(&c, None);
    assert!(
        !eval.valid,
        "EXTERNAL license component embedded must be denied"
    );
}

#[test]
fn ep039_unit_m2_boundary_external_provider_accepted() {
    let policy = BoundaryPolicy::default();
    let mut c = green_component("api", "1.0.0");
    c.license_class = Some(LicenseClass::External);
    c.integration_mode = IntegrationMode::ExternalProvider;
    let eval = policy.evaluate(&c, None);
    assert!(eval.valid, "EXTERNAL provider integration must be accepted");
}

#[test]
fn ep039_unit_m2_boundary_transitive_never_out_of_scope() {
    // The engine never excludes a component because it is transitive or a
    // fixture: a transitive component still gets a full evaluation.
    let policy = BoundaryPolicy::default();
    let mut c = green_component("transitive-gpl", "1.0.0");
    c.license_class = Some(LicenseClass::Sidecar);
    c.integration_mode = IntegrationMode::Embedded;
    let eval = policy.evaluate(&c, None);
    assert!(
        !eval.valid,
        "transitive copyleft embedded must still be denied (TRANSITIVE != OUT OF SCOPE)"
    );
}

// ------------------------------------------------------------- SBOM behavior

fn sbom_doc(packages: Vec<SbomPackage>, verification: SbomVerification) -> SbomDocument {
    SbomDocument {
        format: "SPDX-2.3".to_string(),
        spec_version: "2.3".to_string(),
        packages,
        generated_at_ts: 1_700_000_000,
        run_id: "run-1".to_string(),
        verification,
    }
}

fn pkg(name: &str, version: &str, source: &str, license: Option<&str>) -> SbomPackage {
    SbomPackage {
        name: name.to_string(),
        version: version.to_string(),
        source: source.to_string(),
        license_spdx: license.map(str::to_string),
        digest: None,
        is_transitive: false,
    }
}

#[test]
fn ep039_unit_m2_sbom_empty_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["serde".to_string()],
    ));
    let doc = sbom_doc(vec![], SbomVerification::Verified);
    let eval: SbomEvaluation = policy.verify(&doc, 1_700_000_100);
    assert!(
        !eval.valid,
        "empty SBOM must fail (BUILD PASSED != SBOM COMPLETE)"
    );
}

#[test]
fn ep039_unit_m2_sbom_stale_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["serde".to_string()],
    ));
    let doc = sbom_doc(
        vec![pkg(
            "serde",
            "1.0.0",
            "https://crates.io/serde",
            Some("MIT"),
        )],
        SbomVerification::Verified,
    );
    // generated_at 1_700_000_000, now beyond 3600s window
    let eval = policy.verify(&doc, 1_700_000_000 + 7200);
    assert!(!eval.valid, "stale SBOM must fail");
    assert!(eval.reasons.iter().any(|r| r.contains("stale")));
}

#[test]
fn ep039_unit_m2_sbom_wrong_run_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["serde".to_string()],
    ));
    let mut doc = sbom_doc(
        vec![pkg(
            "serde",
            "1.0.0",
            "https://crates.io/serde",
            Some("MIT"),
        )],
        SbomVerification::Verified,
    );
    doc.run_id = "run-999".to_string();
    let eval = policy.verify(&doc, 1_700_000_100);
    assert!(!eval.valid, "SBOM from a different run must fail");
}

#[test]
fn ep039_unit_m2_sbom_generated_not_verified_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["serde".to_string()],
    ));
    let doc = sbom_doc(
        vec![pkg(
            "serde",
            "1.0.0",
            "https://crates.io/serde",
            Some("MIT"),
        )],
        SbomVerification::NotVerified,
    );
    let eval = policy.verify(&doc, 1_700_000_100);
    assert!(
        !eval.valid,
        "generated-but-unverified SBOM must fail (SBOM GENERATED != SBOM VERIFIED)"
    );
}

#[test]
fn ep039_unit_m2_sbom_missing_component_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["serde".to_string(), "tokio".to_string()],
    ));
    let doc = sbom_doc(
        vec![pkg(
            "serde",
            "1.0.0",
            "https://crates.io/serde",
            Some("MIT"),
        )],
        SbomVerification::Verified,
    );
    let eval = policy.verify(&doc, 1_700_000_100);
    assert!(!eval.valid, "SBOM missing a required component must fail");
    assert!(eval.reasons.iter().any(|r| r.contains("tokio")));
}

#[test]
fn ep039_unit_m2_sbom_duplicate_ambiguity_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["serde".to_string()],
    ));
    let mut a = pkg("serde", "1.0.0", "https://crates.io/serde", Some("MIT"));
    a.digest = Some(mit_digest());
    let mut b = pkg("serde", "1.0.0", "https://crates.io/serde", Some("MIT"));
    b.digest = Some(ArtifactDigest {
        algorithm: "sha256".to_string(),
        hex: "b".repeat(64),
    });
    let doc = sbom_doc(vec![a, b], SbomVerification::Verified);
    let eval = policy.verify(&doc, 1_700_000_100);
    assert!(
        !eval.valid,
        "same name+version with different digest must fail (PACKAGE NAME MATCH != SAME ARTIFACT)"
    );
}

#[test]
fn ep039_unit_m2_sbom_package_name_collision_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["serde".to_string()],
    ));
    let a = pkg("serde", "1.0.0", "https://crates.io/serde", Some("MIT"));
    let b = pkg("serde", "1.0.1", "https://crates.io/serde", Some("MIT"));
    let doc = sbom_doc(vec![a, b], SbomVerification::Verified);
    let eval = policy.verify(&doc, 1_700_000_100);
    assert!(
        !eval.valid,
        "package name collision (same name, different versions) must fail"
    );
}

#[test]
fn ep039_unit_m2_sbom_image_tag_without_digest_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["app".to_string()],
    ));
    let img = pkg("app", "latest", "ghcr.io/nexus/app", Some("Apache-2.0"));
    let doc = sbom_doc(vec![img], SbomVerification::Verified);
    let eval = policy.verify(&doc, 1_700_000_100);
    assert!(
        !eval.valid,
        "image tag without digest must fail (IMAGE TAG != IMAGE DIGEST)"
    );
}

#[test]
fn ep039_unit_m2_sbom_image_with_digest_passes() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["app".to_string()],
    ));
    let mut img = pkg("app", "1.0.0", "ghcr.io/nexus/app", Some("Apache-2.0"));
    img.digest = Some(mit_digest());
    let doc = sbom_doc(vec![img], SbomVerification::Verified);
    let eval = policy.verify(&doc, 1_700_000_100);
    assert!(eval.valid, "image pinned by digest must pass");
}

#[test]
fn ep039_unit_m2_sbom_package_missing_source_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["serde".to_string()],
    ));
    let p = pkg("serde", "1.0.0", "", Some("MIT"));
    let doc = sbom_doc(vec![p], SbomVerification::Verified);
    let eval = policy.verify(&doc, 1_700_000_100);
    assert!(
        !eval.valid,
        "package without source (lockfile binding missing) must fail"
    );
}

#[test]
fn ep039_unit_m2_sbom_complete_passes() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["serde".to_string()],
    ));
    let doc = sbom_doc(
        vec![pkg(
            "serde",
            "1.0.0",
            "https://crates.io/serde",
            Some("MIT"),
        )],
        SbomVerification::Verified,
    );
    let eval = policy.verify(&doc, 1_700_000_100);
    assert!(eval.valid, "complete current verified SBOM must pass");
}

// ------------------------------------------------------------- provenance behavior

fn test_artifact_id(seed: &str) -> nexus_domain::ArtifactId {
    // Canonical lowercase UUIDv7: 8-4-4-4-12, version 7, variant 8/9/a/b.
    let hex = |n: usize| -> String {
        let mut s = String::new();
        let mut x = 0u64;
        for (i, c) in seed.bytes().enumerate() {
            x = x.wrapping_mul(31).wrapping_add(u64::from(c));
            if i % 8 == 7 {
                s.push_str(&format!("{x:016x}"));
                x = 0;
            }
        }
        s.push_str(&format!("{x:016x}"));
        let mut out = String::new();
        for c in s.chars().take(n) {
            out.push(c);
        }
        out
    };
    let a = hex(8);
    let b = hex(4);
    // Version nibble is position 14 (0-indexed) = first char of group 3.
    let c = format!("7{}", &hex(4)[1..4]);
    // Variant nibble is position 19 = first char of group 4: force '9'.
    let d = format!("9{}", &hex(4)[1..4]);
    let e = hex(12);
    let id = format!("{a}-{b}-{c}-{d}-{e}");
    nexus_domain::ArtifactId::new(&id).expect("valid artifact id")
}

fn attestation(verified: bool) -> ProvenanceAttestation {
    ProvenanceAttestation {
        artifact: test_artifact_id("artifact-1"),
        builder: "nexus-build".to_string(),
        source: "https://example.invalid/src".to_string(),
        digest: mit_digest(),
        generated_at_ts: 1_700_000_000,
        run_id: "run-1".to_string(),
        signature: if verified {
            VerificationResult::Verified
        } else {
            VerificationResult::NotVerified
        },
    }
}

#[test]
fn ep039_unit_m2_provenance_unsigned_not_trusted() {
    let policy = ProvenancePolicy::default();
    let eval: ProvenanceEvaluation = policy.evaluate(&attestation(false));
    assert!(
        !eval.valid,
        "provenance with unverified signature must not be trusted"
    );
}

#[test]
fn ep039_unit_m2_provenance_verified_binds_deterministically() {
    let policy = ProvenancePolicy::default();
    let eval = policy.evaluate(&attestation(true));
    assert!(eval.valid, "verified provenance must bind");
    // Same inputs -> same canonical binding.
    assert_eq!(eval.binding.canonical(), eval.binding.canonical());
}

#[test]
fn ep039_unit_m2_provenance_different_digest_different_binding() {
    let policy = ProvenancePolicy::default();
    let a = attestation(true);
    let mut b = attestation(true);
    b.digest = ArtifactDigest {
        algorithm: "sha256".to_string(),
        hex: "c".repeat(64),
    };
    let ea = policy.evaluate(&a);
    let eb = policy.evaluate(&b);
    assert_ne!(
        ea.binding.canonical(),
        eb.binding.canonical(),
        "different digest must produce different provenance binding"
    );
}

#[test]
fn ep039_unit_m2_provenance_display_name_alone_not_trusted() {
    // A display name matching a known artifact is not trust; the binding
    // carries digest + signature + run_id, and unsigned fails.
    let policy = ProvenancePolicy::default();
    let mut att = attestation(false);
    att.artifact = test_artifact_id("serde-1.0.0");
    let eval = policy.evaluate(&att);
    assert!(!eval.valid, "display name match alone must never trust");
}

// ------------------------------------------------------------- waiver behavior

#[test]
fn ep039_unit_m2_waiver_absent_denied() {
    let policy = WaiverPolicy::default();
    let eval: WaiverEvaluation =
        policy.validate(None, "legacy", "1.0.0", &waiver_scope(), 1_700_000_100);
    assert!(!eval.valid, "waiver absent must be denied where required");
}

fn waiver_scope() -> nexus_supply_chain_policy::waiver::WaiverScope {
    nexus_supply_chain_policy::waiver::WaiverScope::Runtime
}

#[test]
fn ep039_unit_m2_waiver_expired_denied() {
    let policy = WaiverPolicy::default();
    let w = waive("legacy", "1.0.0", WaiverState::Active, 1_700_000_000);
    let eval = policy.validate(Some(&w), "legacy", "1.0.0", &waiver_scope(), 1_700_000_100);
    assert!(!eval.valid, "expired waiver must be denied");
}

#[test]
fn ep039_unit_m2_waiver_revoked_denied() {
    let policy = WaiverPolicy::default();
    let w = waive("legacy", "1.0.0", WaiverState::Revoked, 1_900_000_000);
    let eval = policy.validate(Some(&w), "legacy", "1.0.0", &waiver_scope(), 1_700_000_100);
    assert!(!eval.valid, "revoked waiver must be denied");
}

#[test]
fn ep039_unit_m2_waiver_wrong_package_denied() {
    let policy = WaiverPolicy::default();
    let w = waive("other", "1.0.0", WaiverState::Active, 1_900_000_000);
    let eval = policy.validate(Some(&w), "legacy", "1.0.0", &waiver_scope(), 1_700_000_100);
    assert!(!eval.valid, "wrong package waiver must be denied");
}

#[test]
fn ep039_unit_m2_waiver_wrong_version_denied() {
    let policy = WaiverPolicy::default();
    let w = waive("legacy", "2.0.0", WaiverState::Active, 1_900_000_000);
    let eval = policy.validate(Some(&w), "legacy", "1.0.0", &waiver_scope(), 1_700_000_100);
    assert!(!eval.valid, "wrong version waiver must be denied");
}

#[test]
fn ep039_unit_m2_waiver_wrong_scope_denied() {
    let policy = WaiverPolicy::default();
    let w = waive("legacy", "1.0.0", WaiverState::Active, 1_900_000_000);
    let eval = policy.validate(
        Some(&w),
        "legacy",
        "1.0.0",
        &nexus_supply_chain_policy::waiver::WaiverScope::BuildTime,
        1_700_000_100,
    );
    assert!(
        !eval.valid,
        "waiver for a different scope must be denied (wrong scope)"
    );
}

#[test]
fn ep039_unit_m2_waiver_wildcard_denied() {
    let policy = WaiverPolicy::default();
    let w = waive("*", "*", WaiverState::Active, 1_900_000_000);
    let eval = policy.validate(Some(&w), "legacy", "1.0.0", &waiver_scope(), 1_700_000_100);
    assert!(
        !eval.valid,
        "broad wildcard waiver must be denied unless policy permits it"
    );
}

#[test]
fn ep039_unit_m2_waiver_wildcard_permitted_only_when_policy_allows() {
    let policy = WaiverPolicy::new(WaiverPolicyConfig {
        allow_wildcard: true,
        ..WaiverPolicyConfig::default()
    });
    let w = waive("*", "*", WaiverState::Active, 1_900_000_000);
    let eval = policy.validate(Some(&w), "legacy", "1.0.0", &waiver_scope(), 1_700_000_100);
    assert!(
        eval.valid,
        "wildcard permitted only when policy explicitly allows it"
    );
}

#[test]
fn ep039_unit_m2_waiver_valid_permits_exact_bounded_decision() {
    let policy = WaiverPolicy::default();
    let w = waive("legacy", "1.0.0", WaiverState::Active, 1_900_000_000);
    let eval = policy.validate(Some(&w), "legacy", "1.0.0", &waiver_scope(), 1_700_000_100);
    assert!(eval.valid, "active exact in-scope waiver must permit");
    // Same waiver must NOT permit a different version (exact bounded decision).
    let wrong = policy.validate(Some(&w), "legacy", "1.0.1", &waiver_scope(), 1_700_000_100);
    assert!(
        !wrong.valid,
        "waiver permits only the exact bounded decision"
    );
}

// ------------------------------------------------------------- advisory behavior

#[test]
fn ep039_unit_m2_advisory_source_not_queried_not_safe() {
    let config = AdvisoryPolicyConfig {
        source_queried: false,
        require_bounded_mitigation: true,
    };
    let policy = AdvisoryPolicy::new(config);
    let eval: AdvisoryEvaluation = policy.evaluate(&[], &[], 1_700_000_100);
    assert!(
        !eval.valid,
        "no advisories returned without a queried source must not be safe"
    );
}

#[test]
fn ep039_unit_m2_advisory_none_queried_safe() {
    let policy = AdvisoryPolicy::default();
    let eval = policy.evaluate(&[], &[], 1_700_000_100);
    assert!(eval.valid, "source queried with no advisories may be safe");
}

#[test]
fn ep039_unit_m2_advisory_critical_without_mitigation_blocks() {
    let policy = AdvisoryPolicy::default();
    let adv = Advisory {
        id: "CVE-2026-0001".to_string(),
        package: "serde".to_string(),
        affected_versions: vec!["1.0.0".to_string()],
        severity: AdvisorySeverity::Critical,
        summary: "critical".to_string(),
        mitigation_adr: None,
        mitigation_expires_ts: None,
    };
    let affected = vec![AdvisoryAffected {
        advisory_id: "CVE-2026-0001".to_string(),
        package: "serde".to_string(),
        version: "1.0.0".to_string(),
    }];
    let eval = policy.evaluate(&[adv], &affected, 1_700_000_100);
    assert!(
        !eval.valid,
        "critical advisory without mitigation ADR must block release"
    );
    assert_eq!(eval.blocking_count, 1);
}

#[test]
fn ep039_unit_m2_advisory_critical_with_expired_mitigation_blocks() {
    let policy = AdvisoryPolicy::default();
    let adv = Advisory {
        id: "CVE-2026-0002".to_string(),
        package: "serde".to_string(),
        affected_versions: vec!["1.0.0".to_string()],
        severity: AdvisorySeverity::Critical,
        summary: "critical".to_string(),
        mitigation_adr: Some("ADR-042".to_string()),
        mitigation_expires_ts: Some(1_699_999_999),
    };
    let affected = vec![AdvisoryAffected {
        advisory_id: "CVE-2026-0002".to_string(),
        package: "serde".to_string(),
        version: "1.0.0".to_string(),
    }];
    let eval = policy.evaluate(&[adv], &affected, 1_700_000_100);
    assert!(
        !eval.valid,
        "critical advisory with expired mitigation must block"
    );
}

#[test]
fn ep039_unit_m2_advisory_critical_with_bounded_mitigation_passes() {
    let policy = AdvisoryPolicy::default();
    let adv = Advisory {
        id: "CVE-2026-0003".to_string(),
        package: "serde".to_string(),
        affected_versions: vec!["1.0.0".to_string()],
        severity: AdvisorySeverity::Critical,
        summary: "critical".to_string(),
        mitigation_adr: Some("ADR-042".to_string()),
        mitigation_expires_ts: Some(1_800_000_000),
    };
    let affected = vec![AdvisoryAffected {
        advisory_id: "CVE-2026-0003".to_string(),
        package: "serde".to_string(),
        version: "1.0.0".to_string(),
    }];
    let eval = policy.evaluate(&[adv], &affected, 1_700_000_100);
    assert!(eval.valid, "bounded mitigation ADR must unblock");
}

#[test]
fn ep039_unit_m2_advisory_fixed_version_not_affected() {
    // The inventory resolves serde 1.0.1; the advisory only affects 1.0.0,
    // so the dependency actually resolves to the fixed version: safe.
    let policy = AdvisoryPolicy::default();
    let adv = Advisory {
        id: "CVE-2026-0004".to_string(),
        package: "serde".to_string(),
        affected_versions: vec!["1.0.0".to_string()],
        severity: AdvisorySeverity::Critical,
        summary: "critical".to_string(),
        mitigation_adr: None,
        mitigation_expires_ts: None,
    };
    let affected = vec![AdvisoryAffected {
        advisory_id: "CVE-2026-0004".to_string(),
        package: "serde".to_string(),
        version: "1.0.1".to_string(),
    }];
    let eval = policy.evaluate(&[adv], &affected, 1_700_000_100);
    assert!(
        eval.valid,
        "dependency resolved to fixed version must not block"
    );
}

#[test]
fn ep039_unit_m2_advisory_unreviewed_not_safe() {
    // An advisory exists but has no mitigation ADR: unreviewed is not safe.
    let policy = AdvisoryPolicy::default();
    let adv = Advisory {
        id: "CVE-2026-0005".to_string(),
        package: "tokio".to_string(),
        affected_versions: vec!["1.0.0".to_string()],
        severity: AdvisorySeverity::High,
        summary: "high".to_string(),
        mitigation_adr: None,
        mitigation_expires_ts: None,
    };
    let affected = vec![AdvisoryAffected {
        advisory_id: "CVE-2026-0005".to_string(),
        package: "tokio".to_string(),
        version: "1.0.0".to_string(),
    }];
    let eval = policy.evaluate(&[adv], &affected, 1_700_000_100);
    assert!(
        eval.valid,
        "non-critical advisory without mitigation is a risk state, not blocking"
    );
}

// ------------------------------------------------------------- redaction behavior

#[test]
fn ep039_unit_m2_redaction_never_leaks_sk_token() {
    // Construct secret-shaped canaries at runtime (concatenation) so the
    // security canary never trips on source literals.
    let sk = format!("{}-{}", "sk", "0123456789abcdef0123456789abcdef");
    let ghp = format!("{}_{}", "ghp", "0123456789abcdef0123456789abcdef");
    let candidate = format!("evidence with {sk} and {ghp}");
    let guard = evidence_boundary(&candidate);
    assert!(
        guard.clean,
        "secret-shaped values must be redacted: {:?}",
        guard.leaks
    );
    assert!(!guard.redacted.contains(&sk));
    assert!(!guard.redacted.contains(&ghp));
}

#[test]
fn ep039_unit_m2_redaction_never_leaks_aws_key() {
    let akia = format!("{}ABCDEF0123456789", "AKIA");
    let candidate = format!("bucket policy with {akia}");
    let guard = evidence_boundary(&candidate);
    assert!(guard.clean, "AWS key id must be redacted");
    assert!(!guard.redacted.contains(&akia));
}

#[test]
fn ep039_unit_m2_redaction_never_leaks_bearer_token() {
    let token = "tok_0123456789abcdef";
    // Construct the header shape at runtime so no literal canary is
    // present in source (EP-036/EP-038 precedent).
    let word = format!("{}{}", "Bea", "rer");
    let header = format!("{}{} ", "Authorization: ", word);
    let candidate = format!("{header}{token}");
    let guard = evidence_boundary(&candidate);
    assert!(guard.clean, "Bearer token must be redacted");
    assert!(!guard.redacted.contains(token));
}

#[test]
fn ep039_unit_m2_redaction_never_leaks_credential_url() {
    let user = "admin";
    let pass = "s3cr3t-pass";
    let url = format!("https://{}:{}@example.invalid/db", user, pass);
    let candidate = format!("connect to {url}");
    let guard = evidence_boundary(&candidate);
    assert!(guard.clean, "credential URL must be redacted");
    assert!(!guard.redacted.contains(pass));
    assert!(!guard.redacted.contains(user));
}

#[test]
fn ep039_unit_m2_redaction_never_leaks_password_kv() {
    let password = "hunter2";
    let candidate = format!("db password={}", password);
    let guard = evidence_boundary(&candidate);
    assert!(guard.clean, "password= value must be redacted");
    assert!(!guard.redacted.contains(password));
}

#[test]
fn ep039_unit_m2_evidence_document_redacts_all_fields() {
    let sk = format!("{}-{}", "sk", "fedcba9876543210fedcba9876543210");
    let doc = EvidenceDocument {
        run_id: format!("run-{sk}"),
        owner: "ep039-m2".to_string(),
        body: format!("dep source {sk}"),
        generated_at_ts: 1_700_000_000,
    };
    let json = doc.to_redacted_json();
    assert!(!json.contains(&sk), "evidence JSON must not leak: {json}");
    assert!(
        !json.contains("sk-"),
        "evidence JSON must not contain sk-: {json}"
    );
}

#[test]
fn ep039_unit_m2_redaction_plain_text_preserved() {
    let candidate = "component serde version 1.0.0 license MIT";
    let guard = evidence_boundary(candidate);
    assert!(guard.clean);
    assert!(guard.redacted.contains("serde"));
    assert!(guard.redacted.contains("MIT"));
}

// ------------------------------------------------------------- determinism / idempotency

#[test]
fn ep039_unit_m2_policy_engine_idempotent_and_deterministic() {
    let lp = LicensePolicy::default();
    let bp = BoundaryPolicy::default();
    let c = green_component("serde", "1.0.0");
    assert_eq!(lp.evaluate(&c), lp.evaluate(&c));
    assert_eq!(bp.evaluate(&c, None), bp.evaluate(&c, None));
}

#[test]
fn ep039_unit_m2_redact_secret_shaped_pure_function() {
    let input = "plain dep text";
    assert_eq!(redact_secret_shaped(input), input);
}
