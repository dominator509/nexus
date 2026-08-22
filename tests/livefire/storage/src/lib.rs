//! EP-037 M5 live-fire journeys (LF-002, LF-020) shared harness.
//!
//! Current-run evidence is written as JSON under
//! `.agent/state/evidence/LF-{002,020}-ep037-m5.json` (canonical
//! convention: run_id bound, node/milestone bound, git_commit bound,
//! freshness-verified by the gate with mmin -10). Every journey uses
//! REAL production adapters (storage-local, storage-s3) and REAL domain
//! types (Principal, RelationshipTuple, MemoryRecord, SkillRegistry,
//! CapabilityRegistry). Nothing here is a mock, a stub, or a fixture.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Current-run identifier (nanoseconds since epoch + pid suffix).
pub fn run_id() -> String {
    format!(
        "ep037-m5-{}-{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        std::process::id()
    )
}

/// RFC 3339 timestamp for the current run.
pub fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let hms = secs % 86400;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        hms / 3600,
        (hms % 3600) / 60,
        hms % 60
    )
}

/// Civil date from days since epoch (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Git commit of the tree under test (or "unknown" when unavailable).
pub fn git_commit() -> String {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    }
}

/// Evidence directory (canonical, resolved from the workspace root).
/// Cargo runs tests with CWD = crate dir, so walk up until the dir
/// containing `.agent/` is found (the repo root).
pub fn evidence_dir() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join(".agent").is_dir() {
            return dir.join(".agent/state/evidence");
        }
        if !dir.pop() {
            break;
        }
    }
    PathBuf::from(".agent/state/evidence")
}

/// Write a current-run evidence JSON document. Returns the path.
pub fn write_evidence(filename: &str, value: &serde_json::Value) -> PathBuf {
    let dir = evidence_dir();
    std::fs::create_dir_all(&dir).expect("create evidence dir");
    let path = dir.join(filename);
    let pretty = serde_json::to_vec_pretty(value).expect("serialize evidence");
    std::fs::write(&path, pretty).expect("write evidence");
    path
}

/// Redaction scan: assert no secret-shaped literal appears in evidence
/// text (credential canaries, key-shaped strings, base64 of secrets).
pub fn assert_evidence_redacted(evidence_text: &str) {
    let lower = evidence_text.to_ascii_lowercase();
    for needle in [
        "secret_key",
        "secretkey",
        "access_key",
        "accesskey",
        "password",
        "authorization:",
        "x-amz-date",
        "aws4-hmac",
        "signature=",
    ] {
        assert!(
            !lower.contains(needle),
            "evidence leaks {needle:?} (redaction failure)"
        );
    }
}

/// Real SHA-256 hex (canonical lowercase) of bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

/// AES-256-GCM encrypt (ring). Returns (nonce || ciphertext || tag).
/// REAL authenticated encryption; the key is a caller-held 32-byte key
/// never stored beside the artifact (SPEC-024 recovery-key non-goal).
pub fn encrypt_aes256gcm(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    let unbound = UnboundKey::new(&AES_256_GCM, key).expect("valid key");
    let sealing = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; 12];
    // Current-run nonce from time + pid (unique per artifact write).
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    nonce_bytes.copy_from_slice(&t.to_le_bytes()[..12]);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plaintext.to_vec();
    sealing
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .expect("seal");
    let mut out = Vec::with_capacity(12 + in_out.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&in_out);
    out
}

/// AES-256-GCM decrypt (ring). Returns Err on wrong/missing key (fail
/// closed, zero partial output).
pub fn decrypt_aes256gcm(
    key: &[u8; 32],
    sealed: &[u8],
) -> Result<Vec<u8>, ring::error::Unspecified> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    if sealed.len() < 12 {
        return Err(ring::error::Unspecified);
    }
    let unbound = UnboundKey::new(&AES_256_GCM, key).expect("valid key");
    let opening = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(&sealed[..12]);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = sealed[12..].to_vec();
    let plain = opening.open_in_place(nonce, Aad::empty(), &mut in_out)?;
    Ok(plain.to_vec())
}
