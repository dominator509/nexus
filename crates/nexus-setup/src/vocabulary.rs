//! EP-035 canonical setup vocabulary (SPEC-004 / SPEC-016).
//!
//! SPEC-016 canonical terms (DeploymentProfile, NodeManifest,
//! WorkloadManifest, PlacementPlan, Provisioner, BootstrapToken,
//! ReleaseManifest, OfflineBundle, UpdateTransaction) are vocabulary
//! locked; this crate uses them without redefining them. SPEC-004
//! locked terms (Nexus Setup, Deployment Plan, Integration Card,
//! Recovery Kit, Release Channel) are the vocabulary anchors.
//!
//! EP-035 owns the setup-wizard vocabulary: wizard states and step
//! statuses, deployment modes and release channels, hardware
//! provenance, capability certification, owner bootstrap states,
//! enrollment trust states, credential states, discovery kinds,
//! integration statuses, recovery material kinds, failure classes,
//! mutation states, and recovery outcomes. State truthfulness is
//! structural: SELECTED != PROVISIONED, CONFIGURED != HEALTHY,
//! DISCOVERED != TRUSTED, COMPLETE_LOCAL != VERIFIED.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::SetupError;

macro_rules! enum_vocab {
    ($(#[$doc:meta])* $name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $wire),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = SetupError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($wire => Ok($name::$variant),)+
                    other => Err(SetupError::vocabulary(format!(
                        concat!(stringify!($name), " has unsupported value '{}'"),
                        other
                    ))),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = SetupError;
            fn try_from(s: &str) -> Result<Self, SetupError> {
                s.parse()
            }
        }
    };
}

enum_vocab! {
    /// Whole-wizard state. NOT_STARTED -> IN_PROGRESS is the only start;
    /// COMPLETED requires every step VERIFIED.
    WizardState {
        NotStarted => "NOT_STARTED",
        InProgress => "IN_PROGRESS",
        Blocked => "BLOCKED",
        Failed => "FAILED",
        RecoveryRequired => "RECOVERY_REQUIRED",
        Completed => "COMPLETED",
    }
}

enum_vocab! {
    /// Wizard steps owned by EP-035.
    WizardStep {
        DeploymentChoice => "DEPLOYMENT_CHOICE",
        HardwareProfile => "HARDWARE_PROFILE",
        OwnerBootstrap => "OWNER_BOOTSTRAP",
        RecoveryMaterial => "RECOVERY_MATERIAL",
        EdgeEnrollment => "EDGE_ENROLLMENT",
        Discovery => "DISCOVERY",
        IntegrationReview => "INTEGRATION_REVIEW",
        PlanReview => "PLAN_REVIEW",
    }
}

enum_vocab! {
    /// Per-step status. COMPLETE_LOCAL is a local checkpoint and is
    /// NEVER equal to VERIFIED (LOCAL_PROGRESS_SAVED !=
    /// REMOTE_EFFECT_VERIFIED).
    WizardStepStatus {
        Pending => "PENDING",
        InProgress => "IN_PROGRESS",
        Blocked => "BLOCKED",
        Failed => "FAILED",
        CompleteLocal => "COMPLETE_LOCAL",
        Verified => "VERIFIED",
    }
}

enum_vocab! {
    /// Canonical deployment modes (schemas/deployment-profile.schema.json).
    DeploymentMode {
        Managed => "MANAGED",
        Byoc => "BYOC",
        ExistingSsh => "EXISTING_SSH",
        Hybrid => "HYBRID",
        FullyLocal => "FULLY_LOCAL",
    }
}

enum_vocab! {
    /// Canonical release channels (schemas/deployment-profile.schema.json).
    ReleaseChannel {
        Stable => "STABLE",
        Beta => "BETA",
        Developer => "DEVELOPER",
        Pinned => "PINNED",
    }
}

enum_vocab! {
    /// Deployment verification state. Intent (SELECTED) is never
    /// VERIFIED; VERIFIED requires an evidence record.
    DeploymentVerificationState {
        Unverified => "UNVERIFIED",
        Verifying => "VERIFYING",
        Verified => "VERIFIED",
        Failed => "FAILED",
    }
}

enum_vocab! {
    /// Hardware fact provenance. "user says RTX GPU" is USER_DECLARED,
    /// never HOST_OBSERVED.
    HardwareProvenance {
        UserDeclared => "USER_DECLARED",
        HostObserved => "HOST_OBSERVED",
        PlatformReported => "PLATFORM_REPORTED",
        Benchmarked => "BENCHMARKED",
        HardwareCertified => "HARDWARE_CERTIFIED",
    }
}

enum_vocab! {
    /// Capability certification. NOT_CERTIFIED is the fail-closed
    /// default; CERTIFIED requires measured evidence and a measured
    /// provenance.
    CapabilityCertificationState {
        NotCertified => "NOT_CERTIFIED",
        Certified => "CERTIFIED",
    }
}

enum_vocab! {
    /// Owner bootstrap ladder. OWNER_DETAILS_PROVIDED !=
    /// OWNER_IDENTITY_VERIFIED != OWNER_PRINCIPAL_CREATED !=
    /// OWNER_AUTHORIZED.
    OwnerBootstrapState {
        DetailsProvided => "OWNER_DETAILS_PROVIDED",
        IdentityVerified => "OWNER_IDENTITY_VERIFIED",
        PrincipalCreated => "OWNER_PRINCIPAL_CREATED",
        OwnerAuthorized => "OWNER_AUTHORIZED",
    }
}

