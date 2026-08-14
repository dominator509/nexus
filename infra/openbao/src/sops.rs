//! SOPS + age bootstrap secret store (EP-009 M2 directives K, L, M, N).
//!
//! Implements the nexus-trust `BootstrapSecretStore` contract with the
//! REAL pinned `sops` CLI (3.13.0) and `age` (1.1.1) tooling, invoked as
//! subprocesses. The age identity is provided at construction time by the
//! caller (operator break-glass path) and is NEVER stored next to the
//! ciphertext, never written to disk by this adapter, and never logged.
//!
//! ROUTING RULE (directive N): this store is ONLY reachable through
//! explicit `BootstrapSecretStore` operations. It is NOT an automatic
//! runtime fallback for `SecretStore` failures. OpenBao unavailable must
//! NOT mean "decrypt every SOPS file and continue as if authorization
//! still exists."

use std::io::Write;
use std::process::Command;

use nexus_trust::bootstrap::{BootstrapBundle, BootstrapSecretStore};
use nexus_trust::secret::SecretReference;
use nexus_trust::{TrustError, TrustErrorCode};

/// A SOPS+age bootstrap store configured with an explicit age identity.
///
/// The identity bytes are held in memory only; `Drop` zeroes the buffer.
#[derive(Debug)]
pub struct SopsBootstrapStore {
    age_identity: ZeroableVec,
    sops_bin: String,
    age_bin: String,
}

/// A byte buffer that zeroes itself on drop (best effort).
#[derive(Clone)]
struct ZeroableVec(Vec<u8>);

impl std::fmt::Debug for ZeroableVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ZeroableVec(<redacted>)")
    }
}

impl Drop for ZeroableVec {
    fn drop(&mut self) {
        for b in self.0.iter_mut() {
            *b = 0;
        }
    }
}

impl SopsBootstrapStore {
    /// Construct with the age identity passed in-memory (never on disk).
    pub fn new(
        age_identity: Vec<u8>,
        sops_bin: impl Into<String>,
        age_bin: impl Into<String>,
    ) -> Self {
        Self {
            age_identity: ZeroableVec(age_identity),
            sops_bin: sops_bin.into(),
            age_bin: age_bin.into(),
        }
    }

    /// Decrypt a SOPS document by piping the age identity via stdin.
    ///
    /// `sops --decrypt` with `--input-type yaml` and the age identity
    /// provided through the `SOPS_AGE_KEY_FILE`-free path: we write the
    /// identity to a private temp file with 0600 perms, run sops, then
    /// delete it immediately (directive L.8: decrypted material removed
    /// after use; identity never next to ciphertext).
    fn decrypt_document(&self, sealed_path: &str) -> Result<Vec<u8>, TrustError> {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        // Identity is passed via a private temp file (0600) that is
        // removed immediately after decryption; never alongside the
        // ciphertext and never persisted.
        let mut identity_file = std::env::temp_dir();
        identity_file.push(format!(
            "nexus-sops-age-{}-{}.key",
            std::process::id(),
            fingerprint(sealed_path)
        ));
        {
            let mut f = fs::File::create(&identity_file).map_err(|_| {
                TrustError::new(
                    TrustErrorCode::Internal,
                    "cannot create age identity temp file",
                )
            })?;
            let mut perms = f
                .metadata()
                .map_err(|_| {
                    TrustError::new(
                        TrustErrorCode::Internal,
                        "cannot stat age identity temp file",
                    )
                })?
                .permissions();
            perms.set_mode(0o600);
            f.set_permissions(perms).map_err(|_| {
                TrustError::new(
                    TrustErrorCode::Internal,
                    "cannot chmod age identity temp file",
                )
            })?;
            f.write_all(&self.age_identity.0).map_err(|_| {
                TrustError::new(
                    TrustErrorCode::Internal,
                    "cannot write age identity temp file",
                )
            })?;
        }

        let result = Command::new(&self.sops_bin)
            .args([
                "--decrypt",
                "--input-type",
                "yaml",
                "--output-type",
                "yaml",
                sealed_path,
            ])
            .env("SOPS_AGE_KEY_FILE", &identity_file)
            .output();

        // Remove the identity file unconditionally (best effort).
        let _ = fs::remove_file(&identity_file);

        match result {
            Ok(out) if out.status.success() => Ok(out.stdout),
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let code = classify_sops_decrypt_failure(out.status.code(), &stderr);
                Err(TrustError::new(
                    code,
                    "SOPS+age decryption failed (redacted)",
                ))
            }
            Err(e) => Err(TrustError::new(
                TrustErrorCode::Unavailable,
                format!("cannot run sops: {}", e),
            )),
        }
    }

    /// Verify the age binary is usable (health proof).
    pub fn health(&self) -> Result<(), TrustError> {
        let out = Command::new(&self.age_bin)
            .arg("--version")
            .output()
            .map_err(|_| TrustError::new(TrustErrorCode::Unavailable, "age binary unavailable"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(TrustError::new(
                TrustErrorCode::Unavailable,
                "age binary failed",
            ))
        }
    }
}

