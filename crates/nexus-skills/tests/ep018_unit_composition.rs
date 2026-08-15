//! EP-018 M1 composition + dependency-direction suite (SPEC-010
//! behaviors 6-8; ADR-025).
//!
//! Proves the permission-composition semantics (declared requirements
//! union over the closure, effective authority = intersection of caller
//! grants, policy allowance, and trust ceiling), deterministic cycle-
//! free dependency resolution with a bounded depth, deterministic
//! evaluation, and the dependency-direction constraint (the contract
//! crate imports no provider/runtime implementation crates).

use nexus_domain::{ArtifactId, SkillId, TenantId};
use nexus_skills::{
    DeterministicSkillComposer, DeterministicSkillEvaluator, PermissionAuthority, SkillComposer,
    SkillCompositionErrorCode, SkillEvaluator, SkillManifest, SkillPackage, SkillPermission,
    SkillSignature, SkillTrustLevel, MAX_COMPOSITION_DEPTH,
};
use std::collections::HashSet;
use std::str::FromStr;

fn sid() -> SkillId {
    SkillId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6071").expect("valid UUIDv7")
}

fn tid() -> TenantId {
    TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072").expect("valid UUIDv7")
}

fn aid() -> ArtifactId {
    ArtifactId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073").expect("valid UUIDv7")
}

fn signature() -> SkillSignature {
    SkillSignature {
        algorithm: nexus_skills::SignatureAlgorithm::Ed25519,
        public_key_hex: "ab".repeat(32),
        signature_hex: "cd".repeat(64),
        signer: Some("human-owner".into()),
    }
}

fn package(
    name: &str,
    version: &str,
    permissions: Vec<SkillPermission>,
    dependencies: Vec<String>,
) -> SkillPackage {
    let manifest = SkillManifest {
        skill_id: sid(),
        tenant_id: tid(),
        name: name.into(),
        version: version.into(),
        description: "composition fixture".into(),
        permissions,
        dependencies,
        network_rules: vec![],
        license: "MIT".into(),
        provenance: aid(),
        trust_level: SkillTrustLevel::Sandboxed,
        signature: signature(),
    };
    SkillPackage {
        manifest,
        content_hash: format!("{:064x}", name.len()),
        created_at_epoch_ms: 1,
    }
}

fn authority(
    caller_granted: Vec<SkillPermission>,
    policy_allowed: Vec<SkillPermission>,
    trust_ceiling: SkillPermission,
) -> PermissionAuthority {
    PermissionAuthority {
        caller_granted,
        policy_allowed,
        trust_ceiling,
    }
}

