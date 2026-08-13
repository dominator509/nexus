//! Step-up challenge contract (SPEC-005 behavior 4; EP-007).
//!
//! R3 and R4 actions require a cryptographic step-up or explicit
//! preauthorization; R4 never accepts model approval. `StepUpChallenge`
//! is the challenge that proves the operator is present and has satisfied
//! the configured authentication strength before a high-risk action
//! proceeds.

use std::fmt;

use nexus_domain::{CorrelationId, NexusId, Risk, TenantId};
use serde::{Deserialize, Serialize};

use crate::vocabulary::{AuthenticationStrength, StepUpState};

/// Error returned by step-up challenge operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepUpError {
    /// The challenge is in the wrong state.
    WrongState,
    /// The challenge has expired.
    ChallengeExpired,
    /// The response does not satisfy the required strength.
    StrengthNotSatisfied,
    /// The response failed verification.
    VerificationFailed(String),
    /// A required field is absent or malformed.
    Malformed(String),
}

impl fmt::Display for StepUpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongState => f.write_str("step-up challenge in wrong state"),
            Self::ChallengeExpired => f.write_str("step-up challenge expired"),
            Self::StrengthNotSatisfied => f.write_str("step-up strength not satisfied"),
            Self::VerificationFailed(detail) => write!(f, "step-up verification failed: {detail}"),
            Self::Malformed(detail) => write!(f, "malformed step-up challenge: {detail}"),
        }
    }
}

impl std::error::Error for StepUpError {}

/// A step-up challenge for a high-risk action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepUpChallenge {
    /// Nexus-wide challenge identifier.
    pub challenge_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Principal being challenged.
    pub principal_id: NexusId,
    /// Risk class of the action being authorized (R3 or R4).
    pub risk: Risk,
    /// Minimum authentication strength required (STEP_UP for R3/R4).
    pub required_strength: AuthenticationStrength,
    /// Opaque challenge payload (per-issuance random; not stored plaintext
    /// after use).
    pub challenge: String,
    /// Unix seconds when the challenge was created.
    pub created_at_unix_s: i64,
    /// Unix seconds when the challenge expires.
    pub expires_at_unix_s: i64,
    /// Correlation of the challenge event.
    pub correlation: CorrelationId,
    /// Current lifecycle state.
    pub state: StepUpState,
}

impl StepUpChallenge {
    /// Construct a pending step-up challenge.
    ///
    /// The risk class and required strength are validated together:
    /// R3 and R4 must require `STEP_UP` (SPEC-005 behavior 4).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        challenge_id: NexusId,
        tenant_id: TenantId,
        principal_id: NexusId,
        risk: Risk,
        required_strength: AuthenticationStrength,
        challenge: impl Into<String>,
        created_at_unix_s: i64,
        expires_at_unix_s: i64,
        correlation: CorrelationId,
    ) -> Result<Self, StepUpError> {
        let challenge = challenge.into();
        if challenge.trim().is_empty() {
            return Err(StepUpError::Malformed("empty challenge".into()));
        }
        if (risk == Risk::R3 || risk == Risk::R4)
            && required_strength != AuthenticationStrength::StepUp
        {
            return Err(StepUpError::Malformed(
                "R3/R4 challenges require STEP_UP strength".into(),
            ));
        }
        if expires_at_unix_s <= created_at_unix_s {
            return Err(StepUpError::Malformed(
                "expiry must be after creation".into(),
            ));
        }
        Ok(Self {
            challenge_id,
            tenant_id,
            principal_id,
            risk,
            required_strength,
            challenge,
            created_at_unix_s,
            expires_at_unix_s,
            correlation,
            state: StepUpState::Pending,
        })
    }

    /// Whether the challenge is still pending and usable.
    pub fn is_valid_at(&self, now_unix_s: i64) -> bool {
        self.state == StepUpState::Pending
            && self.created_at_unix_s <= now_unix_s
            && now_unix_s < self.expires_at_unix_s
    }

    /// Satisfy the challenge with a verified step-up response.
    pub fn satisfy(
        &mut self,
        response: &StepUpResponse,
        now_unix_s: i64,
    ) -> Result<(), StepUpError> {
        if self.state != StepUpState::Pending {
            return Err(StepUpError::WrongState);
        }
        if now_unix_s >= self.expires_at_unix_s {
            return Err(StepUpError::ChallengeExpired);
        }
        if !response.verification_ok {
            return Err(StepUpError::VerificationFailed(
                response.failure_detail.clone().unwrap_or_default(),
            ));
        }
        if response.strength < self.required_strength {
            return Err(StepUpError::StrengthNotSatisfied);
        }
        self.state = StepUpState::Satisfied;
        Ok(())
    }

    /// Cancel the challenge (fail closed).
    pub fn cancel(&mut self) -> Result<(), StepUpError> {
        if self.state != StepUpState::Pending {
            return Err(StepUpError::WrongState);
        }
        self.state = StepUpState::Cancelled;
        Ok(())
    }

    /// Expire the challenge explicitly.
    pub fn expire(&mut self) -> Result<(), StepUpError> {
        if self.state != StepUpState::Pending {
            return Err(StepUpError::WrongState);
        }
        self.state = StepUpState::Expired;
        Ok(())
    }
}

