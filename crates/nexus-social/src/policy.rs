//! EP-029 approval-class policy (SPEC-015 behavior 5 and 8).
//!
//! Publishing, replies, spend, and crisis statements use SEPARATE
//! approval classes; paid-ad budget changes and public crisis
//! responses require human approval. The policy gate runs BEFORE any
//! provider mutation; a denied action causes zero provider calls.

use nexus_domain::ApprovalClass;

use crate::error::{SocialError, SocialErrorCode};
use crate::vocabulary::SocialActionKind;

/// The approval class required for an action kind.
///
/// SPEC-015 behavior 5: publishing, replies, spend, and crisis
/// statements use SEPARATE approval classes. Behavior 8: paid-ad
/// budget changes and public crisis responses require human approval.
pub fn required_approval_class(kind: SocialActionKind) -> ApprovalClass {
    match kind {
        // Publishing an approved message to a certified account.
        SocialActionKind::Publish => ApprovalClass::Human,
        // Replies are governed by policy (never blind auto-replies).
        SocialActionKind::Reply => ApprovalClass::Policy,
        // Paid-ad budget changes require human approval (behavior 8).
        SocialActionKind::SpendChange => ApprovalClass::StrongHuman,
        // Public crisis responses require human approval (behavior 8),
        // strongest class.
        SocialActionKind::CrisisStatement => ApprovalClass::FourEyes,
    }
}

/// Enforce the approval gate. Returns Ok when the granted class is at
/// least the required class for the action kind; otherwise a Policy
/// error. The caller must invoke this BEFORE any provider mutation.
pub fn enforce_social_action_policy(
    kind: SocialActionKind,
    granted: ApprovalClass,
) -> Result<(), SocialError> {
    let required = required_approval_class(kind);
    if class_rank(granted) >= class_rank(required) {
        Ok(())
    } else {
        Err(SocialError::new(
            SocialErrorCode::Policy,
            format!("social action {kind} requires approval class {required}, got {granted}"),
            None,
            None,
            None,
            None,
        ))
    }
}

/// Strict ordering over approval classes (SPEC-006): NONE < POLICY <
/// HUMAN < STRONG_HUMAN < FOUR_EYES.
pub fn class_rank(class: ApprovalClass) -> u8 {
    match class {
        ApprovalClass::None => 0,
        ApprovalClass::Policy => 1,
        ApprovalClass::Human => 2,
        ApprovalClass::StrongHuman => 3,
        ApprovalClass::FourEyes => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep029_unit_action_kinds_have_separate_approval_classes() {
        // SPEC-015 behavior 5: publishing, replies, spend, and crisis
        // statements use SEPARATE approval classes.
        let classes = [
            required_approval_class(SocialActionKind::Publish),
            required_approval_class(SocialActionKind::Reply),
            required_approval_class(SocialActionKind::SpendChange),
            required_approval_class(SocialActionKind::CrisisStatement),
        ];
        let mut spelled: Vec<&str> = classes.iter().map(|c| c.as_str()).collect();
        spelled.sort_unstable();
        spelled.dedup();
        assert_eq!(spelled.len(), 4, "four separate approval classes");
    }

    #[test]
    fn ep029_unit_spend_and_crisis_require_human_approval() {
        // SPEC-015 behavior 8: paid-ad budget changes and public
        // crisis responses require human approval.
        assert!(
            class_rank(required_approval_class(SocialActionKind::SpendChange))
                >= class_rank(ApprovalClass::Human)
        );
        assert!(
            class_rank(required_approval_class(SocialActionKind::CrisisStatement))
                >= class_rank(ApprovalClass::Human)
        );
    }

    #[test]
    fn ep029_unit_policy_enforcement_denies_insufficient_class() {
        let err =
            enforce_social_action_policy(SocialActionKind::CrisisStatement, ApprovalClass::Policy)
                .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Policy);
        let err = enforce_social_action_policy(SocialActionKind::Publish, ApprovalClass::None)
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Policy);
    }

    #[test]
    fn ep029_unit_policy_enforcement_grants_sufficient_class() {
        assert!(
            enforce_social_action_policy(SocialActionKind::Publish, ApprovalClass::Human).is_ok()
        );
        assert!(
            enforce_social_action_policy(SocialActionKind::Reply, ApprovalClass::Policy).is_ok()
        );
        assert!(enforce_social_action_policy(
            SocialActionKind::SpendChange,
            ApprovalClass::StrongHuman
        )
        .is_ok());
        assert!(enforce_social_action_policy(
            SocialActionKind::CrisisStatement,
            ApprovalClass::FourEyes
        )
        .is_ok());
    }

    #[test]
    fn ep029_unit_policy_rank_ordering() {
        assert!(class_rank(ApprovalClass::None) < class_rank(ApprovalClass::Policy));
        assert!(class_rank(ApprovalClass::Policy) < class_rank(ApprovalClass::Human));
        assert!(class_rank(ApprovalClass::Human) < class_rank(ApprovalClass::StrongHuman));
        assert!(class_rank(ApprovalClass::StrongHuman) < class_rank(ApprovalClass::FourEyes));
    }
}
