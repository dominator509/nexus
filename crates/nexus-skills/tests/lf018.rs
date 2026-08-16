//! EP-018 M5 / LF-018 live-fire proof (SPEC-010 behaviors 6-8;
//! ADR-025).
//!
//! `skill-install-and-run`: inspect, scan, approve, sign, install,
//! discover, execute, and roll back a skill without granting
//! undeclared capabilities. Every link is REAL:
//!
//!   real skill bundle on disk
//!     -> SkillBundleLoader (real fs I/O)
//!     -> real SHA-256 content hash
//!     -> manifest validation (fail closed)
//!     -> REAL ring Ed25519 signature verification
//!     -> proposal/evaluation (human promotion, no self-approval)
//!     -> installation through the real durable registry
//!     -> dependency resolution + permission authority computation
//!     -> resolve_for_execution (fail closed)
//!     -> REAL subprocess execution boundary (SkillExecutor)
//!     -> observable result
//!     -> revoke
//!     -> execution denied afterward
//!
//! The payload is a CONTROLLED_TEST_FIXTURE executable
//! (`tests/skills/fixtures/livefire-transform.sh`): a deterministic
//! input -> bounded transformation -> output skill. The executor
//! scrubs the environment and grants only the declared READ
//! permission; a WRITE directive at runtime is denied (exit 3).
//!
//! No link is simulated: no in-memory substitute for the bundle, the
//! crypto, the registry, or the process boundary.

use nexus_skills::manifest::{SkillManifest, SkillPackage};
use nexus_skills::vocabulary::SignatureAlgorithm;
use nexus_skills::{
    sha256_hex, sign_ed25519, verify_ed25519, DeterministicSkillComposer,
    DeterministicSkillEvaluator, JsonFileSkillRegistryStore, PermissionAuthority,
    SkillBundleLoader, SkillComposer, SkillEvaluator, SkillExecutor, SkillPackageErrorCode,
    SkillPermission, SkillProposal, SkillProposalState, SkillRegistry, SkillRegistryStore,
    SkillSignature, SkillTrustLevel, TenantId,
};
use std::path::{Path, PathBuf};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/skills/fixtures")
        .canonicalize()
        .expect("canonical fixtures root")
}

fn tenant() -> TenantId {
    TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072").expect("valid tenant")
}

/// Build a REAL skill bundle on disk under a temp root: manifest.json
/// plus the SKILL.md payload (the live-fire transform fixture), signed
/// with a REAL freshly generated Ed25519 keypair over the canonical
/// identity digest.
///
/// Returns the temp root, the public key hex, the payload, and the
/// signature hex.
fn write_real_signed_bundle(
    tmp: &Path,
    name: &str,
    version: &str,
    trust: SkillTrustLevel,
) -> (SkillBundleLoader, String, Vec<u8>, String) {
    let dir = tmp.join(name).join(version);
    std::fs::create_dir_all(&dir).expect("create bundle dir");
    let payload =
        std::fs::read(fixture_root().join("livefire-transform.sh")).expect("read fixture payload");
    let content_hash = sha256_hex(&payload);
    let identity = format!("{name}@{version}:{content_hash}");
    let (public_hex, signature_hex) = sign_ed25519(identity.as_bytes()).expect("real sign");
    let manifest = SkillManifest {
        skill_id: nexus_skills::SkillId::new("0cb7d278-1ed7-7da3-867e-99cbef7f8f0c")
            .expect("valid skill id"),
        tenant_id: tenant(),
        name: name.into(),
        version: version.into(),
        description: "live-fire transform skill (LF-018)".into(),
        permissions: vec![SkillPermission::Read],
        dependencies: vec![],
        network_rules: vec![],
        license: "MIT".into(),
        provenance: nexus_skills::ArtifactId::new("567c3a2e-7be9-77c4-87f6-883ddcc7fd86")
            .expect("valid artifact id"),
        trust_level: trust,
        signature: SkillSignature {
            algorithm: SignatureAlgorithm::Ed25519,
            public_key_hex: public_hex.clone(),
            signature_hex: signature_hex.clone(),
            signer: Some("nexus-livefire".into()),
        },
    };
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");
    std::fs::write(dir.join("SKILL.md"), &payload).expect("write payload");
    (
        SkillBundleLoader::new(tmp),
        public_hex,
        payload,
        signature_hex,
    )
}

