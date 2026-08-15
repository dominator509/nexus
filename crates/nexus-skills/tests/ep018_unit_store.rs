//! EP-018 M2 registry persistence and lifecycle suite (SPEC-010
//! behavior 6; ADR-025).
//!
//! Proves the functional skill system lifecycle through REAL filesystem
//! persistence: install a scanned bundle (load -> register -> save),
//! reload from disk, remove, revoke, update to a new version, and the
//! fail-closed unauthorized states. Authority is enforced on mutation,
//! never reconstructed from persisted state.

use nexus_skills::{
    JsonFileSkillRegistryStore, SkillBundleLoader, SkillPackageErrorCode, SkillRegistry,
    SkillRegistryStore, SkillTrustLevel,
};
use std::path::PathBuf;

fn skills_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../skills")
        .canonicalize()
        .expect("canonical skills root")
}

fn temp_store(tag: &str) -> JsonFileSkillRegistryStore {
    let path = std::env::temp_dir().join(format!("ep018-{tag}-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    JsonFileSkillRegistryStore::new(path)
}

#[test]
fn ep018_unit_store_install_persists_and_reloads() {
    let loader = SkillBundleLoader::new(skills_root());
    let bundle = loader
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle");
    let store = temp_store("install");
    let mut registry = SkillRegistry::new();
    let entry = registry
        .install_bundle(bundle, SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install persists");
    assert!(!entry.revoked);
    // The state file is real and readable.
    let state = store.load().expect("store loads");
    assert_eq!(state.entries.len(), 1);
    // A fresh registry rebuilt from the store sees the same package.
    let reloaded = SkillRegistry::from_state(store.load().expect("store loads"));
    let got = reloaded
        .get("nexus/summarize", "1.0.0")
        .expect("reloaded entry");
    assert_eq!(got.package.content_hash, entry.package.content_hash);
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_unit_store_missing_state_is_empty_registry() {
    let path = std::env::temp_dir().join(format!("ep018-missing-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let store = JsonFileSkillRegistryStore::new(path);
    let state = store.load().expect("missing state loads as empty");
    assert!(state.entries.is_empty());
    let registry = SkillRegistry::from_state(state);
    assert!(registry
        .list(&nexus_skills::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072").expect("tenant"))
        .is_empty());
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_unit_store_remove_persists() {
    let loader = SkillBundleLoader::new(skills_root());
    let bundle = loader
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle");
    let store = temp_store("remove");
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(bundle, SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install");
    registry
        .remove("nexus/summarize", "1.0.0", &store)
        .expect("remove persists");
    assert!(registry.get("nexus/summarize", "1.0.0").is_none());
    let reloaded = SkillRegistry::from_state(store.load().expect("store loads"));
    assert!(reloaded.get("nexus/summarize", "1.0.0").is_none());
    // Removing a missing version fails closed.
    let err = registry
        .remove("nexus/summarize", "9.9.9", &store)
        .expect_err("missing remove rejected");
    assert_eq!(err.code, SkillPackageErrorCode::NotFound);
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_unit_store_revoke_is_terminal_and_persists() {
    let loader = SkillBundleLoader::new(skills_root());
    let bundle = loader
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle");
    let store = temp_store("revoke");
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(bundle, SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install");
    assert!(!registry.is_revoked("nexus/summarize", "1.0.0"));
    registry
        .revoke("nexus/summarize", "1.0.0", &store)
        .expect("revoke persists");
    assert!(registry.is_revoked("nexus/summarize", "1.0.0"));
    // Revocation survives reload.
    let reloaded = SkillRegistry::from_state(store.load().expect("store loads"));
    assert!(reloaded.is_revoked("nexus/summarize", "1.0.0"));
    // Revoking a missing version fails closed.
    let err = registry
        .revoke("nexus/summarize", "9.9.9", &store)
        .expect_err("missing revoke rejected");
    assert_eq!(err.code, SkillPackageErrorCode::NotFound);
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_unit_store_update_new_version_is_separate_immutable_package() {
    let loader = SkillBundleLoader::new(skills_root());
    let v1 = loader.load("nexus/summarize", "1.0.0").expect("real v1");
    let store = temp_store("update");
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(v1, SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install v1");
    // A new version is a NEW immutable package: it installs alongside.
    let mut v2 = loader
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle again");
    v2.package.manifest.version = "1.0.1".into();
    registry
        .install_bundle(v2, SkillTrustLevel::Sandboxed, 2, &store)
        .expect("install v2");
    assert!(registry.get("nexus/summarize", "1.0.0").is_some());
    assert!(registry.get("nexus/summarize", "1.0.1").is_some());
    let reloaded = SkillRegistry::from_state(store.load().expect("store loads"));
    assert!(reloaded.get("nexus/summarize", "1.0.1").is_some());
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_unit_store_install_denied_permission_is_fail_closed() {
    let loader = SkillBundleLoader::new(skills_root());
    let notify = loader
        .load("nexus/notify", "1.0.0")
        .expect("real notify bundle");
    let store = temp_store("deny");
    let mut registry = SkillRegistry::new();
    let err = registry
        .install_bundle(notify, SkillTrustLevel::Sandboxed, 1, &store)
        .expect_err("WRITE denied at sandbox ceiling");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    // Nothing was persisted: the failed install wrote no state.
    let state = store.load().expect("store loads");
    assert!(state.entries.is_empty());
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_unit_store_install_idempotent_duplicate() {
    let loader = SkillBundleLoader::new(skills_root());
    let bundle = loader.load("community/echo", "1.0.0").expect("real bundle");
    let store = temp_store("idem");
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(bundle.clone(), SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install once");
    registry
        .install_bundle(bundle, SkillTrustLevel::Sandboxed, 2, &store)
        .expect("exact duplicate idempotent");
    let state = store.load().expect("store loads");
    assert_eq!(state.entries.len(), 1);
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_unit_store_state_never_reconstructs_authority() {
    // Persisted state is entries only; loading it never re-grants
    // permissions. A reloaded registry enforces the same ceiling on
    // the next mutation.
    let loader = SkillBundleLoader::new(skills_root());
    let summarize = loader
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle");
    let store = temp_store("authority");
    let mut registry = SkillRegistry::new();
    registry
        .install_bundle(summarize, SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install");
    let mut reloaded = SkillRegistry::from_state(store.load().expect("store loads"));
    let notify = loader
        .load("nexus/notify", "1.0.0")
        .expect("real notify bundle");
    let err = reloaded
        .install_bundle(notify, SkillTrustLevel::Sandboxed, 2, &store)
        .expect_err("reloaded registry still enforces ceiling");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
    let _ = std::fs::remove_file(store.path());
}
