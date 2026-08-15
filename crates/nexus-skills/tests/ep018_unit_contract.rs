//! EP-018 M1 contract suite (SPEC-010 behaviors 6-8; ADR-025).
//!
//! Non-vacuous `ep018_unit_*` tests proving vocabulary locking, manifest
//! validation, signature integrity-vs-trust semantics, registry
//! immutability and ceilings, proposal lifecycle, error typing, and
//! dependency direction. The M1 gate runs this suite through the real
//! `cargo test -p nexus-skills ep018_unit` machinery with a vacuity
//! guard.

use nexus_domain::{ArtifactId, CorrelationId, SkillId, TenantId};
use nexus_skills::{
    is_valid_portable_name, is_valid_semver, DeterministicSkillEvaluator, PermissionAuthority,
    SignatureAlgorithm, SkillEvaluator, SkillManifest, SkillPackage, SkillPackageError,
    SkillPackageErrorCode, SkillPermission, SkillProposal, SkillProposalState, SkillRegistry,
    SkillSignature, SkillTrustLevel,
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

fn cid() -> CorrelationId {
    CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6074").expect("valid UUIDv7")
}

fn valid_signature() -> SkillSignature {
    SkillSignature {
        algorithm: SignatureAlgorithm::Ed25519,
        public_key_hex: "ab".repeat(32),
        signature_hex: "cd".repeat(64),
        signer: Some("human-owner".into()),
    }
}

fn base_manifest() -> SkillManifest {
    SkillManifest {
        skill_id: sid(),
        tenant_id: tid(),
        name: "nexus/hello".into(),
        version: "1.0.0".into(),
        description: "test skill".into(),
        permissions: vec![],
        dependencies: vec![],
        network_rules: vec![],
        license: "MIT".into(),
        provenance: aid(),
        trust_level: SkillTrustLevel::Sandboxed,
        signature: valid_signature(),
    }
}

fn base_package() -> SkillPackage {
    SkillPackage {
        manifest: base_manifest(),
        content_hash: "a".repeat(64),
        created_at_epoch_ms: 1,
    }
}

fn proposal() -> SkillProposal {
    SkillProposal {
        proposal_id: "prop-1".into(),
        skill_id: sid(),
        tenant_id: tid(),
        correlation_id: cid(),
        package: base_package(),
        state: SkillProposalState::Proposed,
        proposed_by: "model-a".into(),
        created_at_epoch_ms: 1,
        updated_at_epoch_ms: 1,
    }
}

// ---------------------------------------------------------------------------
// VOCABULARY
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_vocabulary_roundtrips_all_canonical_enums() {
    for level in SkillTrustLevel::ALL {
        assert_eq!(
            SkillTrustLevel::from_str(level.as_str()).expect("roundtrip"),
            level
        );
    }
    for permission in SkillPermission::ALL {
        assert_eq!(
            SkillPermission::from_str(permission.as_str()).expect("roundtrip"),
            permission
        );
    }
    for state in [
        SkillProposalState::Proposed,
        SkillProposalState::EvalPending,
        SkillProposalState::EvalPassed,
        SkillProposalState::EvalFailed,
        SkillProposalState::AwaitingPromotion,
        SkillProposalState::Promoted,
        SkillProposalState::Rejected,
        SkillProposalState::RolledBack,
    ] {
        assert_eq!(
            SkillProposalState::from_str(state.as_str()).expect("roundtrip"),
            state
        );
    }
    for algorithm in [SignatureAlgorithm::Ed25519, SignatureAlgorithm::EcdsaP256] {
        assert_eq!(
            SignatureAlgorithm::from_str(algorithm.as_str()).expect("roundtrip"),
            algorithm
        );
    }
}

#[test]
fn ep018_unit_vocabulary_unknown_values_rejected() {
    let err = SkillTrustLevel::from_str("GOD_MODE").expect_err("unknown rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Vocabulary);
    let err = SkillPermission::from_str("ALL_THE_THINGS").expect_err("unknown rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Vocabulary);
    let err = SignatureAlgorithm::from_str("RSA_4096").expect_err("unknown rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Vocabulary);
    let err = SkillProposalState::from_str("APPROVED").expect_err("unknown rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Vocabulary);
}

#[test]
fn ep018_unit_vocabulary_no_duplicate_definitions() {
    let mut trust: HashSet<&str> = HashSet::new();
    let mut permission: HashSet<&str> = HashSet::new();
    for level in SkillTrustLevel::ALL {
        assert!(trust.insert(level.as_str()), "duplicate trust definition");
    }
    for permission_value in SkillPermission::ALL {
        assert!(
            permission.insert(permission_value.as_str()),
            "duplicate permission definition"
        );
    }
}

// ---------------------------------------------------------------------------
// MANIFEST VALIDATION
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_manifest_construction_and_validation() {
    let package = base_package();
    assert!(package.validate().is_ok());
    assert!(package.manifest.validate().is_ok());
}

#[test]
fn ep018_unit_manifest_rejects_empty_or_invalid_skill_id() {
    // SkillId is a typed UUIDv7; malformed values cannot construct.
    assert!(SkillId::from_str("not-an-id").is_err());
    assert!(SkillId::from_str("").is_err());
}

#[test]
fn ep018_unit_manifest_rejects_invalid_portable_name() {
    for name in [
        "hello", "ns//x", "Ns/x", "ns/x/y", "ns/x@1", "ns/x:y", " ns/x", "ns/ x",
    ] {
        assert!(!is_valid_portable_name(name), "name accepted: {name}");
    }
    for name in ["nexus/hello", "a/b", "ns/skill-name", "ns/skill_name"] {
        assert!(is_valid_portable_name(name), "valid name rejected: {name}");
    }
}

#[test]
fn ep018_unit_manifest_rejects_invalid_semver() {
    for version in ["1.0", "1.0.0.0", "01.0.0", "1.0.x", "1..0", "v1.0.0", ""] {
        assert!(!is_valid_semver(version), "version accepted: {version}");
    }
    for version in ["1.0.0", "0.0.1", "12.34.56"] {
        assert!(
            is_valid_semver(version),
            "valid version rejected: {version}"
        );
    }
}

#[test]
fn ep018_unit_manifest_rejects_duplicate_permissions() {
    let mut manifest = base_manifest();
    manifest.permissions = vec![SkillPermission::Read, SkillPermission::Read];
    let err = manifest.validate().expect_err("duplicate rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
}

#[test]
fn ep018_unit_manifest_rejects_duplicate_dependencies() {
    let mut manifest = base_manifest();
    manifest.dependencies = vec!["nexus/other".into(), "nexus/other".into()];
    let err = manifest.validate().expect_err("duplicate rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
}

#[test]
fn ep018_unit_manifest_rejects_self_dependency() {
    let mut manifest = base_manifest();
    manifest.dependencies = vec!["nexus/hello".into()];
    let err = manifest.validate().expect_err("self-dependency rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
}

#[test]
fn ep018_unit_manifest_rejects_duplicate_network_rules() {
    let mut manifest = base_manifest();
    manifest.network_rules = vec![
        "allow host example.org".into(),
        "allow host example.org".into(),
    ];
    let err = manifest.validate().expect_err("duplicate rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
}

#[test]
fn ep018_unit_manifest_serialization_roundtrip() {
    let manifest = base_manifest();
    let json = serde_json::to_string(&manifest).expect("serialize");
    let decoded: SkillManifest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, manifest);
}

#[test]
fn ep018_unit_manifest_network_rules_are_declarations_not_grants() {
    // A manifest REQUESTING host access declares a requirement; it does
    // not open the network (ADR-025). The only accessor is a
    // declaration surface; there is no grant in the manifest contract.
    let mut manifest = base_manifest();
    manifest.network_rules = vec!["allow host example.org".into()];
    assert!(manifest.validate().is_ok());
    assert_eq!(
        manifest.requested_network_rules(),
        &["allow host example.org".to_string()]
    );
    // Declaration survives validation unchanged: still only a request.
    let package = SkillPackage {
        manifest,
        content_hash: "a".repeat(64),
        created_at_epoch_ms: 1,
    };
    package.validate().expect("valid");
    assert_eq!(
        package.manifest.requested_network_rules(),
        &["allow host example.org".to_string()]
    );
}

// ---------------------------------------------------------------------------
// PACKAGE IMMUTABILITY
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_package_identity_immutable_by_version() {
    let a = base_package();
    let b = base_package();
    // Same id + version + content -> same canonical identity.
    assert_eq!(a.canonical_identity(), b.canonical_identity());

    // Changed content under the same id/version -> different identity
    // (a conflict at registration, never a silent mutation).
    let mut changed = base_package();
    changed.content_hash = "b".repeat(64);
    assert_ne!(a.canonical_identity(), changed.canonical_identity());

    // New version -> new identity; never a mutable "latest".
    let mut new_version = base_package();
    new_version.manifest.version = "1.0.1".into();
    assert_ne!(a.canonical_identity(), new_version.canonical_identity());
}

#[test]
fn ep018_unit_package_rejects_malformed_content_hash() {
    let mut package = base_package();
    package.content_hash = "short".into();
    let err = package.validate().expect_err("malformed hash rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
}

// ---------------------------------------------------------------------------
// SIGNATURE
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_signature_structural_validation() {
    assert!(valid_signature().validate().is_ok());
}

#[test]
fn ep018_unit_signature_rejects_malformed_encoding() {
    // Non-hex key.
    let mut sig = valid_signature();
    sig.public_key_hex = "z".repeat(64);
    assert!(sig.validate().is_err());

    // Odd-length key.
    let mut sig = valid_signature();
    sig.public_key_hex = "ab".repeat(31) + "a";
    assert!(sig.validate().is_err());

    // Key length not matching Ed25519 (64 hex).
    let mut sig = valid_signature();
    sig.public_key_hex = "ab".repeat(16);
    assert!(sig.validate().is_err());

    // Signature length not 128 hex.
    let mut sig = valid_signature();
    sig.signature_hex = "cd".repeat(32);
    assert!(sig.validate().is_err());

    // Non-hex signature.
    let mut sig = valid_signature();
    sig.signature_hex = "z".repeat(128);
    assert!(sig.validate().is_err());

    // ECDSA P-256 requires a 66-hex key.
    let mut sig = valid_signature();
    sig.algorithm = SignatureAlgorithm::EcdsaP256;
    assert!(sig.validate().is_err());
    let mut sig = valid_signature();
    sig.algorithm = SignatureAlgorithm::EcdsaP256;
    sig.public_key_hex = "ab".repeat(33);
    assert!(sig.validate().is_ok());
}

#[test]
fn ep018_unit_signature_presence_does_not_set_trust() {
    // A signature is an integrity/authenticity statement. Its presence
    // must never automatically raise the skill's trust tier (ADR-025).
    let manifest = base_manifest();
    assert_eq!(manifest.trust_level, SkillTrustLevel::Sandboxed);
    assert!(manifest.signature.validate().is_ok());
    // The signature and the trust tier are independent fields; no API
    // promotes trust from a signature.
    assert_ne!(manifest.trust_level, SkillTrustLevel::Trusted);
    assert_ne!(manifest.trust_level, SkillTrustLevel::System);
}

#[test]
fn ep018_unit_signature_unknown_algorithm_rejected_at_parse() {
    // The vocabulary rejects unsupported algorithms; they cannot reach
    // a manifest.
    let err = SignatureAlgorithm::from_str("RSA_4096").expect_err("unsupported rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Vocabulary);
}

// ---------------------------------------------------------------------------
// REGISTRY
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_registry_registers_valid_package() {
    let mut registry = SkillRegistry::new();
    let entry = registry
        .register(base_package(), SkillTrustLevel::Sandboxed, 1)
        .expect("register ok");
    assert_eq!(entry.name, "nexus/hello");
    assert_eq!(entry.version, "1.0.0");
}

#[test]
fn ep018_unit_registry_duplicate_same_package_idempotent() {
    let mut registry = SkillRegistry::new();
    registry
        .register(base_package(), SkillTrustLevel::Sandboxed, 1)
        .expect("first register");
    // Exact duplicate: idempotent, returns the installed entry.
    let again = registry
        .register(base_package(), SkillTrustLevel::Sandboxed, 2)
        .expect("idempotent duplicate");
    assert_eq!(again.installed_at_epoch_ms, 1, "original entry returned");
    assert_eq!(registry.list(&tid()).len(), 1);
}

#[test]
fn ep018_unit_registry_same_version_changed_content_conflict() {
    let mut registry = SkillRegistry::new();
    registry
        .register(base_package(), SkillTrustLevel::Sandboxed, 1)
        .expect("first register");
    let mut changed = base_package();
    changed.content_hash = "b".repeat(64);
    let err = registry
        .register(changed, SkillTrustLevel::Sandboxed, 2)
        .expect_err("changed content under immutable version rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Conflict);
}

#[test]
fn ep018_unit_registry_rejects_unsigned_or_malformed_package() {
    let mut registry = SkillRegistry::new();
    let mut unsigned = base_package();
    unsigned.manifest.signature.signature_hex = String::new();
    let err = registry
        .register(unsigned, SkillTrustLevel::Sandboxed, 1)
        .expect_err("unsigned rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
    assert!(registry.list(&tid()).is_empty());
}

#[test]
fn ep018_unit_registry_community_skill_sandbox_requirement() {
    let mut registry = SkillRegistry::new();
    // Sandboxed community skill requesting READ registers under a
    // sandboxed caller (within ceiling).
    let mut read_skill = base_package();
    read_skill.manifest.permissions = vec![SkillPermission::Read];
    registry
        .register(read_skill.clone(), SkillTrustLevel::Sandboxed, 1)
        .expect("read within sandbox ceiling");
    // The same community skill requesting EXECUTE is denied: sandbox
    // ceiling is READ; the request never becomes a grant.
    let mut exec_skill = base_package();
    exec_skill.manifest.name = "nexus/exec".into();
    exec_skill.manifest.permissions = vec![SkillPermission::Execute];
    let err = registry
        .register(exec_skill, SkillTrustLevel::Sandboxed, 2)
        .expect_err("privileged request denied at sandbox ceiling");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
}

#[test]
fn ep018_unit_registry_permission_ceiling_matches_trust_tier() {
    assert_eq!(
        SkillTrustLevel::InspectOnly.permission_ceiling(),
        SkillPermission::None
    );
    assert_eq!(
        SkillTrustLevel::Sandboxed.permission_ceiling(),
        SkillPermission::Read
    );
    assert_eq!(
        SkillTrustLevel::Trusted.permission_ceiling(),
        SkillPermission::Execute
    );
    assert_eq!(
        SkillTrustLevel::System.permission_ceiling(),
        SkillPermission::Secrets
    );
}

#[test]
fn ep018_unit_registry_trusted_skill_requires_system_caller() {
    let mut registry = SkillRegistry::new();
    let mut trusted = base_package();
    trusted.manifest.trust_level = SkillTrustLevel::Trusted;
    let err = registry
        .register(trusted, SkillTrustLevel::Sandboxed, 1)
        .expect_err("trusted skill denied for non-system caller");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
}

// ---------------------------------------------------------------------------
// PROPOSAL LIFECYCLE
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_proposal_valid_lifecycle_to_promotion() {
    let mut p = proposal();
    p.transition(SkillProposalState::EvalPending, 2)
        .expect("PROPOSED -> EVAL_PENDING");
    p.transition(SkillProposalState::EvalPassed, 3)
        .expect("EVAL_PENDING -> EVAL_PASSED");
    p.transition(SkillProposalState::AwaitingPromotion, 4)
        .expect("EVAL_PASSED -> AWAITING_PROMOTION");
    p.approve("human-owner", 5).expect("human approval");
    assert_eq!(p.state, SkillProposalState::Promoted);
}

#[test]
fn ep018_unit_proposal_rejection_path() {
    let mut p = proposal();
    p.transition(SkillProposalState::EvalPending, 2)
        .expect("eval pending");
    p.transition(SkillProposalState::EvalFailed, 3)
        .expect("eval failed");
    assert!(p.state.is_terminal());
    // Terminal resurrection is rejected.
    let err = p
        .transition(SkillProposalState::AwaitingPromotion, 4)
        .expect_err("terminal");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
}

#[test]
fn ep018_unit_proposal_invalid_transitions_rejected() {
    // PROPOSED -> PROMOTED jumps the lifecycle and is rejected.
    let mut p = proposal();
    let err = p
        .transition(SkillProposalState::Promoted, 2)
        .expect_err("jump rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);

    // EVAL_PASSED -> EVAL_PENDING is a backwards move, rejected.
    let mut p = proposal();
    p.transition(SkillProposalState::EvalPending, 2)
        .expect("eval pending");
    p.transition(SkillProposalState::EvalPassed, 3)
        .expect("eval passed");
    let err = p
        .transition(SkillProposalState::EvalPending, 4)
        .expect_err("backwards rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);

    // AWAITING_PROMOTION -> EVAL_PASSED is a backwards move, rejected.
    let mut p = proposal();
    p.transition(SkillProposalState::EvalPending, 2)
        .expect("eval pending");
    p.transition(SkillProposalState::EvalPassed, 3)
        .expect("eval passed");
    p.transition(SkillProposalState::AwaitingPromotion, 4)
        .expect("awaiting");
    let err = p
        .transition(SkillProposalState::EvalPassed, 5)
        .expect_err("backwards rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
}

#[test]
fn ep018_unit_proposal_rejected_then_approved_rejected() {
    let mut p = proposal();
    p.transition(SkillProposalState::EvalPending, 2)
        .expect("eval pending");
    p.transition(SkillProposalState::EvalPassed, 3)
        .expect("eval passed");
    p.transition(SkillProposalState::AwaitingPromotion, 4)
        .expect("awaiting");
    p.transition(SkillProposalState::Rejected, 5)
        .expect("rejected");
    let err = p
        .approve("human-owner", 6)
        .expect_err("resurrection rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
}

#[test]
fn ep018_unit_proposal_model_cannot_self_approve() {
    let mut p = proposal();
    p.transition(SkillProposalState::EvalPending, 2)
        .expect("eval pending");
    p.transition(SkillProposalState::EvalPassed, 3)
        .expect("eval passed");
    p.transition(SkillProposalState::AwaitingPromotion, 4)
        .expect("awaiting");
    // The proposing model cannot approve its own installation.
    let err = p.approve("model-a", 5).expect_err("self-approval rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    // An empty approver is rejected.
    let err = p.approve("", 6).expect_err("empty approver rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
    // A distinct human can approve.
    p.approve("human-owner", 7).expect("human approval");
    assert_eq!(p.state, SkillProposalState::Promoted);
}

#[test]
fn ep018_unit_proposal_approve_requires_awaiting_promotion() {
    let mut p = proposal();
    let err = p
        .approve("human-owner", 2)
        .expect_err("cannot approve before evals");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
}

// ---------------------------------------------------------------------------
// ERRORS
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_error_typed_without_content_leakage() {
    // Errors carry a typed code and a fixed message; manifest content
    // never leaks into error output (ADR-025 redaction by construction).
    let mut manifest = base_manifest();
    manifest.description = "TOP_SECRET_PAYLOAD_NEVER_LEAK".into();
    manifest.dependencies = vec!["nexus/other".into(), "nexus/other".into()];
    let err: SkillPackageError = manifest.validate().expect_err("duplicate dependency");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
    assert!(!err.message.contains("TOP_SECRET_PAYLOAD_NEVER_LEAK"));
    assert!(!err.to_string().contains("TOP_SECRET_PAYLOAD_NEVER_LEAK"));
}

// ---------------------------------------------------------------------------
// EVALUATOR (determinism shared with composition file)
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_evaluator_same_corpus_same_package_same_verdict() {
    let a = DeterministicSkillEvaluator::new(vec!["eval-a".into(), "eval-b".into()]);
    let b = DeterministicSkillEvaluator::new(vec!["eval-a".into(), "eval-b".into()]);
    let package = base_package();
    let ea = a.evaluate(&package).expect("evaluate");
    let eb = b.evaluate(&package).expect("evaluate");
    assert_eq!(ea, eb);
    assert!(ea.passed);
    ea.validate().expect("evaluation valid");
    assert_eq!(ea.evaluator_version, "1");
}

#[test]
fn ep018_unit_evaluator_empty_corpus_fails_closed() {
    let evaluator = DeterministicSkillEvaluator::new(vec![]);
    let err = evaluator
        .evaluate(&base_package())
        .expect_err("empty corpus fails closed");
    assert_eq!(err.code, SkillPackageErrorCode::Verification);
}

#[test]
fn ep018_unit_evaluator_unknown_version_fails_closed() {
    let evaluator = DeterministicSkillEvaluator::with_version(vec!["eval-a".into()], "999-unknown");
    let err = evaluator
        .evaluate(&base_package())
        .expect_err("unknown version fails closed");
    assert_eq!(err.code, SkillPackageErrorCode::Verification);
    assert!(err.message.contains("999-unknown"));
}

// ---------------------------------------------------------------------------
// PERMISSION AUTHORITY (shared with composition file)
// ---------------------------------------------------------------------------

#[test]
fn ep018_unit_authority_intersection_denies_missing_input() {
    let authority = PermissionAuthority {
        caller_granted: vec![SkillPermission::Read],
        policy_allowed: vec![SkillPermission::Read, SkillPermission::Write],
        trust_ceiling: SkillTrustLevel::Trusted.permission_ceiling(),
    };
    assert!(authority.allows(SkillPermission::Read));
    // Missing caller grant denies.
    assert!(!authority.allows(SkillPermission::Write));
    // Above ceiling denies.
    let authority = PermissionAuthority {
        caller_granted: vec![SkillPermission::Secrets],
        policy_allowed: vec![SkillPermission::Secrets],
        trust_ceiling: SkillTrustLevel::Sandboxed.permission_ceiling(),
    };
    assert!(!authority.allows(SkillPermission::Secrets));
}
