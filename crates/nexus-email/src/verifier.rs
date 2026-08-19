//! EP-026 exact-target mail verification (SPEC-014 error states).
//!
//! Verification is exact-target: only the expected target reaching the
//! expected state satisfies a verification plan. An unrelated message
//! change NEVER verifies the target. Unobservable targets fail closed
//! as Unknown/Verification, never fabricated success.

use crate::error::{MailError, MailErrorCode};
use crate::vocabulary::{MailState, MessageId};

/// Result of an exact-target verification check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MailVerification {
    /// The exact target reached the expected state.
    Verified,
    /// The exact target exists but is in a different state.
    Mismatch,
    /// The exact target could not be observed (fail closed).
    Unknown,
    /// An unrelated target changed; it cannot satisfy this plan.
    UnrelatedChange,
}

impl MailVerification {
    pub const fn is_verified(self) -> bool {
        matches!(self, Self::Verified)
    }
}

/// Exact-target mail verifier: a readback satisfies a plan only when
/// the SAME message reaches the SAME expected state.
pub struct MailVerifier;

impl MailVerifier {
    /// Check whether `observed_message` matching `expected_state`
    /// verifies the plan for `target`.
    pub fn check(
        target: &MessageId,
        observed_message: Option<&MessageId>,
        observed_state: Option<MailState>,
        expected_state: MailState,
    ) -> MailVerification {
        match observed_message {
            Some(observed) if observed == target => match observed_state {
                Some(state) if state == expected_state => MailVerification::Verified,
                Some(_) => MailVerification::Mismatch,
                None => MailVerification::Unknown,
            },
            Some(_) => MailVerification::UnrelatedChange,
            None => MailVerification::Unknown,
        }
    }

    /// Convenience: return an error for a non-verified plan.
    pub fn require_verified(
        target: &MessageId,
        observed_message: Option<&MessageId>,
        observed_state: Option<MailState>,
        expected_state: MailState,
    ) -> Result<(), MailError> {
        match Self::check(target, observed_message, observed_state, expected_state) {
            MailVerification::Verified => Ok(()),
            MailVerification::Mismatch => Err(MailError::verification(format!(
                "message {target} did not reach expected state {:?}",
                expected_state.as_str()
            ))),
            MailVerification::Unknown => Err(MailError::new(
                MailErrorCode::Verification,
                format!("message {target} state is unobservable"),
                None,
                None,
            )),
            MailVerification::UnrelatedChange => Err(MailError::verification(format!(
                "unrelated message change cannot verify {target}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::MessageId;

    #[test]
    fn ep026_unit_verifier_exact_target_verified() {
        let id = MessageId::new("msg-1").expect("id");
        let other = MessageId::new("msg-2").expect("id");
        assert!(
            MailVerifier::check(&id, Some(&id), Some(MailState::Sent), MailState::Sent)
                .is_verified()
        );
        assert_eq!(
            MailVerifier::check(&id, Some(&other), Some(MailState::Sent), MailState::Sent),
            MailVerification::UnrelatedChange
        );
        assert_eq!(
            MailVerifier::check(&id, Some(&id), Some(MailState::Queued), MailState::Sent),
            MailVerification::Mismatch
        );
        assert_eq!(
            MailVerifier::check(&id, None, Some(MailState::Sent), MailState::Sent),
            MailVerification::Unknown
        );
        assert_eq!(
            MailVerifier::check(&id, Some(&id), None, MailState::Sent),
            MailVerification::Unknown
        );
    }

    #[test]
    fn ep026_unit_verifier_require_verified() {
        let id = MessageId::new("msg-3").expect("id");
        assert!(MailVerifier::require_verified(
            &id,
            Some(&id),
            Some(MailState::Delivered),
            MailState::Delivered
        )
        .is_ok());
        assert!(MailVerifier::require_verified(
            &id,
            Some(&id),
            Some(MailState::Sending),
            MailState::Delivered
        )
        .is_err());
        let other = MessageId::new("msg-4").expect("id");
        let err = MailVerifier::require_verified(
            &id,
            Some(&other),
            Some(MailState::Delivered),
            MailState::Delivered,
        )
        .expect_err("unrelated change must fail");
        assert_eq!(err.code, MailErrorCode::Verification);
    }
}
