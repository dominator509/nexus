//! EP-018 M4 forced-failure suite (SPEC-010; ADR-025).
//!
//! Proves the skill plane fails safely under dependency, policy,
//! security, and resource faults. Real mechanisms only: a store whose
//! persistence fails (proving rollback/compensation of partial side
//! effects), corrupted bundle/state files on disk, denied permission
//! decisions, tampered content, revoked execution boundaries, and
//! redacted error output. The component being proven is never mocked;
//! the failing store is a controlled failure at the persistence port.

use nexus_skills::{
    JsonFileSkillRegistryStore, SkillBundleLoader, SkillPackageErrorCode, SkillPermission,
    SkillRegistry, SkillRegistryState, SkillRegistryStore, SkillTrustLevel,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

fn skills_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../skills")
        .canonicalize()
        .expect("canonical skills root")
}

fn loader() -> SkillBundleLoader {
    SkillBundleLoader::new(skills_root())
}

fn summarize_bundle() -> nexus_skills::SkillBundle {
    loader()
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle")
}

/// A store whose `save` fails exactly once at a chosen call index:
/// exercises rollback/compensation of partial side effects at the
/// real persistence port. `failing_on(1)` fails the first save;
/// `failing_on(2)` lets an install (save 1) succeed and fails the
/// remove/revoke save (save 2).
#[derive(Clone)]
struct FailingStore {
    fail_on: usize,
    save_count: Rc<RefCell<usize>>,
}

impl FailingStore {
    fn new() -> Self {
        Self::failing_on(1)
    }
    fn failing_on(save_index: usize) -> Self {
        Self {
            fail_on: save_index,
            save_count: Rc::new(RefCell::new(0)),
        }
    }
}

