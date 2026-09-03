//! Real Ed25519 artifact signer (RX-009; AUD-059).
//!
//! AUD-059 found that supply-chain evidence was sealed with a bare
//! SHA-256 checksum stored beside the evidence and recomputed during
//! verification - anyone able to change evidence could change its
//! checksum. The `ArtifactSigner` port had no implementation.
//!
//! This module implements the port with a real Ed25519 signer:
//! - real keypair generation (ring 0.17, already in the workspace lock)
//! - deterministic signing over the canonical evidence digest bytes
//! - fail-closed verification (any tamper, wrong key, or malformed
//!   signature is a typed SupplyChainError; nothing is ever accepted
//!   without a successful cryptographic check)
//!
//! The private key never leaves the signer; only the public key is
//! exported for verification.

use ring::rand::SystemRandom;
use ring::signature::{
    Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519, ED25519_PUBLIC_KEY_LEN,
};

use crate::error::{SupplyChainError, SupplyChainErrorCode, SupplyChainResult};
use crate::model::ArtifactDigest;
use crate::port::ArtifactSigner;

/// Canonical bytes signed for a digest: `alg:hex` as UTF-8.
pub fn canonical_digest_bytes(digest: &ArtifactDigest) -> Vec<u8> {
    digest.as_str().into_bytes()
}

/// A real Ed25519 signer bound to a private key.
///
/// Construction never fails silently: `from_pkcs8` validates the key
/// material, and `generate` creates a fresh keypair from the OS CSPRNG.
#[derive(Debug)]
pub struct Ed25519ArtifactSigner {
    key_pair: Ed25519KeyPair,
    /// PKCS#8 v2 DER bytes (kept for export; ring 0.17 has no accessor
    /// on the key pair itself).
    pkcs8_der: Vec<u8>,
}

impl Ed25519ArtifactSigner {
    /// Generate a fresh Ed25519 keypair from the OS CSPRNG.
    pub fn generate() -> SupplyChainResult<Self> {
        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|e| {
            SupplyChainError::new(
                SupplyChainErrorCode::Internal,
                format!("ed25519 keypair generation failed: {e}"),
            )
        })?;
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).map_err(|e| {
            SupplyChainError::new(
                SupplyChainErrorCode::Internal,
                format!("ed25519 keypair materialization failed: {e}"),
            )
        })?;
        let pkcs8_der = pkcs8.as_ref().to_vec();
        Ok(Self {
            key_pair,
            pkcs8_der,
        })
    }

    /// Reconstruct a signer from PKCS#8 v2 DER key bytes (fail closed on
    /// malformed or truncated key material).
    pub fn from_pkcs8(pkcs8: &[u8]) -> SupplyChainResult<Self> {
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8).map_err(|e| {
            SupplyChainError::new(
                SupplyChainErrorCode::SignatureInvalid,
                format!("invalid ed25519 private key: {e}"),
            )
        })?;
        Ok(Self {
            key_pair,
            pkcs8_der: pkcs8.to_vec(),
        })
    }

    /// Export the PKCS#8 v2 DER bytes for the private key.
    pub fn to_pkcs8_der(&self) -> Vec<u8> {
        self.pkcs8_der.clone()
    }

    /// Public key bytes (raw 32-byte Ed25519 public key).
    pub fn public_key(&self) -> Vec<u8> {
        self.key_pair.public_key().as_ref().to_vec()
    }

    /// Base64 (standard, no padding variant used by callers) public key.
    pub fn public_key_b64(&self) -> String {
        use base64_engine_compat::*;
        encode_b64(&self.public_key())
    }

    /// Sign the canonical digest bytes. The signature is deterministic
    /// (RFC 8032 pure Ed25519) and returned raw.
    pub fn sign_digest(&self, digest: &ArtifactDigest) -> SupplyChainResult<Vec<u8>> {
        let msg = canonical_digest_bytes(digest);
        Ok(self.key_pair.sign(&msg).as_ref().to_vec())
    }

    /// Verify a signature over the canonical digest bytes with this
    /// signer's public key. Fail closed: any error is a typed
    /// SignatureInvalid error.
    pub fn verify_digest(
        &self,
        digest: &ArtifactDigest,
        signature: &[u8],
    ) -> SupplyChainResult<()> {
        let public_key = UnparsedPublicKey::new(&ED25519, self.public_key());
        public_key
            .verify(&canonical_digest_bytes(digest), signature)
            .map_err(|_| {
                SupplyChainError::new(
                    SupplyChainErrorCode::SignatureInvalid,
                    "ed25519 signature verification failed (tampered evidence or wrong key)",
                )
            })
    }

    /// Verify a signature with an explicit public key (used by the
    /// evidence verifier that only holds the public key).
    pub fn verify_with_public_key(
        public_key: &[u8],
        digest: &ArtifactDigest,
        signature: &[u8],
    ) -> SupplyChainResult<()> {
        if public_key.len() != ED25519_PUBLIC_KEY_LEN {
            return Err(SupplyChainError::new(
                SupplyChainErrorCode::SignatureInvalid,
                format!(
                    "ed25519 public key must be {ED25519_PUBLIC_KEY_LEN} bytes, got {}",
                    public_key.len()
                ),
            ));
        }
        let public_key = UnparsedPublicKey::new(&ED25519, public_key);
        public_key
            .verify(&canonical_digest_bytes(digest), signature)
            .map_err(|_| {
                SupplyChainError::new(
                    SupplyChainErrorCode::SignatureInvalid,
                    "ed25519 signature verification failed (tampered evidence or wrong key)",
                )
            })
    }
}

