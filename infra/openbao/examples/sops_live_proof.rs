//! EP-009 M2 SOPS+age live proof: real BootstrapSecretStore path.
//!
//! Builds a real SOPS-encrypted envelope with a real ephemeral age
//! identity (generated in a temp dir outside the repo), then resolves it
//! through the REAL nexus-openbao `SopsBootstrapStore` adapter. Proves:
//! - the adapter decrypts with the correct identity (process piping,
//!   identity never persisted next to ciphertext);
//! - the canary value is absent from the encrypted document;
//! - wrong identity fails typed (ProviderAuthorization);
//! - missing identity fails typed;
//! - corrupted document fails typed;
//! - decrypted material + identity removed immediately (temp dir).

use std::io::Write;
use std::process::Command;

use nexus_openbao::SopsBootstrapStore;
use nexus_trust::bootstrap::{BootstrapBundle, BootstrapSecretStore};
use nexus_trust::secret::SecretReference;

const SOPS: &str = "/usr/local/bin/sops";
const AGE: &str = "/usr/bin/age";
const AGE_KEYGEN: &str = "/usr/bin/age-keygen";
const CANARY: &str = "canary-nexus-ep009-adapter-3f7c91b2";

fn run(cmd: &mut Command) -> std::process::Output {
    cmd.output().expect("subprocess must run")
}

fn main() {
    let td = std::env::temp_dir().join(format!("nexus-ep009-adapter-{}", std::process::id()));
    std::fs::create_dir_all(&td).expect("temp dir");
    let identity_path = td.join("adapter.key");
    let fixture_path = td.join("bootstrap.yaml");
    let enc_path = td.join("bootstrap.enc.yaml");

    // 1. ephemeral identity OUTSIDE the repo (directive M).
    let out = run(Command::new(AGE_KEYGEN).arg("-o").arg(&identity_path));
    assert!(out.status.success(), "age-keygen failed");
    let identity = std::fs::read(&identity_path).expect("read identity");
    let recipient =
        String::from_utf8(run(Command::new(AGE_KEYGEN).arg("-y").arg(&identity_path)).stdout)
            .expect("recipient utf8")
            .trim()
            .to_string();

    // 2. plaintext fixture only in temp scope.
    let mut f = std::fs::File::create(&fixture_path).expect("fixture");
    writeln!(f, "db_password: {}", CANARY).expect("write fixture");

    // 3. encrypt -> encrypted document; canary must not appear.
    let out = run(Command::new(SOPS)
        .args([
            "--encrypt",
            "--age",
            &recipient,
            "--input-type",
            "yaml",
            "--output-type",
            "yaml",
            "--output",
        ])
        .arg(&enc_path)
        .arg(&fixture_path));
    assert!(
        out.status.success(),
        "sops encrypt failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let encrypted = std::fs::read_to_string(&enc_path).expect("encrypted doc");
    assert!(
        !encrypted.contains(CANARY),
        "plaintext leaked into encrypted document"
    );

    let bundle = BootstrapBundle::new(
        enc_path.to_str().unwrap(),
        SecretReference::new("age", "adapter.key", None).unwrap(),
        vec![SecretReference::new("sops", "db_password", None).unwrap()],
    )
    .unwrap();

    // 4. correct identity decrypts through the REAL adapter.
    let store = SopsBootstrapStore::new(identity.clone(), SOPS, AGE);
    let refs = store.load(&bundle).expect("bootstrap load");
    assert_eq!(refs.len(), 1);
    let value = store.get(&bundle, &refs[0]).expect("bootstrap get");
    assert_eq!(String::from_utf8(value).unwrap(), CANARY);

    // 5. wrong identity fails typed.
    let wrong_path = td.join("wrong.key");
    run(Command::new(AGE_KEYGEN).arg("-o").arg(&wrong_path));
    let wrong_identity = std::fs::read(&wrong_path).expect("wrong identity");
    let wrong_store = SopsBootstrapStore::new(wrong_identity, SOPS, AGE);
    let err = wrong_store
        .get(&bundle, &refs[0])
        .expect_err("wrong identity must fail");
    assert_eq!(err.code, nexus_trust::TrustErrorCode::ProviderAuthorization);

    // 6. corrupted document fails typed.
    let mut bytes = std::fs::read(&enc_path).expect("encrypted bytes");
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0x01;
    let corrupt_path = td.join("corrupt.enc.yaml");
    std::fs::write(&corrupt_path, bytes).expect("corrupt doc");
    let corrupt_bundle = BootstrapBundle::new(
        corrupt_path.to_str().unwrap(),
        SecretReference::new("age", "adapter.key", None).unwrap(),
        vec![SecretReference::new("sops", "db_password", None).unwrap()],
    )
    .unwrap();
    let err = store
        .get(&corrupt_bundle, &refs[0])
        .expect_err("corrupt must fail");
    assert_eq!(
        err.code,
        nexus_trust::TrustErrorCode::MalformedProviderResponse
    );

    // 7. missing file fails typed.
    let missing_bundle = BootstrapBundle::new(
        td.join("missing.enc.yaml").to_str().unwrap(),
        SecretReference::new("age", "adapter.key", None).unwrap(),
        vec![SecretReference::new("sops", "db_password", None).unwrap()],
    )
    .unwrap();
    let err = store
        .get(&missing_bundle, &refs[0])
        .expect_err("missing must fail");
    assert_eq!(err.code, nexus_trust::TrustErrorCode::NotFound);

    // 8. teardown: identity + plaintext removed.
    std::fs::remove_dir_all(&td).expect("temp cleanup");
    assert!(!td.exists(), "temp dir must be removed");

    println!("EP-009 M2 SOPS adapter live proof: ok (canary={})", CANARY);
}