fn proposal_for(package: SkillPackage, proposed_by: &str) -> SkillProposal {
    SkillProposal {
        proposal_id: format!("prop-{}", package.manifest.name.replace('/', "-")),
        skill_id: package.manifest.skill_id.clone(),
        tenant_id: package.manifest.tenant_id.clone(),
        correlation_id: nexus_skills::CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6074")
            .expect("valid correlation id"),
        package,
        state: SkillProposalState::Proposed,
        proposed_by: proposed_by.into(),
        created_at_epoch_ms: 1,
        updated_at_epoch_ms: 1,
    }
}

fn promote(proposal: &mut SkillProposal, approver: &str) {
    proposal
        .transition(SkillProposalState::EvalPending, 2)
        .expect("eval pending");
    proposal
        .transition(SkillProposalState::EvalPassed, 3)
        .expect("eval passed");
    proposal
        .transition(SkillProposalState::AwaitingPromotion, 4)
        .expect("awaiting promotion");
    proposal.approve(approver, 5).expect("human approve");
    assert_eq!(proposal.state, SkillProposalState::Promoted);
}

#[test]
fn lf018_real_chain_install_execute_revoke() {
    let tmp = std::env::temp_dir().join(format!("lf018-chain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let (loader, public_hex, payload, _sig) =
        write_real_signed_bundle(&tmp, "nexus/livefire", "1.0.0", SkillTrustLevel::Sandboxed);

    // Inspect + scan: real loader, real SHA-256 content hash, manifest
    // validation, and REAL cryptographic verification.
    let bundle = loader
        .load("nexus/livefire", "1.0.0")
        .expect("real bundle loads");
    assert_eq!(sha256_hex(&payload), bundle.package.content_hash);
    bundle.package.validate().expect("package valid");
    verify_ed25519(
        &public_hex,
        &bundle.package.manifest.signature.signature_hex,
        &nexus_skills::package_signing_message(&bundle.package),
    )
    .expect("real ed25519 verify");
    assert!(bundle
        .package
        .manifest
        .signature
        .verify_cryptographic(&bundle.package)
        .is_ok());

    // Approve: proposal lifecycle -> human promotion (no self-approval).
    let mut proposal = proposal_for(bundle.package.clone(), "model-a");
    promote(&mut proposal, "human-owner");
    assert_eq!(proposal.state, SkillProposalState::Promoted);

    // Install through the real durable registry.
    let state_path = std::env::temp_dir().join(format!("lf018-state-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&state_path);
    let store = JsonFileSkillRegistryStore::new(state_path.clone());
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(bundle.clone(), SkillTrustLevel::Sandboxed, 10, &store)
        .expect("install");

    // Discover + resolve.
    assert!(registry.get("nexus/livefire", "1.0.0").is_some());
    let resolved = registry
        .resolve_for_execution("nexus/livefire", "1.0.0")
        .expect("resolved for execution");

    // Permission authority computation (no runtime self-grant).
    let authority = PermissionAuthority {
        caller_granted: vec![SkillPermission::Read],
        policy_allowed: vec![SkillPermission::Read],
        trust_ceiling: SkillTrustLevel::Sandboxed.permission_ceiling(),
    };
    assert!(authority.allows(SkillPermission::Read));
    assert!(!authority.allows(SkillPermission::Write));

    // Execute through the REAL subprocess boundary: input ->
    // bounded transformation -> output artifact.
    let executor = SkillExecutor::new(
        std::env::temp_dir().join(format!("lf018-scratch-{}", std::process::id())),
    );
    let result = executor
        .execute(
            &resolved,
            &payload,
            b"hello world",
            &[SkillPermission::Read],
        )
        .expect("execution");
    assert_eq!(result.exit_code, 0);
    assert!(
        result.stdout.contains("transformed:hello world"),
        "got: {}",
        result.stdout
    );

    // Revoke: terminal for execution.
    registry
        .revoke("nexus/livefire", "1.0.0", &store)
        .expect("revoke");
    let err = registry
        .resolve_for_execution("nexus/livefire", "1.0.0")
        .expect_err("revoked execution denied");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);

    // Restart (store reload) preserves revocation.
    let reloaded = SkillRegistry::from_state(store.load().expect("store loads"));
    assert!(reloaded.is_revoked("nexus/livefire", "1.0.0"));
    let err = reloaded
        .resolve_for_execution("nexus/livefire", "1.0.0")
        .expect_err("revoked after restart");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);

    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_file(&state_path);
}

#[test]
fn lf018_tampered_bundle_fails_signature_and_never_executes() {
    let tmp = std::env::temp_dir().join(format!("lf018-tamper-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let (loader, _public_hex, payload, _sig) =
        write_real_signed_bundle(&tmp, "nexus/livefire", "1.0.0", SkillTrustLevel::Sandboxed);

    // Install the genuine bundle first (the immutable installed
    // identity).
    let state_path =
        std::env::temp_dir().join(format!("lf018-tamper-state-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&state_path);
    let store = JsonFileSkillRegistryStore::new(state_path.clone());
    let mut registry = SkillRegistry::new();
    let original = loader
        .load("nexus/livefire", "1.0.0")
        .expect("genuine loads");
    registry
        .install_bundle(original.clone(), SkillTrustLevel::Sandboxed, 10, &store)
        .expect("genuine install");

    // Flip one payload byte on disk after signing: digest changes, the
    // signature no longer verifies.
    let payload_path = tmp.join("nexus/livefire/1.0.0/SKILL.md");
    let mut tampered = payload.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    std::fs::write(&payload_path, &tampered).expect("write tampered");

    let tampered_bundle = loader
        .load("nexus/livefire", "1.0.0")
        .expect("loads with new hash");
    assert_ne!(tampered_bundle.package.content_hash, sha256_hex(&payload));
    // 1. Cryptographic verification fails: the signature covers the
    //    ORIGINAL canonical identity, not the tampered digest.
    let err = tampered_bundle
        .package
        .manifest
        .signature
        .verify_cryptographic(&tampered_bundle.package)
        .expect_err("tampered signature fails");
    assert_eq!(err.code, SkillPackageErrorCode::Verification);
    // 2. The tampered identity cannot replace the installed immutable
    //    version (ADR-025): same name/version, different content.
    let err = registry
        .install_bundle(
            tampered_bundle.clone(),
            SkillTrustLevel::Sandboxed,
            11,
            &store,
        )
        .expect_err("tampered install rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Conflict);
    // 3. The tampered package never executes: the executor requires
    //    real cryptographic verification of the package identity.
    let executor = SkillExecutor::new(
        std::env::temp_dir().join(format!("lf018-tamper-scratch-{}", std::process::id())),
    );
    let err = executor
        .execute(
            &tampered_bundle.package,
            &tampered,
            b"hello",
            &[SkillPermission::Read],
        )
        .expect_err("tampered execution denied");
    assert_eq!(err.code, SkillPackageErrorCode::Verification);
    // 4. The installed ORIGINAL still resolves (immutable identity
    //    preserved).
    assert!(registry
        .resolve_for_execution("nexus/livefire", "1.0.0")
        .is_ok());

    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_file(&state_path);
}

#[test]
fn lf018_wrong_signer_fails_verification() {
    // A signature made by a DIFFERENT key must not verify.
    let message = b"nexus/livefire@1.0.0:deadbeef";
    let (pk_a, _) = sign_ed25519(message).expect("signer a");
    let (pk_b, sig_b) = sign_ed25519(message).expect("signer b");
    assert_ne!(pk_a, pk_b);
    let err = verify_ed25519(&pk_a, &sig_b, message).expect_err("wrong signer fails");
    assert_eq!(err.code, SkillPackageErrorCode::Verification);
}

#[test]
fn lf018_runtime_permission_not_granted_is_denied() {
    let tmp = std::env::temp_dir().join(format!("lf018-perm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let (loader, _pk, payload, _sig) =
        write_real_signed_bundle(&tmp, "nexus/livefire", "1.0.0", SkillTrustLevel::Sandboxed);
    let bundle = loader.load("nexus/livefire", "1.0.0").expect("loads");

    // The skill declares READ only; the caller grants only READ.
    // A WRITE directive at runtime is denied by the fixture (exit 3)
    // because WRITE was never granted in the scrubbed environment.
    let executor = SkillExecutor::new(
        std::env::temp_dir().join(format!("lf018-perm-scratch-{}", std::process::id())),
    );
    let result = executor
        .execute(
            &bundle.package,
            &payload,
            b"WRITE:secret",
            &[SkillPermission::Read],
        )
        .expect("execution returns observable result");
    assert_eq!(result.exit_code, 3);
    assert!(
        result.stderr.contains("write-denied"),
        "got: {}",
        result.stderr
    );

    // The registry policy independently denies a WRITE declaration at
    // the SANDBOXED ceiling: install fails closed, nothing persists.
    let state_path =
        std::env::temp_dir().join(format!("lf018-perm-state-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&state_path);
    let store = JsonFileSkillRegistryStore::new(state_path.clone());
    let mut registry = SkillRegistry::new();
    let mut bundle_w = write_real_signed_bundle(
        &std::env::temp_dir().join(format!("lf018-perm2-{}", std::process::id())),
        "nexus/livefire",
        "1.0.0",
        SkillTrustLevel::Sandboxed,
    );
    let _ = &mut bundle_w;
    let loader2 = SkillBundleLoader::new(
        std::env::temp_dir().join(format!("lf018-perm2-{}", std::process::id())),
    );
    let bundle2 = loader2.load("nexus/livefire", "1.0.0").expect("loads");
    let mut escalated = bundle2.package.clone();
    escalated.manifest.permissions = vec![SkillPermission::Write];
    let err = registry
        .install_bundle(
            nexus_skills::SkillBundle {
                package: escalated,
                payload: payload.clone(),
            },
            SkillTrustLevel::Sandboxed,
            10,
            &store,
        )
        .expect_err("escalation denied");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);

    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_file(&state_path);
}

#[test]
fn lf018_agent_cannot_self_approve_installation() {
    // A model/agent may PROPOSE a skill; it may not self-approve.
    let tmp = std::env::temp_dir().join(format!("lf018-self-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let (loader, _pk, _payload, _sig) =
        write_real_signed_bundle(&tmp, "nexus/livefire", "1.0.0", SkillTrustLevel::Sandboxed);
    let bundle = loader.load("nexus/livefire", "1.0.0").expect("loads");
    let mut proposal = proposal_for(bundle.package, "model-a");
    proposal
        .transition(SkillProposalState::EvalPending, 2)
        .expect("eval pending");
    proposal
        .transition(SkillProposalState::EvalPassed, 3)
        .expect("eval passed");
    proposal
        .transition(SkillProposalState::AwaitingPromotion, 4)
        .expect("awaiting promotion");
    // The proposer (model-a) cannot approve its own proposal.
    let err = proposal
        .approve("model-a", 5)
        .expect_err("self-approval denied");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    // A distinct human CAN.
    proposal.approve("human-owner", 5).expect("human approval");
    assert_eq!(proposal.state, SkillProposalState::Promoted);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn lf018_evaluation_frozen_corpus_gate() {
    // Factory output must pass frozen evals (SPEC-010 behavior 8):
    // the deterministic evaluator exercises the corpus; an EMPTY
    // corpus fails closed (no promotion without evaluation evidence).
    let tmp = std::env::temp_dir().join(format!("lf018-eval-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let (loader, _pk, _payload, _sig) =
        write_real_signed_bundle(&tmp, "nexus/livefire", "1.0.0", SkillTrustLevel::Sandboxed);
    let bundle = loader.load("nexus/livefire", "1.0.0").expect("loads");

    let evaluator = DeterministicSkillEvaluator::new(vec!["lf018-eval-1".into()]);
    let eval = evaluator.evaluate(&bundle.package).expect("evaluated");
    assert!(eval.passed);
    assert_eq!(eval.eval_ids, vec!["lf018-eval-1".to_string()]);

    let empty = DeterministicSkillEvaluator::new(vec![]);
    let err = empty
        .evaluate(&bundle.package)
        .expect_err("empty corpus fails closed");
    assert_eq!(err.code, SkillPackageErrorCode::Verification);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn lf018_dependency_composition_boundary() {
    // Dependency composition: a root with a real dependency resolves;
    // a missing dependency fails closed; effective authority never
    // exceeds the caller's envelope.
    let tmp = std::env::temp_dir().join(format!("lf018-comp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let (loader, _pk, _payload, _sig) =
        write_real_signed_bundle(&tmp, "nexus/livefire", "1.0.0", SkillTrustLevel::Sandboxed);
    let base = loader.load("nexus/livefire", "1.0.0").expect("loads");

    let mut root = base.package.clone();
    root.manifest.name = "nexus/root".into();
    root.manifest.dependencies = vec!["nexus/livefire".into()];
    root.manifest.permissions = vec![SkillPermission::Read];

    let composer = DeterministicSkillComposer;
    let authority = PermissionAuthority {
        caller_granted: vec![SkillPermission::Read],
        policy_allowed: vec![SkillPermission::Read],
        trust_ceiling: SkillTrustLevel::Sandboxed.permission_ceiling(),
    };
    // Dependency present: composition succeeds and effective authority
    // is the intersection (never wider than the caller grant).
    let composition = composer
        .compose_with_authority(&root, &[root.clone(), base.package.clone()], &authority)
        .expect("composition with dependency");
    assert!(composition
        .versions
        .contains(&"nexus/livefire@1.0.0".to_string()));
    assert_eq!(
        composition.effective_permissions,
        vec![SkillPermission::Read]
    );

    // Dependency missing: fails closed.
    let err = composer
        .compose_with_authority(&root, &[root.clone()], &authority)
        .expect_err("missing dependency fails closed");
    assert_eq!(err.code, nexus_skills::SkillCompositionErrorCode::NotFound);
    let _ = std::fs::remove_dir_all(&tmp);
}