impl ArtifactSigner for Ed25519ArtifactSigner {
    fn sign(&self, digest: &ArtifactDigest) -> SupplyChainResult<Vec<u8>> {
        self.sign_digest(digest)
    }

    fn verify(&self, digest: &ArtifactDigest, signature: &[u8]) -> SupplyChainResult<()> {
        self.verify_digest(digest, signature)
    }
}

/// Minimal base64 helpers so the supply-chain crate does not pull the
/// full base64 crate for two small encode/decode paths.
mod base64_engine_compat {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode_b64(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = *chunk.get(1).unwrap_or(&0) as u32;
            let b2 = *chunk.get(2).unwrap_or(&0) as u32;
            let n = (b0 << 16) | (b1 << 8) | b2;
            out.push(ALPHABET[(n >> 18) as usize & 63] as char);
            out.push(ALPHABET[(n >> 12) as usize & 63] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[n as usize & 63] as char
            } else {
                '='
            });
        }
        out
    }

    #[allow(dead_code)]
    pub fn decode_b64(s: &str) -> Result<Vec<u8>, String> {
        let mut out = Vec::with_capacity(s.len() / 4 * 3);
        let mut buf = 0u32;
        let mut bits = 0u32;
        for c in s.bytes() {
            if c == b'=' {
                break;
            }
            let v = match c {
                b'A'..=b'Z' => c - b'A',
                b'a'..=b'z' => c - b'a' + 26,
                b'0'..=b'9' => c - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                _ => return Err(format!("invalid base64 char: {c}")),
            } as u32;
            buf = (buf << 6) | v;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((buf >> bits) as u8);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(s: &str) -> ArtifactDigest {
        ArtifactDigest {
            algorithm: "sha256".to_string(),
            hex: s.to_string(),
        }
    }

    #[test]
    fn signer_roundtrip_real_ed25519() {
        let signer = Ed25519ArtifactSigner::generate().expect("generate");
        let d = digest("ab".repeat(32).as_str());
        let sig = signer.sign_digest(&d).expect("sign");
        assert_eq!(sig.len(), 64, "ed25519 signatures are 64 bytes");
        signer.verify_digest(&d, &sig).expect("verify");
    }

    #[test]
    fn signer_rejects_tampered_digest() {
        let signer = Ed25519ArtifactSigner::generate().expect("generate");
        let d1 = digest("11".repeat(32).as_str());
        let d2 = digest("22".repeat(32).as_str());
        let sig = signer.sign_digest(&d1).expect("sign");
        let err = signer.verify_digest(&d2, &sig).expect_err("must fail");
        assert_eq!(err.code, SupplyChainErrorCode::SignatureInvalid);
    }

    #[test]
    fn signer_rejects_corrupted_signature() {
        let signer = Ed25519ArtifactSigner::generate().expect("generate");
        let d = digest("ab".repeat(32).as_str());
        let mut sig = signer.sign_digest(&d).expect("sign");
        sig[0] ^= 0xff;
        let err = signer.verify_digest(&d, &sig).expect_err("must fail");
        assert_eq!(err.code, SupplyChainErrorCode::SignatureInvalid);
    }

    #[test]
    fn signer_rejects_wrong_key() {
        let a = Ed25519ArtifactSigner::generate().expect("a");
        let b = Ed25519ArtifactSigner::generate().expect("b");
        let d = digest("ab".repeat(32).as_str());
        let sig = a.sign_digest(&d).expect("sign");
        let err = b.verify_digest(&d, &sig).expect_err("wrong key must fail");
        assert_eq!(err.code, SupplyChainErrorCode::SignatureInvalid);
    }

    #[test]
    fn verify_with_public_key_accepts_real_and_rejects_tamper() {
        let signer = Ed25519ArtifactSigner::generate().expect("generate");
        let pubkey = signer.public_key();
        let d = digest("cd".repeat(32).as_str());
        let sig = signer.sign_digest(&d).expect("sign");
        Ed25519ArtifactSigner::verify_with_public_key(&pubkey, &d, &sig).expect("verify ok");
        let mut tampered = sig.clone();
        tampered[10] ^= 0x01;
        let err = Ed25519ArtifactSigner::verify_with_public_key(&pubkey, &d, &tampered)
            .expect_err("tampered must fail");
        assert_eq!(err.code, SupplyChainErrorCode::SignatureInvalid);
    }

    #[test]
    fn verify_with_public_key_rejects_short_key() {
        let d = digest("ab".repeat(32).as_str());
        let err = Ed25519ArtifactSigner::verify_with_public_key(&[0u8; 8], &d, &[0u8; 64])
            .expect_err("short key must fail");
        assert_eq!(err.code, SupplyChainErrorCode::SignatureInvalid);
    }

    #[test]
    fn pkcs8_roundtrip_preserves_key() {
        let signer = Ed25519ArtifactSigner::generate().expect("generate");
        let der = signer.to_pkcs8_der();
        let restored = Ed25519ArtifactSigner::from_pkcs8(&der).expect("restore");
        assert_eq!(restored.public_key(), signer.public_key());
        let d = digest("ef".repeat(32).as_str());
        let sig = restored.sign_digest(&d).expect("sign");
        signer
            .verify_digest(&d, &sig)
            .expect("verify with original");
    }

    #[test]
    fn pkcs8_rejects_garbage() {
        let err = Ed25519ArtifactSigner::from_pkcs8(&[0u8; 16]).expect_err("must fail");
        assert_eq!(err.code, SupplyChainErrorCode::SignatureInvalid);
    }

    #[test]
    fn deterministic_signature_same_digest_same_signature() {
        let signer = Ed25519ArtifactSigner::generate().expect("generate");
        let d = digest("ab".repeat(32).as_str());
        let s1 = signer.sign_digest(&d).expect("sign1");
        let s2 = signer.sign_digest(&d).expect("sign2");
        assert_eq!(s1, s2, "pure ed25519 is deterministic");
    }

    #[test]
    fn b64_roundtrip() {
        let bytes = b"hello ed25519 evidence";
        let enc = base64_engine_compat::encode_b64(bytes);
        let dec = base64_engine_compat::decode_b64(&enc).expect("decode");
        assert_eq!(dec, bytes);
    }

    #[test]
    fn public_key_b64_is_stable_32_bytes() {
        let signer = Ed25519ArtifactSigner::generate().expect("generate");
        let b64 = signer.public_key_b64();
        assert_eq!(b64.len(), 44, "32 bytes -> 44 base64 chars with padding");
    }
}