impl SkillRegistryStore for FailingStore {
    fn load(&self) -> Result<SkillRegistryState, nexus_skills::SkillPackageError> {
        Ok(SkillRegistryState {
            entries: Vec::new(),
        })
    }
    fn save(&self, _state: &SkillRegistryState) -> Result<(), nexus_skills::SkillPackageError> {
        let mut count = self.save_count.borrow_mut();
        *count += 1;
        if *count == self.fail_on {
            return Err(nexus_skills::SkillPackageError::unavailable(
                "controlled persistence failure",
                Some("failing-store".into()),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PARTIAL SIDE EFFECT / ROLLBACK
// ---------------------------------------------------------------------------

#[test]
fn ep018_failure_install_rolls_back_when_persistence_fails() {
    let store = FailingStore::new();
    let mut registry = SkillRegistry::new();
    let err = registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect_err("persistence failure propagates");
    assert_eq!(err.code, SkillPackageErrorCode::Unavailable);
    // Rollback: the in-memory entry is gone; memory and disk agree.
    assert!(registry.get("nexus/summarize", "1.0.0").is_none());
    assert!(registry
        .list(&nexus_skills::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072").expect("tenant"))
        .is_empty());
}

#[test]
fn ep018_failure_remove_restores_entry_when_persistence_fails() {
    // Install's save (1st) succeeds; remove's save (2nd) fails -> the
    // removed entry is restored so memory and disk never diverge.
    let store = FailingStore::failing_on(2);
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install ok");
    let err = registry
        .remove("nexus/summarize", "1.0.0", &store)
        .expect_err("persistence failure propagates");
    assert_eq!(err.code, SkillPackageErrorCode::Unavailable);
    assert!(registry.get("nexus/summarize", "1.0.0").is_some());
}

#[test]
fn ep018_failure_revoke_undoes_when_persistence_fails() {
    // Install's save (1st) succeeds; revoke's save (2nd) fails -> the
    // revocation is undone; revoked state is never committed in memory.
    let store = FailingStore::failing_on(2);
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install ok");
    let err = registry
        .revoke("nexus/summarize", "1.0.0", &store)
        .expect_err("persistence failure propagates");
    assert_eq!(err.code, SkillPackageErrorCode::Unavailable);
    assert!(!registry.is_revoked("nexus/summarize", "1.0.0"));
}

// ---------------------------------------------------------------------------
// MALFORMED INPUT
// ---------------------------------------------------------------------------

#[test]
fn ep018_failure_loader_rejects_corrupted_manifest_json() {
    let tmp = std::env::temp_dir().join(format!("ep018-m4-badjson-{}", std::process::id()));
    let dir = tmp.join("nexus/bad/1.0.0");
    std::fs::create_dir_all(&dir).expect("create dir");
    std::fs::write(dir.join("manifest.json"), b"{ not valid json").expect("write");
    std::fs::write(dir.join("SKILL.md"), b"payload").expect("write");
    let err = SkillBundleLoader::new(tmp.clone())
        .load("nexus/bad", "1.0.0")
        .expect_err("corrupt manifest rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn ep018_failure_loader_rejects_unknown_enum_value() {
    let tmp = std::env::temp_dir().join(format!("ep018-m4-badenum-{}", std::process::id()));
    let dir = tmp.join("nexus/bad/1.0.0");
    std::fs::create_dir_all(&dir).expect("create dir");
    let mut manifest = summarize_bundle().package.manifest;
    manifest.name = "nexus/bad".into();
    let mut json = serde_json::to_value(&manifest).expect("serialize");
    json["trust_level"] = serde_json::json!("GOD_MODE");
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&json).expect("serialize"),
    )
    .expect("write");
    std::fs::write(dir.join("SKILL.md"), b"payload").expect("write");
    let err = SkillBundleLoader::new(tmp.clone())
        .load("nexus/bad", "1.0.0")
        .expect_err("unknown enum rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn ep018_failure_loader_rejects_malformed_signature_encoding() {
    let tmp = std::env::temp_dir().join(format!("ep018-m4-badsig-{}", std::process::id()));
    let dir = tmp.join("nexus/bad/1.0.0");
    std::fs::create_dir_all(&dir).expect("create dir");
    let mut manifest = summarize_bundle().package.manifest;
    manifest.name = "nexus/bad".into();
    manifest.signature.signature_hex = "not-hex".into();
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize"),
    )
    .expect("write");
    std::fs::write(dir.join("SKILL.md"), b"payload").expect("write");
    let err = SkillBundleLoader::new(tmp.clone())
        .load("nexus/bad", "1.0.0")
        .expect_err("malformed signature rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// UNAVAILABLE DEPENDENCY / RESOURCE
// ---------------------------------------------------------------------------

#[test]
fn ep018_failure_loader_missing_bundle_is_not_found() {
    let err = loader()
        .load("nexus/ghost", "9.9.9")
        .expect_err("missing bundle rejected");
    assert_eq!(err.code, SkillPackageErrorCode::NotFound);
}

#[test]
fn ep018_failure_store_load_corrupted_state_fails_closed() {
    let path = std::env::temp_dir().join(format!("ep018-m4-state-{}.json", std::process::id()));
    std::fs::write(&path, b"{ corrupted state").expect("write corrupted state");
    let store = JsonFileSkillRegistryStore::new(path.clone());
    let err = store.load().expect_err("corrupted state rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------------------------------
// DUPLICATE REQUEST / TAMPER
// ---------------------------------------------------------------------------

#[test]
fn ep018_failure_duplicate_install_conflict_on_tampered_content() {
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-dup-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install");
    let mut tampered = summarize_bundle().package;
    tampered.content_hash = "0".repeat(64);
    let err = registry
        .register(tampered, SkillTrustLevel::Sandboxed, 2)
        .expect_err("tampered content conflict");
    assert_eq!(err.code, SkillPackageErrorCode::Conflict);
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_failure_duplicate_install_same_identity_is_idempotent() {
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-idem-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install once");
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 2, &store)
        .expect("exact duplicate idempotent");
    assert_eq!(
        registry
            .list(
                &nexus_skills::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072")
                    .expect("tenant")
            )
            .len(),
        1
    );
    let _ = std::fs::remove_file(store.path());
}

// ---------------------------------------------------------------------------
// DENIED PERMISSION / POLICY
// ---------------------------------------------------------------------------

#[test]
fn ep018_failure_permission_escalation_denied() {
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-esc-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    let notify = loader()
        .load("nexus/notify", "1.0.0")
        .expect("real notify bundle");
    // WRITE exceeds the SANDBOXED ceiling: escalation denied.
    let err = registry
        .install_bundle(notify, SkillTrustLevel::Sandboxed, 1, &store)
        .expect_err("escalation denied");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    // Nothing persisted.
    assert!(store.load().expect("store loads").entries.is_empty());
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_failure_trusted_skill_denied_for_community_caller() {
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-trust-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    let mut bundle = summarize_bundle();
    bundle.package.manifest.trust_level = SkillTrustLevel::Trusted;
    let err = registry
        .install_bundle(bundle, SkillTrustLevel::Sandboxed, 1, &store)
        .expect_err("trusted skill denied for sandbox caller");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    let _ = std::fs::remove_file(store.path());
}

// ---------------------------------------------------------------------------
// REVOCATION / EXECUTION BOUNDARY
// ---------------------------------------------------------------------------

#[test]
fn ep018_failure_revoked_skill_cannot_resolve_for_execution() {
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-rev-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install");
    registry
        .revoke("nexus/summarize", "1.0.0", &store)
        .expect("revoke");
    // The execution boundary fails closed: revoked is never executable.
    let err = registry
        .resolve_for_execution("nexus/summarize", "1.0.0")
        .expect_err("revoked execution denied");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    // Missing entries also fail closed.
    let err = registry
        .resolve_for_execution("nexus/summarize", "9.9.9")
        .expect_err("missing execution denied");
    assert_eq!(err.code, SkillPackageErrorCode::NotFound);
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_failure_store_reload_preserves_revocation() {
    // Revocation is durable: a process restart (store reload through
    // the real JSON file) must still deny execution.
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-reload-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install");
    registry
        .revoke("nexus/summarize", "1.0.0", &store)
        .expect("revoke");
    // "Restart": a fresh registry built from the persisted state.
    let reloaded = SkillRegistry::from_state(store.load().expect("store loads"));
    assert!(reloaded.is_revoked("nexus/summarize", "1.0.0"));
    let err = reloaded
        .resolve_for_execution("nexus/summarize", "1.0.0")
        .expect_err("revoked after restart");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_failure_revoked_identity_cannot_be_resurrected() {
    // Re-introducing the SAME immutable identity after revocation must
    // not clear the terminal revoked flag; only a NEW version is a new
    // installation identity (ADR-025 immutable-by-version).
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-resurrect-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install");
    registry
        .revoke("nexus/summarize", "1.0.0", &store)
        .expect("revoke");
    // Re-installing the same bundle (same name/version/content) is
    // idempotent against the existing revoked entry: still revoked.
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 2, &store)
        .expect("same identity install idempotent");
    assert!(registry.is_revoked("nexus/summarize", "1.0.0"));
    let err = registry
        .resolve_for_execution("nexus/summarize", "1.0.0")
        .expect_err("still revoked after reinstall attempt");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    // A NEW version is a distinct identity: allowed only if valid.
    let mut v2 = summarize_bundle().package;
    v2.manifest.version = "1.0.1".into();
    registry
        .register(v2, SkillTrustLevel::Sandboxed, 3)
        .expect("new version installs");
    assert!(registry.get("nexus/summarize", "1.0.1").is_some());
    assert!(registry
        .resolve_for_execution("nexus/summarize", "1.0.1")
        .is_ok());
    // The revoked v1 remains denied.
    let err = registry
        .resolve_for_execution("nexus/summarize", "1.0.0")
        .expect_err("v1 still revoked");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_failure_on_disk_tamper_rejected_at_install() {
    // Real on-disk tamper: modify one payload byte of the installed
    // bundle, reload through the real loader, and require the changed
    // canonical digest to be rejected as a conflict with the installed
    // immutable identity.
    let tmp = std::env::temp_dir().join(format!("ep018-m4-tamper-{}", std::process::id()));
    let dir = tmp.join("nexus/summarize/1.0.0");
    let original = summarize_bundle();
    let original_identity = original.package.canonical_identity();
    std::fs::create_dir_all(&dir).expect("create dir");
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&original.package.manifest).expect("serialize"),
    )
    .expect("write manifest");
    std::fs::write(dir.join("SKILL.md"), &original.payload).expect("write payload");
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-tamper-state-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(original, SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install original");
    // Flip one payload byte on disk under the same name/version.
    let mut tampered_payload = std::fs::read(dir.join("SKILL.md")).expect("read payload");
    let last = tampered_payload.len() - 1;
    tampered_payload[last] ^= 0x01;
    std::fs::write(dir.join("SKILL.md"), &tampered_payload).expect("write tampered payload");
    let tampered = SkillBundleLoader::new(tmp.clone())
        .load("nexus/summarize", "1.0.0")
        .expect("tampered bundle loads with changed digest");
    assert_ne!(tampered.package.canonical_identity(), original_identity);
    // Installing the tampered identity under the same immutable
    // version is rejected: no silent content replacement.
    let err = registry
        .install_bundle(tampered, SkillTrustLevel::Sandboxed, 2, &store)
        .expect_err("tampered install rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Conflict);
    // The installed original still resolves; the tampered content
    // never became executable.
    assert!(registry
        .resolve_for_execution("nexus/summarize", "1.0.0")
        .is_ok());
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_failure_remove_missing_version_fails_closed() {
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-rm-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    let err = registry
        .remove("nexus/summarize", "9.9.9", &store)
        .expect_err("missing remove denied");
    assert_eq!(err.code, SkillPackageErrorCode::NotFound);
    let _ = std::fs::remove_file(store.path());
}

// ---------------------------------------------------------------------------
// ERROR REDACTION / OBSERVABILITY
// ---------------------------------------------------------------------------

#[test]
fn ep018_failure_errors_are_typed_and_redacted() {
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-redact-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    let mut bundle = summarize_bundle();
    bundle.package.manifest.description = "TOP_SECRET_SKILL_DESCRIPTION".into();
    bundle.package.manifest.permissions = vec![SkillPermission::Secrets];
    let err = registry
        .install_bundle(bundle, SkillTrustLevel::Sandboxed, 1, &store)
        .expect_err("policy denial");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    assert!(!err.message.contains("TOP_SECRET_SKILL_DESCRIPTION"));
    assert!(!err.to_string().contains("TOP_SECRET_SKILL_DESCRIPTION"));
    let _ = std::fs::remove_file(store.path());
}

// ---------------------------------------------------------------------------
// BOUNDED RECOVERY
// ---------------------------------------------------------------------------

#[test]
fn ep018_failure_clear_is_bounded_recovery() {
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m4-clear-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(summarize_bundle(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install");
    registry.clear(&store).expect("clear persists");
    assert!(registry
        .list(&nexus_skills::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072").expect("tenant"))
        .is_empty());
    let reloaded = SkillRegistry::from_state(store.load().expect("store loads"));
    assert!(reloaded
        .list(&nexus_skills::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072").expect("tenant"))
        .is_empty());
    let _ = std::fs::remove_file(store.path());
}
