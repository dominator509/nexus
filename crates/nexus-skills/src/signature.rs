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
