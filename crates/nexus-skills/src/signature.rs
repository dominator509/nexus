//! EP-018 skill signature (SPEC-010 behavior 6; ADR-025).
//!
//! A skill package is signed; the signature covers the manifest and
//! the content hash. Signature validation is format-level at the M1
//! contract boundary (hex-encoded key/signature with the declared
//! algorithm); cryptographic verification is owned by the M2/M3
//! behavior boundary and the real scan-before-install proof.
//!
//! Semantic distinction (ADR-025): a valid signature is an
//! integrity/authenticity statement. It is NOT trust, NOT an
//! authorized installation, and NOT execution permission. The presence
//! of a signature never sets `SkillTrustLevel`; trust is a separate,
//! independently governed input to authorization.

use crate::manifest::{is_hex_encoded, SkillPackageError};
use crate::vocabulary::SignatureAlgorithm;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The signature of a skill package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSignature {
    pub algorithm: SignatureAlgorithm,
    /// Hex-encoded public key fingerprint (32 or 33 bytes -> 64/66 hex
    /// chars; kept as hex string for portability).
    pub public_key_hex: String,
    /// Hex-encoded signature over the canonical manifest + content
    /// hash.
    pub signature_hex: String,
    /// Signer identity (optional; verified against the key).
    pub signer: Option<String>,
}

impl SkillSignature {
    /// Structural validation (M1 contract): non-empty hex-encoded
    /// key/signature with algorithm-appropriate lengths. This proves
    /// well-formedness, never authenticity; cryptographic verification
    /// is owned by the M2/M3 behavior boundary.
    pub fn validate(&self) -> Result<(), SkillPackageError> {
        if self.public_key_hex.is_empty() {
            return Err(SkillPackageError::validation(
                "signature public key must not be empty",
                Some("skill-signature".into()),
            ));
        }
        if !is_hex_encoded(&self.public_key_hex) {
            return Err(SkillPackageError::validation(
                "signature public key must be hex-encoded",
                Some("skill-signature".into()),
            ));
        }
        let key_len = match self.algorithm {
            SignatureAlgorithm::Ed25519 => 64,
            SignatureAlgorithm::EcdsaP256 => 66,
        };
        if self.public_key_hex.len() != key_len {
            return Err(SkillPackageError::validation(
                "signature public key length does not match algorithm",
                Some("skill-signature".into()),
            ));
        }
        if self.signature_hex.is_empty() {
            return Err(SkillPackageError::validation(
                "signature value must not be empty",
                Some("skill-signature".into()),
            ));
        }
        if !is_hex_encoded(&self.signature_hex) || self.signature_hex.len() != 128 {
            return Err(SkillPackageError::validation(
                "signature value must be 128 hex chars (64-byte signature)",
                Some("skill-signature".into()),
            ));
        }
        Ok(())
    }
}

impl fmt::Display for SkillSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} key={} sig={}",
            self.algorithm.as_str(),
            self.public_key_hex,
            self.signature_hex
        )
    }
}

// ---------------------------------------------------------------------------
// REAL cryptographic signature verification (EP-018 M5 / LF-018)
// ---------------------------------------------------------------------------
//
// The M1 contract boundary validates signature STRUCTURE (hex, lengths).
// Cryptographic verification is real and pinned: ring 0.17 Ed25519, the
// same locked workspace dependency class rustls/rcgen already use. A
// valid cryptographic signature is an integrity/authenticity statement
// only (ADR-025): it is NOT trust, NOT an authorized installation, and
// NOT execution permission.

/// Hex-decode a string into bytes (pure std, fails closed on
/// malformed input). Only used for signature material, never content.
pub fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if !is_hex_encoded(value) {
        return None;
    }
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

/// Real Ed25519 verification (ring 0.17). Returns `Ok(())` when the
/// signature verifies against the public key for the message.
pub fn verify_ed25519(
    public_key_hex: &str,
    signature_hex: &str,
    message: &[u8],
) -> Result<(), SkillPackageError> {
    let public_key = decode_hex(public_key_hex).ok_or_else(|| {
        SkillPackageError::verification(
            "signature public key is not valid hex",
            Some("skill-signature".into()),
        )
    })?;
    let signature = decode_hex(signature_hex).ok_or_else(|| {
        SkillPackageError::verification(
            "signature value is not valid hex",
            Some("skill-signature".into()),
        )
    })?;
    use ring::signature::{UnparsedPublicKey, ED25519};
    let key = UnparsedPublicKey::new(&ED25519, &public_key);
    key.verify(message, &signature).map_err(|_| {
        SkillPackageError::verification(
            "skill signature verification failed",
            Some("skill-signature".into()),
        )
    })
}

/// Generate a fresh Ed25519 keypair and sign `message`. Returns
/// `(public_key_hex, signature_hex)`. Used by the Skill Factory and by
/// the LF-018 live-fire proof; real ring crypto, never hand-rolled.
pub fn sign_ed25519(message: &[u8]) -> Result<(String, String), SkillPackageError> {
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|_| {
        SkillPackageError::unavailable(
            "cannot generate ed25519 keypair",
            Some("skill-signature".into()),
        )
    })?;
    let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).map_err(|_| {
        SkillPackageError::unavailable(
            "cannot load generated ed25519 keypair",
            Some("skill-signature".into()),
        )
    })?;
    let public_hex = key_pair
        .public_key()
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let signature_hex = key_pair
        .sign(message)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok((public_hex, signature_hex))
}

/// The canonical message a signature covers: the immutable package
/// identity bytes (`name@version:content_hash`, ADR-025). Signing this
/// digest binds the signature to the exact immutable version+content.
pub fn package_signing_message(package: &crate::manifest::SkillPackage) -> Vec<u8> {
    package.canonical_identity().into_bytes()
}

impl SkillSignature {
    /// Real cryptographic verification of this signature over the
    /// package's canonical identity digest. Structural validity is
    /// required first (M1), then real ring Ed25519 verification.
    pub fn verify_cryptographic(
        &self,
        package: &crate::manifest::SkillPackage,
    ) -> Result<(), SkillPackageError> {
        self.validate()?;
        match self.algorithm {
            SignatureAlgorithm::Ed25519 => verify_ed25519(
                &self.public_key_hex,
                &self.signature_hex,
                &package_signing_message(package),
            ),
            SignatureAlgorithm::EcdsaP256 => Err(SkillPackageError::verification(
                "ECDSA_P256 cryptographic verification unavailable (Ed25519 only)",
                Some("skill-signature".into()),
            )),
        }
    }
}