enum_vocab! {
    /// Edge enrollment trust ladder. DISCOVERED != ENROLLMENT_REQUESTED
    /// != IDENTITY_VERIFIED != ENROLLED != TRUSTED != AUTHORIZED.
    EnrollmentTrustState {
        Discovered => "DISCOVERED",
        EnrollmentRequested => "ENROLLMENT_REQUESTED",
        IdentityVerified => "IDENTITY_VERIFIED",
        Enrolled => "ENROLLED",
        Trusted => "TRUSTED",
        Authorized => "AUTHORIZED",
    }
}

enum_vocab! {
    /// Enrollment credential state. Only ISSUED credentials within
    /// their validity window are usable; USED/REVOKED/EXPIRED are never
    /// valid again, even if the UI cached them.
    EnrollmentCredentialState {
        Issued => "ISSUED",
        Used => "USED",
        Revoked => "REVOKED",
        Expired => "EXPIRED",
    }
}

enum_vocab! {
    /// Discovery observation kind.
    DiscoveryKind {
        Service => "SERVICE",
        Device => "DEVICE",
        Edge => "EDGE",
    }
}

enum_vocab! {
    /// Integration status. UNCONFIGURED != CONFIGURED != AUTHENTICATED
    /// != REACHABLE != HEALTHY, with DEGRADED and ERROR distinct.
    /// Credential-exists never becomes HEALTHY.
    IntegrationStatus {
        Unconfigured => "UNCONFIGURED",
        Configured => "CONFIGURED",
        Authenticated => "AUTHENTICATED",
        Reachable => "REACHABLE",
        Degraded => "DEGRADED",
        Healthy => "HEALTHY",
        Error => "ERROR",
    }
}

enum_vocab! {
    /// Canonical recovery material kinds
    /// (schemas/auth/recovery-kit.schema.json).
    RecoveryMaterialKind {
        RecoveryCodes => "RECOVERY_CODES",
        OfflinePassphrase => "OFFLINE_PASSPHRASE",
        DeviceBackup => "DEVICE_BACKUP",
    }
}

enum_vocab! {
    /// Recovery failure classes.
    RecoveryFailureClass {
        Ambiguous => "AMBIGUOUS",
        Validation => "VALIDATION",
        Authorization => "AUTHORIZATION",
        Unavailable => "UNAVAILABLE",
        Timeout => "TIMEOUT",
        Conflict => "CONFLICT",
        Internal => "INTERNAL",
    }
}

enum_vocab! {
    /// Recovery mutation state. UNKNOWN means reconciliation is
    /// required before retry.
    RecoveryMutationState {
        Unknown => "UNKNOWN",
        Reconciled => "RECONCILED",
    }
}

enum_vocab! {
    /// Recovery outcomes. Retry is safe ONLY for RETRYABLE after the
    /// mutation state is reconciled or known-no-mutation.
    RecoveryOutcome {
        Retryable => "RETRYABLE",
        NonRetryable => "NON_RETRYABLE",
        ResumeCheckpoint => "RESUME_CHECKPOINT",
        Reconcile => "RECONCILE",
        Rollback => "ROLLBACK",
        Reauthenticate => "REAUTHENTICATE",
        Reset => "RESET",
        ManualIntervention => "MANUAL_INTERVENTION",
    }
}

/// Hostile discovery authority tokens: data when observed, never
/// authority when claimed.
pub const HOSTILE_AUTHORITY_TOKENS: &[&str] = &[
    "ADMIN",
    "TRUSTED",
    "AUTO-APPROVE",
    "AUTO_APPROVE",
    "OWNER DEVICE",
    "OWNER_DEVICE",
    "ROOT",
    "SUPERUSER",
];

/// A hostile authority token claims authority from discovery data.
pub fn contains_hostile_authority_token(haystack: &str) -> bool {
    let upper = haystack.to_ascii_uppercase();
    HOSTILE_AUTHORITY_TOKENS
        .iter()
        .any(|token| upper.contains(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep035_unit_vocabulary_round_trip() {
        assert_eq!(DeploymentMode::FullyLocal.as_str(), "FULLY_LOCAL");
        assert_eq!(
            "FULLY_LOCAL".parse::<DeploymentMode>().unwrap(),
            DeploymentMode::FullyLocal
        );
        assert_eq!(IntegrationStatus::Healthy.as_str(), "HEALTHY");
        assert_eq!(
            "HEALTHY".parse::<IntegrationStatus>().unwrap(),
            IntegrationStatus::Healthy
        );
    }

    #[test]
    fn ep035_unit_vocabulary_rejects_unknown() {
        assert!("MADE_UP".parse::<WizardState>().is_err());
        assert!("GOD_MODE".parse::<IntegrationStatus>().is_err());
    }

    #[test]
    fn ep035_unit_hostile_token_detection() {
        assert!(contains_hostile_authority_token("mdns://ADMIN.local"));
        assert!(contains_hostile_authority_token("OWNER DEVICE"));
        assert!(!contains_hostile_authority_token("kitchen-speaker"));
    }
}
