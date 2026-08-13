//! Device enrollment contract (SPEC-005; EP-007).
//!
//! Device enrollment binds a device identity (EP-003 `DeviceIdentity`)
//! to an owner principal and a trust posture. Enrollment starts in
//! `PENDING_VERIFICATION`; only after the verification evidence is
//! accepted does the device become `ENROLLED`. Trust is evidence, never
//! cryptographic authentication (INV-003).

use std::fmt;

use nexus_domain::{CorrelationId, DeviceId, NexusId, TenantId};
use serde::{Deserialize, Serialize};

use crate::vocabulary::DeviceEnrollmentState;

/// Error returned by device enrollment operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEnrollmentError {
    /// The enrollment is in the wrong state for the operation.
    WrongState,
    /// The verification evidence was rejected.
    VerificationRejected(String),
    /// A required field is absent or malformed.
    Malformed(String),
}

impl fmt::Display for DeviceEnrollmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongState => f.write_str("device enrollment in wrong state"),
            Self::VerificationRejected(detail) => {
                write!(f, "device verification rejected: {detail}")
            }
            Self::Malformed(detail) => write!(f, "malformed device enrollment: {detail}"),
        }
    }
}

impl std::error::Error for DeviceEnrollmentError {}

/// Device enrollment record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceEnrollment {
    /// Nexus-wide enrollment identifier.
    pub enrollment_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Device being enrolled (EP-003 device identity).
    pub device_id: DeviceId,
    /// Owning principal (person or service).
    pub owner_principal_id: NexusId,
    /// Current lifecycle state.
    pub state: DeviceEnrollmentState,
    /// Enrollment display name (for audit).
    pub display_name: String,
    /// Correlation of the enrollment event.
    pub correlation: CorrelationId,
}

impl DeviceEnrollment {
    /// Construct a pending device enrollment.
    pub fn new(
        enrollment_id: NexusId,
        tenant_id: TenantId,
        device_id: DeviceId,
        owner_principal_id: NexusId,
        display_name: impl Into<String>,
        correlation: CorrelationId,
    ) -> Result<Self, DeviceEnrollmentError> {
        let display_name = display_name.into();
        if display_name.trim().is_empty() {
            return Err(DeviceEnrollmentError::Malformed(
                "empty display name".into(),
            ));
        }
        Ok(Self {
            enrollment_id,
            tenant_id,
            device_id,
            owner_principal_id,
            state: DeviceEnrollmentState::PendingVerification,
            display_name,
            correlation,
        })
    }

    /// Accept the verification evidence and enroll the device.
    pub fn verify(&mut self, evidence: &VerificationEvidence) -> Result<(), DeviceEnrollmentError> {
        if self.state != DeviceEnrollmentState::PendingVerification {
            return Err(DeviceEnrollmentError::WrongState);
        }
        if !evidence.accepted {
            self.state = DeviceEnrollmentState::Rejected;
            return Err(DeviceEnrollmentError::VerificationRejected(
                evidence.failure_detail.clone().unwrap_or_default(),
            ));
        }
        self.state = DeviceEnrollmentState::Enrolled;
        Ok(())
    }

    /// Revoke the enrollment (terminal).
    pub fn revoke(&mut self) -> Result<(), DeviceEnrollmentError> {
        if self.state == DeviceEnrollmentState::Revoked {
            return Err(DeviceEnrollmentError::WrongState);
        }
        self.state = DeviceEnrollmentState::Revoked;
        Ok(())
    }

    /// Whether the device is enrolled.
    pub fn is_enrolled(&self) -> bool {
        self.state == DeviceEnrollmentState::Enrolled
    }
}

/// Normalized device verification evidence from the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationEvidence {
    /// Whether the provider accepted the device attestation.
    pub accepted: bool,
    /// Failure detail when rejected (redacted upstream).
    pub failure_detail: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const DID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105";
    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103";
    const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";

    fn enrollment() -> DeviceEnrollment {
        DeviceEnrollment::new(
            NexusId::new(EID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            DeviceId::new(DID).unwrap(),
            NexusId::new(PID).unwrap(),
            "Living Room Tablet",
            CorrelationId::new(CORR).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn ep007_unit_device_enrollment_starts_pending() {
        let e = enrollment();
        assert!(!e.is_enrolled());
        assert_eq!(e.state, DeviceEnrollmentState::PendingVerification);
    }

    #[test]
    fn ep007_unit_device_enrollment_accepts_verification() {
        let mut e = enrollment();
        e.verify(&VerificationEvidence {
            accepted: true,
            failure_detail: None,
        })
        .unwrap();
        assert!(e.is_enrolled());
        assert_eq!(e.state, DeviceEnrollmentState::Enrolled);
    }

    #[test]
    fn ep007_unit_device_enrollment_rejects_verification_evidence() {
        let mut e = enrollment();
        let res = e.verify(&VerificationEvidence {
            accepted: false,
            failure_detail: Some("attestation failed".into()),
        });
        assert_eq!(
            res,
            Err(DeviceEnrollmentError::VerificationRejected(
                "attestation failed".into()
            ))
        );
        assert_eq!(e.state, DeviceEnrollmentState::Rejected);
    }

    #[test]
    fn ep007_unit_device_enrollment_revokes_terminal() {
        let mut e = enrollment();
        e.verify(&VerificationEvidence {
            accepted: true,
            failure_detail: None,
        })
        .unwrap();
        e.revoke().unwrap();
        assert_eq!(e.state, DeviceEnrollmentState::Revoked);
        assert!(!e.is_enrolled());
        // Revoking twice fails; verifying after revocation fails.
        assert_eq!(e.revoke(), Err(DeviceEnrollmentError::WrongState));
        let res = e.verify(&VerificationEvidence {
            accepted: true,
            failure_detail: None,
        });
        assert_eq!(res, Err(DeviceEnrollmentError::WrongState));
    }

    #[test]
    fn ep007_unit_device_enrollment_rejects_empty_display_name() {
        let res = DeviceEnrollment::new(
            NexusId::new(EID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            DeviceId::new(DID).unwrap(),
            NexusId::new(PID).unwrap(),
            "",
            CorrelationId::new(CORR).unwrap(),
        );
        assert_eq!(
            res,
            Err(DeviceEnrollmentError::Malformed(
                "empty display name".into()
            ))
        );
    }

    #[test]
    fn ep007_unit_device_enrollment_serde_roundtrip() {
        let mut e = enrollment();
        e.verify(&VerificationEvidence {
            accepted: true,
            failure_detail: None,
        })
        .unwrap();
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"ENROLLED\""));
        let back: DeviceEnrollment = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }
}
