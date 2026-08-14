//! Scoped context capsule contract (SPEC-003 required behavior 5).
//!
//! Capsules contain only authorized, task-relevant, cited data and
//! expire after the task or declared retention. A capsule reference
//! travels on the wire; the contents stay in the capsule service.

use crate::error::FabricError;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Context capsule identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapsuleId(pub String);

/// Capsule lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapsuleState {
    Active,
    Expired,
    Revoked,
}

impl CapsuleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Expired => "EXPIRED",
            Self::Revoked => "REVOKED",
        }
    }
}

impl std::str::FromStr for CapsuleState {
    type Err = crate::vocabulary::FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ACTIVE" => Ok(Self::Active),
            "EXPIRED" => Ok(Self::Expired),
            "REVOKED" => Ok(Self::Revoked),
            other => Err(crate::vocabulary::FabricVocabularyError::unknown(
                "CapsuleState",
                other,
            )),
        }
    }
}

/// Wire-safe reference to a capsule (never the contents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapsuleReference {
    pub capsule_id: CapsuleId,
    pub task_id: String,
    pub expires_at_epoch_ms: u64,
}

/// A scoped context capsule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCapsule {
    pub capsule_id: CapsuleId,
    pub task_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    /// Citations for every included item (SPEC-003 required behavior 5).
    pub citations: Vec<String>,
    pub payload: serde_json::Value,
    pub created_at_epoch_ms: u64,
    pub expires_at_epoch_ms: u64,
    pub state: CapsuleState,
}

impl ContextCapsule {
    /// A capsule is expired when now passes its expiry bound.
    pub fn is_expired_at(&self, now_epoch_ms: u64) -> bool {
        now_epoch_ms >= self.expires_at_epoch_ms
    }

    /// Deterministic TTL helper for constructors.
    pub fn ttl_epoch_ms(ttl: Duration, now: SystemTime) -> u64 {
        let now_ms = now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        now_ms.saturating_add(ttl.as_millis() as u64)
    }
}

/// Provider-neutral context capsule service port.
pub trait ContextCapsuleService {
    /// Create a capsule for a task; payload must carry citations.
    fn create(&mut self, capsule: ContextCapsule) -> Result<CapsuleReference, FabricError>;
    /// Read a capsule by reference; expired/revoked capsules fail closed.
    fn read(&self, reference: &CapsuleReference) -> Result<ContextCapsule, FabricError>;
    /// Expire a capsule early.
    fn expire(&mut self, capsule_id: &CapsuleId) -> Result<(), FabricError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_capsule_state_round_trip() {
        for (wire, expected) in [
            ("ACTIVE", CapsuleState::Active),
            ("EXPIRED", CapsuleState::Expired),
            ("REVOKED", CapsuleState::Revoked),
        ] {
            assert_eq!(wire.parse::<CapsuleState>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("PENDING".parse::<CapsuleState>().is_err());
    }

    #[test]
    fn ep012_unit_capsule_expiry_is_deterministic() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let exp = ContextCapsule::ttl_epoch_ms(Duration::from_secs(60), now);
        assert_eq!(exp, 1_700_000_000_000 + 60_000);

        let capsule = ContextCapsule {
            capsule_id: CapsuleId("cap-1".into()),
            task_id: "task-1".into(),
            tenant_id: "tenant-1".into(),
            principal_id: "user:alice".into(),
            citations: vec!["doc-1".into()],
            payload: serde_json::json!({"summary": "..."}),
            created_at_epoch_ms: 1_700_000_000_000,
            expires_at_epoch_ms: exp,
            state: CapsuleState::Active,
        };
        assert!(!capsule.is_expired_at(exp - 1));
        assert!(capsule.is_expired_at(exp));
        assert!(capsule.is_expired_at(exp + 1));
    }
}
