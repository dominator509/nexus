//! Evidence fusion engine (EP-003 M2).
//!
//! Combines voice, room, BLE, mobile, and camera evidence into a single
//! `IdentityConfidence`. The engine is deterministic and provider-neutral.
//! INV-003: the result is evidence, never cryptographic authentication; the
//! engine cannot produce an authorization token.

use nexus_identity::{ConfidenceLevel, EvidenceKind, IdentityConfidence, PresenceEvidence};

use crate::error::PresenceError;

/// How old (seconds) an observation may be before it is too stale to fuse.
pub const MAX_EVIDENCE_AGE_SECS: i64 = 300;

/// Single-source cap: evidence from one kind alone can never reach HIGH.
///
/// This encodes the acceptance obligation that multiple evidence kinds
/// combine; a lone camera or lone BLE observation is suggestive, not
/// conclusive. The cap keeps a single spoofed or noisy source from
/// fabricating high confidence.
pub const SINGLE_SOURCE_CAP: f64 = 0.6;

/// Deterministic presence fusion engine.
#[derive(Debug, Clone, Default)]
pub struct PresenceFusionEngine {
    /// Maximum age of an observation in seconds.
    pub max_evidence_age_secs: i64,
}

impl PresenceFusionEngine {
    /// Create the engine with default bounds.
    pub fn new() -> Self {
        Self {
            max_evidence_age_secs: MAX_EVIDENCE_AGE_SECS,
        }
    }

    /// Fuse evidence observed by one interaction into confidence.
    ///
    /// Steps (deterministic):
    /// 1. Reject an empty set (fail closed; never fabricate presence).
    /// 2. Drop evidence older than `max_evidence_age_secs`.
    /// 3. If nothing fresh remains, fail closed with `StaleEvidence`.
    /// 4. Fuse with recency weighting (see `IdentityConfidence::fuse`).
    /// 5. Apply the single-source cap when only one evidence kind is fresh.
    ///
    /// The result is evidence, never authentication: `is_authoritative()`
    /// is always false.
    pub fn fuse(
        &self,
        evidence: Vec<PresenceEvidence>,
        now_unix_s: i64,
    ) -> Result<IdentityConfidence, PresenceError> {
        if evidence.is_empty() {
            return Err(PresenceError::Validation(
                "cannot fuse an empty evidence set".into(),
            ));
        }
        let fresh: Vec<PresenceEvidence> = evidence
            .into_iter()
            .filter(|e| now_unix_s - e.observed_at_unix_s <= self.max_evidence_age_secs)
            .collect();
        if fresh.is_empty() {
            return Err(PresenceError::StaleEvidence(
                "no fresh presence evidence available".into(),
            ));
        }
        let mut kinds: Vec<EvidenceKind> = fresh.iter().map(|e| e.kind).collect();
        kinds.sort_by_key(|k| k.as_str());
        kinds.dedup();
        let mut confidence = IdentityConfidence::fuse(fresh, now_unix_s)
            .map_err(|e| PresenceError::Validation(e.to_string()))?;
        if kinds.len() == 1 {
            confidence.score = confidence.score.min(SINGLE_SOURCE_CAP);
            confidence.level = ConfidenceLevel::from_score(confidence.score);
        }
        Ok(confidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::DeviceId;
    use nexus_identity::EvidenceKind;

    const DID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105";
    const NOW: i64 = 1_700_000_000;

    fn ev(kind: EvidenceKind, confidence: f64, age_s: i64) -> PresenceEvidence {
        PresenceEvidence::new(
            kind,
            Some(DeviceId::new(DID).unwrap()),
            confidence,
            NOW - age_s,
        )
        .unwrap()
    }

    #[test]
    fn ep003_unit_fusion_rejects_empty_evidence() {
        let engine = PresenceFusionEngine::new();
        let res = engine.fuse(vec![], NOW);
        assert_eq!(res.err().map(|e| e.code()), Some("validation"));
    }

    #[test]
    fn ep003_unit_fusion_fails_closed_on_stale_evidence() {
        let engine = PresenceFusionEngine::new();
        let stale = vec![ev(EvidenceKind::Room, 0.9, MAX_EVIDENCE_AGE_SECS + 1)];
        let res = engine.fuse(stale, NOW);
        assert_eq!(res.err().map(|e| e.code()), Some("unavailable"));
    }

    #[test]
    fn ep003_unit_fusion_single_source_is_capped() {
        let engine = PresenceFusionEngine::new();
        // One camera at 1.0 would fuse to 1.0; the cap holds it to 0.6.
        let fused = engine
            .fuse(vec![ev(EvidenceKind::Camera, 1.0, 0)], NOW)
            .unwrap();
        assert_eq!(fused.score, SINGLE_SOURCE_CAP);
        assert_eq!(fused.level, ConfidenceLevel::Medium);
        assert!(!fused.is_authoritative());
    }

    #[test]
    fn ep003_unit_fusion_multi_kind_can_reach_high() {
        let engine = PresenceFusionEngine::new();
        // Voice + BLE + camera at 1.0 fuse to 1.0; no cap applies.
        let fused = engine
            .fuse(
                vec![
                    ev(EvidenceKind::Voice, 1.0, 0),
                    ev(EvidenceKind::Ble, 1.0, 0),
                    ev(EvidenceKind::Camera, 1.0, 0),
                ],
                NOW,
            )
            .unwrap();
        assert_eq!(fused.level, ConfidenceLevel::High);
        assert!((fused.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ep003_unit_fusion_recency_weighting_is_deterministic() {
        let engine = PresenceFusionEngine::new();
        let a = engine
            .fuse(
                vec![
                    ev(EvidenceKind::Voice, 1.0, 0),
                    ev(EvidenceKind::Room, 0.0, 0),
                ],
                NOW,
            )
            .unwrap();
        let b = engine
            .fuse(
                vec![
                    ev(EvidenceKind::Voice, 1.0, 0),
                    ev(EvidenceKind::Room, 0.0, 0),
                ],
                NOW,
            )
            .unwrap();
        assert_eq!(a, b);
    }
}