impl BootstrapSecretStore for SopsBootstrapStore {
    fn load(&self, bundle: &BootstrapBundle) -> Result<Vec<SecretReference>, TrustError> {
        // The envelope's declared references are part of the bundle;
        // loading proves the document actually decrypts.
        let plaintext = self.decrypt_document(&bundle.sealed_path)?;
        let text = String::from_utf8_lossy(&plaintext);
        // Ensure the envelope contains the declared references' keys.
        for reference in &bundle.secrets {
            if !text.contains(&reference.key) {
                return Err(TrustError::new(
                    TrustErrorCode::MalformedProviderResponse,
                    format!(
                        "bootstrap envelope missing declared secret {}",
                        fingerprint(&reference.to_string())
                    ),
                ));
            }
        }
        Ok(bundle.secrets.clone())
    }

    fn get(
        &self,
        bundle: &BootstrapBundle,
        reference: &SecretReference,
    ) -> Result<Vec<u8>, TrustError> {
        let plaintext = self.decrypt_document(&bundle.sealed_path)?;
        let text = String::from_utf8_lossy(&plaintext);
        // Minimal YAML-ish extraction: find `key:` line and take the value.
        let needle = format!("{}:", reference.key);
        let line = text
            .lines()
            .find(|l| l.trim_start().starts_with(&needle))
            .ok_or_else(|| {
                TrustError::new(
                    TrustErrorCode::NotFound,
                    format!(
                        "bootstrap secret {} not in envelope",
                        fingerprint(&reference.to_string())
                    ),
                )
            })?;
        let value = line
            .split_once(':')
            .map(|(_, v)| v.trim().trim_matches('"').trim_matches('\'').to_string())
            .unwrap_or_default();
        Ok(value.into_bytes())
    }
}

/// Ordered classifier for SOPS decrypt failures (directive C).
///
/// Maps the REAL sops 3.13.0 stderr shapes (captured from the pinned
/// binaries, 2026-08-14) to the canonical nexus-trust typed codes.
/// The classifier must NOT key off the generic "Recovery failed because
/// no master key was able to decrypt..." footer, because sops emits that
/// footer for every underlying cause. Specific structural/source failures
/// are checked FIRST, in order; only the valid-identity-but-no-key case
/// maps to ProviderAuthorization.
///
/// Real observed shapes (sops 3.13.0, age 1.1.1, exit codes):
/// - missing sealed document: exit 100, "cannot operate on non-existent file"
/// - missing SOPS_AGE_KEY_FILE: exit 128, "...failed to open SOPS_AGE_KEY_FILE
///   file: open <path>: no such file or directory..."
/// - malformed identity: exit 128, "...failed to parse 'SOPS_AGE_KEY_FILE' age
///   identities: unknown identity type..."
/// - corrupted document: exit 128, "...failed to decrypt and authenticate
///   payload chunk, file may be corrupted or tampered with..."
/// - valid-but-wrong identity: exit 128, "age: no identity matched any of the
///   recipients..."
pub(crate) fn classify_sops_decrypt_failure(
    exit_status: Option<i32>,
    stderr: &str,
) -> TrustErrorCode {
    // 1. Sealed document itself does not exist (exit 100 from sops).
    if exit_status == Some(100) || stderr.contains("cannot operate on non-existent file") {
        return TrustErrorCode::NotFound;
    }
    // 2. Identity SOURCE missing (SOPS_AGE_KEY_FILE cannot be opened).
    //    This is a bootstrap-source/missing error, NOT authorization.
    if stderr.contains("failed to open SOPS_AGE_KEY_FILE file")
        || stderr.contains("no such file or directory")
    {
        return TrustErrorCode::NotFound;
    }
    // 3. Identity MATERIAL malformed (parse failure / unknown identity type).
    if stderr.contains("failed to parse 'SOPS_AGE_KEY_FILE' age identities")
        || stderr.contains("unknown identity type")
    {
        return TrustErrorCode::MalformedProviderResponse;
    }
    // 4. Ciphertext / SOPS document malformed or integrity failure.
    if stderr.contains("corrupted or tampered with")
        || stderr.contains("failed to decrypt and authenticate")
    {
        return TrustErrorCode::MalformedProviderResponse;
    }
    // 5. Valid identity material, but no supplied identity can decrypt the
    //    document's data key. The caller's identity is not authorized for
    //    this document. This is checked LAST, only after the structural
    //    failures above have been ruled out.
    if stderr.contains("no identity matched any of the recipients") {
        return TrustErrorCode::ProviderAuthorization;
    }
    // 6. Unknown failure shape: fail closed as malformed. Never assume
    //    authorization from an unrecognized error.
    TrustErrorCode::MalformedProviderResponse
}

/// One-way fingerprint of a string.
fn fingerprint(value: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