/// A normalized step-up response from the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepUpResponse {
    /// Whether the provider verified the step-up cryptographically.
    pub verification_ok: bool,
    /// Failure detail when verification failed (redacted upstream).
    pub failure_detail: Option<String>,
    /// Strength the response satisfies.
    pub strength: AuthenticationStrength,
}

#[cfg(test)]
mod tests {
    use super::*;

    const CID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103";
    const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";

    fn r4_challenge(created: i64, expires: i64) -> StepUpChallenge {
        StepUpChallenge::new(
            NexusId::new(CID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            Risk::R4,
            AuthenticationStrength::StepUp,
            "challenge-payload",
            created,
            expires,
            CorrelationId::new(CORR).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn ep007_unit_step_up_challenge_constructs_for_r4() {
        let c = r4_challenge(1000, 2000);
        assert!(c.is_valid_at(1000));
        assert!(c.is_valid_at(1500));
        assert!(!c.is_valid_at(999)); // before creation
        assert_eq!(c.state, StepUpState::Pending);
    }

    #[test]
    fn ep007_unit_step_up_rejects_weak_strength_for_r3_r4() {
        let res = StepUpChallenge::new(
            NexusId::new(CID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            Risk::R4,
            AuthenticationStrength::MultiFactor,
            "challenge-payload",
            1000,
            2000,
            CorrelationId::new(CORR).unwrap(),
        );
        assert!(matches!(res, Err(StepUpError::Malformed(_))));
    }

    #[test]
    fn ep007_unit_step_up_allows_low_strength_for_r1() {
        let c = StepUpChallenge::new(
            NexusId::new(CID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            Risk::R1,
            AuthenticationStrength::SingleFactor,
            "challenge-payload",
            1000,
            2000,
            CorrelationId::new(CORR).unwrap(),
        )
        .unwrap();
        assert_eq!(c.required_strength, AuthenticationStrength::SingleFactor);
    }

    #[test]
    fn ep007_unit_step_up_rejects_inverted_window() {
        let res = StepUpChallenge::new(
            NexusId::new(CID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            Risk::R4,
            AuthenticationStrength::StepUp,
            "challenge-payload",
            2000,
            1000,
            CorrelationId::new(CORR).unwrap(),
        );
        assert_eq!(
            res,
            Err(StepUpError::Malformed(
                "expiry must be after creation".into()
            ))
        );
    }

    #[test]
    fn ep007_unit_step_up_satisfies_with_verified_response() {
        let mut c = r4_challenge(1000, 2000);
        c.satisfy(
            &StepUpResponse {
                verification_ok: true,
                failure_detail: None,
                strength: AuthenticationStrength::StepUp,
            },
            1500,
        )
        .unwrap();
        assert_eq!(c.state, StepUpState::Satisfied);
    }

    #[test]
    fn ep007_unit_step_up_rejects_insufficient_strength() {
        let mut c = r4_challenge(1000, 2000);
        let res = c.satisfy(
            &StepUpResponse {
                verification_ok: true,
                failure_detail: None,
                strength: AuthenticationStrength::MultiFactor,
            },
            1500,
        );
        assert_eq!(res, Err(StepUpError::StrengthNotSatisfied));
        assert_eq!(c.state, StepUpState::Pending);
    }

    #[test]
    fn ep007_unit_step_up_rejects_expired_challenge() {
        let mut c = r4_challenge(1000, 2000);
        let res = c.satisfy(
            &StepUpResponse {
                verification_ok: true,
                failure_detail: None,
                strength: AuthenticationStrength::StepUp,
            },
            2500,
        );
        assert_eq!(res, Err(StepUpError::ChallengeExpired));
    }

    #[test]
    fn ep007_unit_step_up_cancel_fails_closed() {
        let mut c = r4_challenge(1000, 2000);
        c.cancel().unwrap();
        assert_eq!(c.state, StepUpState::Cancelled);
        assert!(!c.is_valid_at(1500));
        // Cannot satisfy or re-cancel after cancellation.
        let res = c.satisfy(
            &StepUpResponse {
                verification_ok: true,
                failure_detail: None,
                strength: AuthenticationStrength::StepUp,
            },
            1500,
        );
        assert_eq!(res, Err(StepUpError::WrongState));
        assert_eq!(c.cancel(), Err(StepUpError::WrongState));
    }

    #[test]
    fn ep007_unit_step_up_serde_roundtrip() {
        let mut c = r4_challenge(1000, 2000);
        c.satisfy(
            &StepUpResponse {
                verification_ok: true,
                failure_detail: None,
                strength: AuthenticationStrength::StepUp,
            },
            1500,
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"SATISFIED\""));
        let back: StepUpChallenge = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }
}
