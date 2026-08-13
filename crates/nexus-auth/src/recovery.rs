//! Recovery kit contract (SPEC-005 behavior 6; EP-007).
//!
//! Owner onboarding creates passkey and offline recovery material. The
//! recovery kit holds sealed material (envelope, split shares, or
//! recovery code) that can restore access after device loss. Material is
//! sealed at the boundary; this contract manages the lifecycle and the
//! audit trail, never the plaintext secrets themselves.

use std::fmt;

use nexus_domain::{CorrelationId, NexusId, TenantId};
use serde::{Deserialize, Serialize};

use crate::vocabulary::RecoveryMaterialKind;

/// Error returned by recovery kit operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    /// The kit is sealed and cannot be modified.
    Sealed,
    /// The kit is in the wrong state for the operation.
    WrongState,
    /// A required field is absent or malformed.
    Malformed(String),
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sealed => f.write_str("recovery kit is sealed"),
            Self::WrongState => f.write_str("recovery kit in wrong state"),
            Self::Malformed(detail) => write!(f, "malformed recovery kit: {detail}"),
        }
    }
}

impl std::error::Error for RecoveryError {}

/// Recovery kit lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecoveryKitState {
    /// Material created but not yet sealed/verified.
    Provisioned,
    /// Material sealed and usable for recovery.
    Sealed,
    /// Material verified during a recovery exercise.
    Verified,
    /// Material revoked (terminal).
    Revoked,
}

impl RecoveryKitState {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Provisioned => "PROVISIONED",
            Self::Sealed => "SEALED",
            Self::Verified => "VERIFIED",
            Self::Revoked => "REVOKED",
        }
    }
}

impl fmt::Display for RecoveryKitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An offline recovery kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryKit {
    /// Nexus-wide kit identifier.
    pub kit_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Owning principal.
    pub principal_id: NexusId,
    /// Kind of sealed material.
    pub material_kind: RecoveryMaterialKind,
    /// Secret reference to the sealed material (SPEC-005 behavior 6);
    /// never the material itself.
    pub material_ref: String,
    /// State of the kit.
    pub state: RecoveryKitState,
    /// Unix seconds when the kit was created.
    pub created_at_unix_s: i64,
    /// Unix seconds of the last verification, when verified.
    pub last_verified_at_unix_s: Option<i64>,
    /// Correlation of the creation event.
    pub correlation: CorrelationId,
}

impl RecoveryKit {
    /// Construct a provisioned recovery kit.
    pub fn new(
        kit_id: NexusId,
        tenant_id: TenantId,
        principal_id: NexusId,
        material_kind: RecoveryMaterialKind,
        material_ref: impl Into<String>,
        created_at_unix_s: i64,
        correlation: CorrelationId,
    ) -> Result<Self, RecoveryError> {
        let material_ref = material_ref.into();
        if material_ref.trim().is_empty() {
            return Err(RecoveryError::Malformed("empty material reference".into()));
        }
        Ok(Self {
            kit_id,
            tenant_id,
            principal_id,
            material_kind,
            material_ref,
            state: RecoveryKitState::Provisioned,
            created_at_unix_s,
            last_verified_at_unix_s: None,
            correlation,
        })
    }

    /// Seal the kit (mark material verified as correctly stored).
    pub fn seal(&mut self) -> Result<(), RecoveryError> {
        if self.state != RecoveryKitState::Provisioned {
            return Err(RecoveryError::WrongState);
        }
        self.state = RecoveryKitState::Sealed;
        Ok(())
    }

    /// Record a successful recovery exercise.
    pub fn verify(&mut self, at_unix_s: i64) -> Result<(), RecoveryError> {
        if self.state == RecoveryKitState::Revoked {
            return Err(RecoveryError::WrongState);
        }
        if self.state == RecoveryKitState::Provisioned {
            return Err(RecoveryError::Sealed);
        }
        self.state = RecoveryKitState::Verified;
        self.last_verified_at_unix_s = Some(at_unix_s);
        Ok(())
    }

    /// Revoke the kit (terminal).
    pub fn revoke(&mut self) -> Result<(), RecoveryError> {
        if self.state == RecoveryKitState::Revoked {
            return Err(RecoveryError::WrongState);
        }
        self.state = RecoveryKitState::Revoked;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103";
    const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";

    fn kit() -> RecoveryKit {
        RecoveryKit::new(
            NexusId::new(KID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            RecoveryMaterialKind::SealedEnvelope,
            "vault://secret/recovery/owner-1",
            1000,
            CorrelationId::new(CORR).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn ep007_unit_recovery_kit_provisions_then_seals() {
        let mut k = kit();
        assert_eq!(k.state, RecoveryKitState::Provisioned);
        k.seal().unwrap();
        assert_eq!(k.state, RecoveryKitState::Sealed);
    }

    #[test]
    fn ep007_unit_recovery_kit_verify_requires_sealed() {
        let mut k = kit();
        let res = k.verify(1500);
        assert_eq!(res, Err(RecoveryError::Sealed));
        k.seal().unwrap();
        k.verify(1500).unwrap();
        assert_eq!(k.state, RecoveryKitState::Verified);
        assert_eq!(k.last_verified_at_unix_s, Some(1500));
    }

    #[test]
    fn ep007_unit_recovery_kit_revokes_terminal() {
        let mut k = kit();
        k.seal().unwrap();
        k.revoke().unwrap();
        assert_eq!(k.state, RecoveryKitState::Revoked);
        assert_eq!(k.revoke(), Err(RecoveryError::WrongState));
        assert_eq!(k.verify(1600), Err(RecoveryError::WrongState));
    }

    #[test]
    fn ep007_unit_recovery_kit_rejects_empty_material_ref() {
        let res = RecoveryKit::new(
            NexusId::new(KID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            RecoveryMaterialKind::RecoveryCode,
            "",
            1000,
            CorrelationId::new(CORR).unwrap(),
        );
        assert_eq!(
            res,
            Err(RecoveryError::Malformed("empty material reference".into()))
        );
    }

    #[test]
    fn ep007_unit_recovery_kit_serde_roundtrip() {
        let mut k = kit();
        k.seal().unwrap();
        k.verify(1500).unwrap();
        let json = serde_json::to_string(&k).unwrap();
        assert!(json.contains("\"VERIFIED\""));
        let back: RecoveryKit = serde_json::from_str(&json).unwrap();
        assert_eq!(back, k);
    }
}