fn sorted(permissions: &[SkillPermission]) -> Vec<SkillPermission> {
    let mut out = permissions.to_vec();
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// PERMISSION COMPOSITION SEMANTICS (directive A)
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_composition_declared_requirements_union_across_closure() {
    // child A requires READ; child B requires WRITE. The composed
    // declared requirements may contain READ + WRITE.
    let a = package("nexus/a", "1.0.0", vec![SkillPermission::Read], vec![]);
    let b = package(
        "nexus/b",
        "1.0.0",
        vec![SkillPermission::Write],
        vec!["nexus/a".into()],
    );
    let composer = DeterministicSkillComposer;
    let composition = composer
        .compose(&b, &[a, b.clone()])
        .expect("composition ok");
    assert_eq!(
        sorted(&composition.declared_required_permissions),
        vec![SkillPermission::Read, SkillPermission::Write]
    );
}

#[test]
fn ep018_unit_composition_effective_authority_caller_grant_read_only() {
    // caller grants READ only -> effective composed authority is READ
    // only, even though the closure declared READ + WRITE.
    let a = package("nexus/a", "1.0.0", vec![SkillPermission::Read], vec![]);
    let b = package(
        "nexus/b",
        "1.0.0",
        vec![SkillPermission::Write],
        vec!["nexus/a".into()],
    );
    let composer = DeterministicSkillComposer;
    let authority = authority(
        vec![SkillPermission::Read],
        vec![SkillPermission::Read, SkillPermission::Write],
        SkillTrustLevel::Trusted.permission_ceiling(),
    );
    let composition = composer
        .compose_with_authority(&b, &[a, b.clone()], &authority)
        .expect("composition ok");
    assert_eq!(
        composition.effective_permissions,
        vec![SkillPermission::Read]
    );
}

#[test]
fn ep018_unit_composition_no_write_grant_no_write_authority() {
    // No caller WRITE grant: composition cannot obtain WRITE.
    let a = package("nexus/a", "1.0.0", vec![SkillPermission::Write], vec![]);
    let composer = DeterministicSkillComposer;
    let authority = authority(
        vec![SkillPermission::Read],
        vec![SkillPermission::Read, SkillPermission::Write],
        SkillTrustLevel::Trusted.permission_ceiling(),
    );
    let composition = composer
        .compose_with_authority(&a, std::slice::from_ref(&a), &authority)
        .expect("composition ok");
    assert!(!composition
        .effective_permissions
        .contains(&SkillPermission::Write));
}

#[test]
fn ep018_unit_composition_community_ceiling_forbids_privileged_permission() {
    // Untrusted/community trust ceiling forbids privileged permission
    // even if the manifest requests it.
    let a = package("nexus/a", "1.0.0", vec![SkillPermission::Execute], vec![]);
    let composer = DeterministicSkillComposer;
    // The caller granted EXECUTE and policy allows it, but the trust
    // ceiling (sandboxed = READ) still denies it.
    let authority = authority(
        vec![SkillPermission::Execute],
        vec![SkillPermission::Execute],
        SkillTrustLevel::Sandboxed.permission_ceiling(),
    );
    let composition = composer
        .compose_with_authority(&a, std::slice::from_ref(&a), &authority)
        .expect("composition ok");
    assert!(composition.effective_permissions.is_empty());
    assert!(!composition
        .effective_permissions
        .contains(&SkillPermission::Execute));
}

#[test]
fn ep018_unit_composition_nested_never_exceeds_root_authority_envelope() {
    // A -> B -> C; C requests SECRETS, root caller grants READ only.
    // Effective authority is bounded by the root envelope: READ.
    let c = package("nexus/c", "1.0.0", vec![SkillPermission::Secrets], vec![]);
    let b = package(
        "nexus/b",
        "1.0.0",
        vec![SkillPermission::Read],
        vec!["nexus/c".into()],
    );
    let a = package(
        "nexus/a",
        "1.0.0",
        vec![SkillPermission::Read],
        vec!["nexus/b".into()],
    );
    let composer = DeterministicSkillComposer;
    let authority = authority(
        vec![SkillPermission::Read],
        vec![SkillPermission::Read, SkillPermission::Secrets],
        SkillTrustLevel::System.permission_ceiling(),
    );
    let composition = composer
        .compose_with_authority(&a, &[a.clone(), b, c], &authority)
        .expect("composition ok");
    assert_eq!(
        composition.effective_permissions,
        vec![SkillPermission::Read]
    );
    assert!(!composition
        .effective_permissions
        .contains(&SkillPermission::Secrets));
    // Declared requirements still report the full closure union.
    assert!(composition
        .declared_required_permissions
        .contains(&SkillPermission::Secrets));
}

#[test]
fn ep018_unit_composition_manifest_declaration_is_not_authorization() {
    // A manifest declaring EXECUTE produces NO effective authority
    // unless the caller granted it, policy allows it, and the ceiling
    // admits it. Declaration alone never authorizes.
    let a = package("nexus/a", "1.0.0", vec![SkillPermission::Execute], vec![]);
    let composer = DeterministicSkillComposer;
    let authority = authority(
        vec![],
        vec![SkillPermission::Execute],
        SkillTrustLevel::Trusted.permission_ceiling(),
    );
    let composition = composer
        .compose_with_authority(&a, std::slice::from_ref(&a), &authority)
        .expect("composition ok");
    assert!(composition.effective_permissions.is_empty());
}

// ---------------------------------------------------------------------------
// DEPENDENCY GRAPH (directive G)
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_composition_direct_self_cycle_rejected() {
    let a = package("nexus/a", "1.0.0", vec![], vec!["nexus/a".into()]);
    // Manifest validation rejects self-dependency before composition.
    assert!(a.validate().is_err());
}

#[test]
fn ep018_unit_composition_cycle_rejected() {
    let a = package("nexus/a", "1.0.0", vec![], vec!["nexus/b".into()]);
    let b = package("nexus/b", "1.0.0", vec![], vec!["nexus/a".into()]);
    let composer = DeterministicSkillComposer;
    let err = composer
        .compose(&a, &[a.clone(), b])
        .expect_err("cycle rejected");
    assert_eq!(err.code, SkillCompositionErrorCode::Cycle);
}

#[test]
fn ep018_unit_composition_transitive_cycle_rejected() {
    let a = package("nexus/a", "1.0.0", vec![], vec!["nexus/b".into()]);
    let b = package("nexus/b", "1.0.0", vec![], vec!["nexus/c".into()]);
    let c = package("nexus/c", "1.0.0", vec![], vec!["nexus/a".into()]);
    let composer = DeterministicSkillComposer;
    let err = composer
        .compose(&a, &[a.clone(), b, c])
        .expect_err("transitive cycle rejected");
    assert_eq!(err.code, SkillCompositionErrorCode::Cycle);
}

#[test]
fn ep018_unit_composition_missing_dependency_rejected() {
    let a = package("nexus/a", "1.0.0", vec![], vec!["nexus/ghost".into()]);
    let composer = DeterministicSkillComposer;
    let err = composer
        .compose(&a, std::slice::from_ref(&a))
        .expect_err("missing dependency rejected");
    assert_eq!(err.code, SkillCompositionErrorCode::NotFound);
}

#[test]
fn ep018_unit_composition_traversal_is_deterministic_across_input_order() {
    let a = package("nexus/a", "1.0.0", vec![SkillPermission::Read], vec![]);
    let b = package(
        "nexus/b",
        "1.0.0",
        vec![SkillPermission::Write],
        vec!["nexus/a".into()],
    );
    let c = package(
        "nexus/c",
        "1.0.0",
        vec![SkillPermission::Read],
        vec!["nexus/b".into()],
    );
    let composer = DeterministicSkillComposer;
    let first = composer
        .compose(&c, &[c.clone(), b.clone(), a.clone()])
        .expect("composition ok");
    let second = composer
        .compose(&c, &[b, a, c.clone()])
        .expect("composition ok");
    assert_eq!(first.versions, second.versions);
    assert_eq!(
        first.declared_required_permissions,
        second.declared_required_permissions
    );
    // Post-order: dependencies before dependents.
    let pos_a = first
        .versions
        .iter()
        .position(|v| v == "nexus/a@1.0.0")
        .unwrap();
    let pos_b = first
        .versions
        .iter()
        .position(|v| v == "nexus/b@1.0.0")
        .unwrap();
    let pos_c = first
        .versions
        .iter()
        .position(|v| v == "nexus/c@1.0.0")
        .unwrap();
    assert!(pos_a < pos_b && pos_b < pos_c);
}

#[test]
fn ep018_unit_composition_lowest_version_canonical_resolution() {
    // When multiple versions of a dependency exist, the deterministic
    // policy resolves the lowest version.
    let a_old = package("nexus/a", "1.0.0", vec![SkillPermission::Read], vec![]);
    let a_new = package("nexus/a", "2.0.0", vec![SkillPermission::Write], vec![]);
    let b = package("nexus/b", "1.0.0", vec![], vec!["nexus/a".into()]);
    let composer = DeterministicSkillComposer;
    let composition = composer
        .compose(&b, &[a_new, b.clone(), a_old])
        .expect("composition ok");
    assert!(composition.versions.contains(&"nexus/a@1.0.0".to_string()));
    assert!(!composition.versions.contains(&"nexus/a@2.0.0".to_string()));
    assert!(composition
        .declared_required_permissions
        .contains(&SkillPermission::Read));
}

#[test]
fn ep018_unit_composition_depth_is_bounded() {
    // A chain longer than MAX_COMPOSITION_DEPTH is rejected: infinite
    // recursive skill loading is impossible by contract.
    let mut available = Vec::new();
    for i in 0..(MAX_COMPOSITION_DEPTH + 2) {
        let name = format!("nexus/d{i}");
        let dependency = if i + 1 < MAX_COMPOSITION_DEPTH + 2 {
            vec![format!("nexus/d{}", i + 1)]
        } else {
            vec![]
        };
        available.push(package(&name, "1.0.0", vec![], dependency));
    }
    let composer = DeterministicSkillComposer;
    let root = available[0].clone();
    let err = composer
        .compose(&root, &available)
        .expect_err("depth bounded");
    assert_eq!(err.code, SkillCompositionErrorCode::Depth);
}

#[test]
fn ep018_unit_composition_duplicate_dependency_rejected_by_manifest() {
    let mut manifest = SkillManifest {
        skill_id: sid(),
        tenant_id: tid(),
        name: "nexus/a".into(),
        version: "1.0.0".into(),
        description: "dup dep".into(),
        permissions: vec![],
        dependencies: vec!["nexus/b".into(), "nexus/b".into()],
        network_rules: vec![],
        license: "MIT".into(),
        provenance: aid(),
        trust_level: SkillTrustLevel::Sandboxed,
        signature: signature(),
    };
    assert!(manifest.validate().is_err());
    manifest.dependencies = vec!["nexus/b".into()];
    assert!(manifest.validate().is_ok());
}

// ---------------------------------------------------------------------------
// EVALUATOR (directive I)
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_evaluator_same_corpus_same_package_same_evaluation() {
    let corpus = vec!["eval-1".to_string(), "eval-2".to_string()];
    let package = package("nexus/a", "1.0.0", vec![], vec![]);
    let evaluator_a = DeterministicSkillEvaluator::new(corpus.clone());
    let evaluator_b = DeterministicSkillEvaluator::new(corpus);
    let ea = evaluator_a.evaluate(&package).expect("evaluate");
    let eb = evaluator_b.evaluate(&package).expect("evaluate");
    assert_eq!(ea, eb);
    assert!(ea.passed);
}

#[test]
fn ep018_unit_evaluator_empty_corpus_fails_closed_no_fabricated_score() {
    let evaluator = DeterministicSkillEvaluator::new(vec![]);
    let err = evaluator
        .evaluate(&package("nexus/a", "1.0.0", vec![], vec![]))
        .expect_err("empty corpus fails closed");
    assert_eq!(err.code, nexus_skills::SkillPackageErrorCode::Verification);
}

#[test]
fn ep018_unit_evaluator_unknown_version_fails_closed() {
    let evaluator = DeterministicSkillEvaluator::with_version(vec!["eval-1".into()], "2.0");
    let err = evaluator
        .evaluate(&package("nexus/a", "1.0.0", vec![], vec![]))
        .expect_err("unknown version fails closed");
    assert_eq!(err.code, nexus_skills::SkillPackageErrorCode::Verification);
}

// ---------------------------------------------------------------------------
// DEPENDENCY DIRECTION (directive K)
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_dependency_direction_contract_crate_imports_no_provider_impl() {
    // The M1 contract crate must not import network/runtime/provider
    // implementation crates; dependency direction is enforced by the
    // manifest of the crate itself.
    let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let text = std::fs::read_to_string(manifest_path).expect("read Cargo.toml");
    for forbidden in [
        "nexus-model-gateway",
        "nexus-harness-adapters",
        "nexus-agents",
        "nexus-context",
        "nexus-memory-workers",
        "tokio",
        "reqwest",
        "temporal",
        "postgres",
        "sqlx",
    ] {
        assert!(
            !text.contains(forbidden),
            "contract crate must not depend on provider/runtime crate {forbidden}"
        );
    }
    // The contract crate depends only on the shared domain/fabric
    // crates and serde.
    assert!(text.contains("nexus-domain"));
    assert!(text.contains("serde"));
}

#[test]
fn ep018_unit_dependency_direction_no_import_of_provider_modules_in_source() {
    // Source-level guard: no `use` of runtime/provider crate paths in
    // the contract crate's own modules.
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut files: Vec<_> = std::fs::read_dir(src)
        .expect("src dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    files.sort();
    let mut sources = String::new();
    for path in &files {
        if let Ok(content) = std::fs::read_to_string(path) {
            sources.push_str(&content);
        }
    }
    for forbidden in [
        "nexus_harness_adapters",
        "nexus_model_gateway",
        "nexus_context",
    ] {
        assert!(
            !sources.contains(forbidden),
            "contract crate source must not reference {forbidden}"
        );
    }
}

// ---------------------------------------------------------------------------
// VOCABULARY uniqueness shared guard (kept here to cover the ALL arrays)
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_vocabulary_all_arrays_unique_across_enums() {
    let mut seen: HashSet<&str> = HashSet::new();
    for level in SkillTrustLevel::ALL {
        assert!(seen.insert(level.as_str()));
    }
    for permission in SkillPermission::ALL {
        assert!(seen.insert(permission.as_str()));
    }
}

// ---------------------------------------------------------------------------
// FromStr smoke (composition file mirrors contract file; keep one here)
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_composition_trust_level_fromstr_uses_locked_vocabulary() {
    assert_eq!(
        SkillTrustLevel::from_str("SANDBOXED").expect("parse"),
        SkillTrustLevel::Sandboxed
    );
    assert!(SkillTrustLevel::from_str("TRUSTWORTHY").is_err());
}
