//! EP-025 exact-target call verification.
//!
//! A command on session A is verified ONLY by an observed state
//! transition on session A (exact target + expected state). An
//! unrelated change on session B never satisfies A's verification
//! (directive 22: UNKNOWN OUTCOME -> VERIFY FIRST -> NO BLIND RETRY;
//! directive J precedent preserved from EP-024).

use serde::{Deserialize, Serialize};

use crate::error::{CallError, CallErrorCode};
use crate::vocabulary::{CallSessionId, CallState};

/// Verification outcome for a requested state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallVerification {
    /// Exact target reached the expected state.
    Verified,
    /// Exact target observed but not in the expected state.
    Mismatch,
    /// Target state not observable (missing/unavailable).
    Unknown,
    /// An unrelated target changed; never satisfies verification.
    UnrelatedChange,
}

/// Exact-target call verifier (SPEC-006 verification).
#[derive(Debug, Clone, Default)]
pub struct CallVerifier;

impl CallVerifier {
    /// Evaluate whether the observed state of `target` verifies the
    /// requested transition for `expected` on the SAME session.
    pub fn verify(
        &self,
        expected_target: &CallSessionId,
        expected_state: CallState,
        observed_target: &CallSessionId,
        observed_state: CallState,
    ) -> CallVerification {
        if observed_target != expected_target {
            return CallVerification::UnrelatedChange;
        }
        if observed_state == expected_state {
            CallVerification::Verified
        } else if observed_state.is_terminal() || observed_state < expected_state {
            // A terminal state can never be the expected active state;
            // a state behind the requested rung is a mismatch.
            CallVerification::Mismatch
        } else {
            CallVerification::Mismatch
        }
    }

    /// Verify a single-leg readback for the exact session.
    pub fn verify_leg(
        &self,
        expected_target: &CallSessionId,
        expected_state: CallState,
        observed_target: &CallSessionId,
        observed_leg_states: &[CallState],
    ) -> Result<CallVerification, CallError> {
        if observed_target != expected_target {
            return Ok(CallVerification::UnrelatedChange);
        }
        if observed_leg_states.is_empty() {
            return Ok(CallVerification::Unknown);
        }
        if observed_leg_states.contains(&expected_state) {
            return Ok(CallVerification::Verified);
        }
        Ok(CallVerification::Mismatch)
    }

    /// Convert a failed verification into a typed error.
    pub fn error_for(
        &self,
        result: CallVerification,
        expected_target: &CallSessionId,
        expected_state: CallState,
        correlation: Option<String>,
    ) -> Result<(), CallError> {
        match result {
            CallVerification::Verified => Ok(()),
            CallVerification::UnrelatedChange => Err(CallError::new(
                CallErrorCode::Verification,
                "unrelated target changed; exact-target verification failed",
                correlation,
                Some(expected_target.to_string()),
            )),
            CallVerification::Mismatch => Err(CallError::new(
                CallErrorCode::Verification,
                format!(
                    "exact target did not reach expected state {}",
                    expected_state.as_str()
                ),
                correlation,
                Some(expected_target.to_string()),
            )),
            CallVerification::Unknown => Err(CallError::new(
                CallErrorCode::Verification,
                format!(
                    "target state not observable for expected {}",
                    expected_state.as_str()
                ),
                correlation,
                Some(expected_target.to_string()),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(name: &str) -> CallSessionId {
        CallSessionId::new(name).unwrap()
    }

    #[test]
    fn ep025_unit_verifier_exact_target() {
        let v = CallVerifier;
        let a = sid("session/a");
        let b = sid("session/b");
        // Exact target + expected state -> Verified.
        assert_eq!(
            v.verify(&a, CallState::Ringing, &a, CallState::Ringing),
            CallVerification::Verified
        );
        // Exact target, wrong state -> Mismatch.
        assert_eq!(
            v.verify(&a, CallState::Ringing, &a, CallState::Answered),
            CallVerification::Mismatch
        );
        // Unrelated target change NEVER verifies A's command.
        assert_eq!(
            v.verify(&a, CallState::Ringing, &b, CallState::Ringing),
            CallVerification::UnrelatedChange
        );
    }

    #[test]
    fn ep025_unit_verifier_unknown_when_unobservable() {
        let v = CallVerifier;
        let a = sid("session/a");
        assert_eq!(
            v.verify_leg(&a, CallState::Bridged, &a, &[]).unwrap(),
            CallVerification::Unknown
        );
    }

    #[test]
    fn ep025_unit_verifier_error_mapping() {
        let v = CallVerifier;
        let a = sid("session/a");
        let b = sid("session/b");
        assert!(v
            .error_for(
                v.verify(&a, CallState::Answered, &b, CallState::Answered),
                &a,
                CallState::Answered,
                Some("tel-1".into())
            )
            .is_err());
        let err = v
            .error_for(
                v.verify(&a, CallState::Answered, &b, CallState::Answered),
                &a,
                CallState::Answered,
                Some("tel-1".into()),
            )
            .unwrap_err();
        assert_eq!(err.code, CallErrorCode::Verification);
        assert_eq!(err.correlation.as_deref(), Some("tel-1"));
    }

    #[test]
    fn ep025_unit_verifier_terminal_never_verifies_active() {
        let v = CallVerifier;
        let a = sid("session/a");
        // A BUSY readback can never verify an ANSWERED expectation.
        assert_eq!(
            v.verify(&a, CallState::Answered, &a, CallState::Busy),
            CallVerification::Mismatch
        );
    }
}
