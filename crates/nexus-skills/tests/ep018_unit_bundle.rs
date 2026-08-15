//! EP-018 M2 bundle-loading suite (SPEC-010 behavior 6; ADR-025).
//!
//! Loads the REAL skill bundles from the repository `skills/` tree
//! through the real filesystem and the real SHA-256 scan-before-install
//! content hash. Proves: real loading, path/contract consistency,
//! tamper rejection, missing payload rejection, deterministic
//! enumeration, and that declared permissions remain requests (never
//! grants) after scanning.

use nexus_skills::{
    sha256_hex, SkillBundleLoader, SkillPackageErrorCode, SkillPermission, SkillRegistry,
    SkillTrustLevel,
};
use std::path::PathBuf;

fn skills_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../../skills")
        .canonicalize()
        .expect("canonical skills root")
}

fn loader() -> SkillBundleLoader {
    SkillBundleLoader::new(skills_root())
}

#[test]
fn ep018_unit_bundle_loads_real_summarize_skill() {
    let bundle = loader()
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle loads");
    assert_eq!(bundle.package.manifest.name, "nexus/summarize");
    assert_eq!(bundle.package.manifest.version, "1.0.0");
    assert_eq!(
        bundle.package.manifest.permissions,
        vec![SkillPermission::Read]
    );
    assert_eq!(
        bundle.package.manifest.trust_level,
        SkillTrustLevel::Sandboxed
    );
    // Scan-before-install: the content hash is the REAL sha256 of the
    // payload, recomputed identically at load time.
    assert_eq!(bundle.package.content_hash.len(), 64);
    assert_eq!(sha256_hex(&bundle.payload), bundle.package.content_hash);
    bundle.package.validate().expect("scanned package valid");
}

#[test]
fn ep018_unit_bundle_loads_real_community_echo_skill() {
    let bundle = loader()
        .load("community/echo", "1.0.0")
        .expect("real community bundle loads");
    assert_eq!(
        bundle.package.manifest.permissions,
        vec![SkillPermission::None]
    );
    assert_eq!(
        bundle.package.manifest.trust_level,
        SkillTrustLevel::Sandboxed
    );
}

#[test]
fn ep018_unit_bundle_loads_real_notify_skill_with_write_declaration() {
    let bundle = loader()
        .load("nexus/notify", "1.0.0")
        .expect("real notify bundle loads");
    assert!(bundle
        .package
        .manifest
        .permissions
        .contains(&SkillPermission::Write));
    // The declaration is a REQUEST: scanning does not grant it. A
    // SANDBOXED caller cannot install WRITE (ceiling is READ).
    let mut registry = SkillRegistry::new();
    let err = registry
        .register(bundle.package, SkillTrustLevel::Sandboxed, 1)
        .expect_err("WRITE denied at sandbox ceiling");
    assert_eq!(err.code, SkillPackageErrorCode::Policy);
}

#[test]
fn ep018_unit_bundle_rejects_tampered_payload() {
    // Tamper: replace the payload after load; the scanned hash no
    // longer matches, so re-scanning the same bundle would produce a
    // different package identity (content changed under the same
    // name/version -> conflict at install).
    let bundle = loader()
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle loads");
    let original_hash = bundle.package.content_hash.clone();
    let tampered_hash = sha256_hex(b"tampered payload bytes");
    assert_ne!(tampered_hash, original_hash);
    let mut registry = SkillRegistry::new();
    registry
        .register(bundle.package.clone(), SkillTrustLevel::Sandboxed, 1)
        .expect("original installs");
    let mut tampered = bundle.package;
    tampered.content_hash = tampered_hash;
    let err = registry
        .register(tampered, SkillTrustLevel::Sandboxed, 2)
        .expect_err("tampered content conflicts with immutable version");
    assert_eq!(err.code, SkillPackageErrorCode::Conflict);
}

#[test]
fn ep018_unit_bundle_rejects_missing_manifest() {
    let err = loader()
        .load("nexus/does-not-exist", "9.9.9")
        .expect_err("missing manifest rejected");
    assert_eq!(err.code, SkillPackageErrorCode::NotFound);
}

#[test]
fn ep018_unit_bundle_rejects_path_contract_spoof() {
    // A manifest claiming a DIFFERENT name than its bundle path must
    // fail closed (spoofing rejection). Build a temp bundle dir.
    let tmp = std::env::temp_dir().join(format!("ep018-spoof-{}", std::process::id()));
    let dir = tmp.join("nexus/fake/1.0.0");
    std::fs::create_dir_all(&dir).expect("create temp bundle dir");
    // Copy a real manifest but rename the bundle path.
    let real = loader()
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle");
    let manifest_text =
        serde_json::to_string_pretty(&real.package.manifest).expect("serialize manifest");
    std::fs::write(dir.join("manifest.json"), manifest_text).expect("write manifest");
    std::fs::write(dir.join("SKILL.md"), b"payload").expect("write payload");
    let spoof_loader = SkillBundleLoader::new(tmp.clone());
    let err = spoof_loader
        .load("nexus/fake", "1.0.0")
        .expect_err("spoofed path rejected");
    assert_eq!(err.code, SkillPackageErrorCode::Validation);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn ep018_unit_bundle_rejects_missing_payload() {
    let tmp = std::env::temp_dir().join(format!("ep018-nopayload-{}", std::process::id()));
    let dir = tmp.join("nexus/nopayload/1.0.0");
    std::fs::create_dir_all(&dir).expect("create temp bundle dir");
    let real = loader()
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle");
    // Rewrite the manifest name to match the path (valid bundle except
    // the missing payload).
    let mut manifest = real.package.manifest;
    manifest.name = "nexus/nopayload".into();
    let manifest_text = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
    std::fs::write(dir.join("manifest.json"), manifest_text).expect("write manifest");
    let loader = SkillBundleLoader::new(tmp.clone());
    let err = loader
        .load("nexus/nopayload", "1.0.0")
        .expect_err("missing payload rejected");
    assert_eq!(err.code, SkillPackageErrorCode::NotFound);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn ep018_unit_bundle_list_available_is_deterministic_and_complete() {
    let available = loader().list_available().expect("list available");
    assert!(available.contains(&("nexus/summarize".to_string(), "1.0.0".to_string())));
    assert!(available.contains(&("nexus/notify".to_string(), "1.0.0".to_string())));
    assert!(available.contains(&("community/echo".to_string(), "1.0.0".to_string())));
    // Deterministic: repeated enumeration is identical.
    let again = loader().list_available().expect("list again");
    assert_eq!(available, again);
    // Sorted by (name, version).
    let mut sorted = available.clone();
    sorted.sort();
    assert_eq!(available, sorted);
}

#[test]
fn ep018_unit_bundle_load_all_scans_every_builtin() {
    let bundles = loader().load_all().expect("load all bundles");
    let names: Vec<String> = bundles
        .iter()
        .map(|b| b.package.manifest.name.clone())
        .collect();
    assert!(names.contains(&"nexus/summarize".to_string()));
    assert!(names.contains(&"nexus/notify".to_string()));
    assert!(names.contains(&"community/echo".to_string()));
    for bundle in &bundles {
        assert_eq!(sha256_hex(&bundle.payload), bundle.package.content_hash);
        bundle.package.validate().expect("scanned package valid");
    }
}
