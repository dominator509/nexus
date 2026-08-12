//! Presence evidence and identity confidence (SPEC-005, EP-003).
//!
//! INV-003: voice, room, BLE, mobile, and camera evidence combine into
//! confidence. Confidence is evidence, never cryptographic authentication.
//! No type here can authorize an R3/R4 action by itself.

use std::fmt;

use nexus_domain::DeviceId;
use serde::{Deserialize, Serialize};

/// Kind of presence evidence (EP-003 acceptance obligation 2; ADR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceKind {
    Voice,
    Room,
    Ble,
    Mobile,
    Camera,
}

impl EvidenceKind {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Voice => "VOICE",
            Self::Room => "ROOM",
            Self::Ble => "BLE",
            Self::Mobile => "MOBILE",
            Self::Camera => "CAMERA",
        }
    }
}

impl fmt::Display for EvidenceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Deterministic confidence band (ADR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConfidenceLevel {
    Low,
    Medium,
    High,
}

impl ConfidenceLevel {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
        }
    }

    /// Classify a fused score in `0.0..=1.0`.
    ///
    /// Deterministic bands: `[0, 0.5)` LOW, `[0.5, 0.8)` MEDIUM,
    /// `[0.8, 1.0]` HIGH.
    pub fn from_score(score: f64) -> Self {
        if score >= 0.8 {
            Self::High
        } else if score >= 0.5 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

impl fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned by presence evidence construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceError {
    /// The confidence score is outside `0.0..=1.0`.
    ScoreOutOfRange,
    /// An empty evidence set cannot be fused.
    EmptyEvidence,
}

impl fmt::Display for PresenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScoreOutOfRange => f.write_str("evidence confidence must be in 0.0..=1.0"),
            Self::EmptyEvidence => f.write_str("cannot fuse an empty evidence set"),
        }
    }
}

impl std::error::Error for PresenceError {}

/// A single observed piece of presence evidence.
///
/// Evidence is an observation from a source; it carries a confidence score
/// assigned by the source and a timestamp. It never grants authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresenceEvidence {
    /// Kind of evidence.
    pub kind: EvidenceKind,
    /// Device that observed the evidence, when known.
    pub source_device_id: Option<DeviceId>,
    /// Source-assigned confidence in `0.0..=1.0`.
    pub confidence: f64,
    /// Unix timestamp in seconds when observed.
    pub observed_at_unix_s: i64,
}

impl PresenceEvidence {
    /// Construct validated evidence.
    pub fn new(
        kind: EvidenceKind,
        source_device_id: Option<DeviceId>,
        confidence: f64,
        observed_at_unix_s: i64,
    ) -> Result<Self, PresenceError> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(PresenceError::ScoreOutOfRange);
        }
        Ok(Self {
            kind,
            source_device_id,
            confidence,
            observed_at_unix_s,
        })
    }
}

/// Fused identity confidence from a set of presence evidence.
///
/// The fused score is the recency-weighted mean of the evidence confidence
/// values. This is a deterministic, provider-neutral combination; it is
/// evidence about "is this person plausibly here", never authentication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdentityConfidence {
    /// Fused score in `0.0..=1.0`.
    pub score: f64,
    /// Deterministic band for the fused score.
    pub level: ConfidenceLevel,
    /// Evidence that contributed to this confidence.
    pub evidence: Vec<PresenceEvidence>,
    /// Unix timestamp in seconds when fused.
    pub fused_at_unix_s: i64,
}

impl IdentityConfidence {
    /// Fuse evidence into a single confidence score.
    ///
    /// Uses recency weighting: evidence observed more recently counts more.
    /// The formula is deterministic: each piece is weighted `1/(1 + age)`
    /// where `age` is seconds since the newest observation, then normalized.
    /// An empty set is an error (fail closed, never fabricate presence).
    pub fn fuse(evidence: Vec<PresenceEvidence>, now_unix_s: i64) -> Result<Self, PresenceError> {
        if evidence.is_empty() {
            return Err(PresenceError::EmptyEvidence);
        }
        let newest = evidence
            .iter()
            .map(|e| e.observed_at_unix_s)
            .max()
            .unwrap_or(now_unix_s);
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;
        for e in &evidence {
            let age = (newest - e.observed_at_unix_s).max(0) as f64;
            let weight = 1.0 / (1.0 + age);
            weighted_sum += weight * e.confidence;
            weight_sum += weight;
        }
        let score = if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            0.0
        };
        Ok(Self {
            score,
            level: ConfidenceLevel::from_score(score),
            evidence,
            fused_at_unix_s: now_unix_s,
        })
    }

    /// Whether this confidence alone can authorize an action.
    ///
    /// Always false: presence evidence is never cryptographic
    /// authentication (INV-003, SPEC-005 behavior 3). Higher-level policy
    /// layers may use it as one input, but this type never grants authority.
    pub fn is_authoritative(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::DeviceId;

    const DID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105";

    fn ev(kind: EvidenceKind, confidence: f64, age_s: i64, now: i64) -> PresenceEvidence {
        PresenceEvidence::new(
            kind,
            Some(DeviceId::new(DID).unwrap()),
            confidence,
            now - age_s,
        )
        .unwrap()
    }

    #[test]
    fn ep003_unit_evidence_constructs_and_validates() {
        let e = PresenceEvidence::new(EvidenceKind::Voice, None, 0.9, 1_700_000_000).unwrap();
        assert_eq!(e.kind.as_str(), "VOICE");
        assert!(PresenceEvidence::new(EvidenceKind::Room, None, 1.5, 0).is_err());
        assert!(PresenceEvidence::new(EvidenceKind::Room, None, -0.1, 0).is_err());
    }

    #[test]
    fn ep003_unit_confidence_fuses_recency_weighted() {
        let now = 1_700_000_000;
        // Two identical fresh pieces of 1.0 and 0.0 -> 0.5 (MEDIUM).
        let fused = IdentityConfidence::fuse(
            vec![
                ev(EvidenceKind::Voice, 1.0, 0, now),
                ev(EvidenceKind::Room, 0.0, 0, now),
            ],
            now,
        )
        .unwrap();
        assert!((fused.score - 0.5).abs() < 1e-9);
        assert_eq!(fused.level, ConfidenceLevel::Medium);
    }

    #[test]
    fn ep003_unit_confidence_bands_are_deterministic() {
        assert_eq!(ConfidenceLevel::from_score(0.0), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.49), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.5), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.79), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.8), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(1.0), ConfidenceLevel::High);
    }

    #[test]
    fn ep003_unit_confidence_rejects_empty_evidence() {
        let res = IdentityConfidence::fuse(vec![], 1_700_000_000);
        assert_eq!(res, Err(PresenceError::EmptyEvidence));
    }

    #[test]
    fn ep003_unit_confidence_is_never_authoritative() {
        let now = 1_700_000_000;
        let fused =
            IdentityConfidence::fuse(vec![ev(EvidenceKind::Camera, 1.0, 0, now)], now).unwrap();
        // INV-003: even maximal evidence is not authentication.
        assert_eq!(fused.level, ConfidenceLevel::High);
        assert!(!fused.is_authoritative());
    }

    #[test]
    fn ep003_unit_confidence_serde_roundtrip() {
        let now = 1_700_000_000;
        let fused =
            IdentityConfidence::fuse(vec![ev(EvidenceKind::Ble, 0.9, 0, now)], now).unwrap();
        let json = serde_json::to_string(&fused).unwrap();
        assert!(json.contains("\"level\":\"HIGH\""));
        let back: IdentityConfidence = serde_json::from_str(&json).unwrap();
        assert_eq!(back, fused);
    }
}
