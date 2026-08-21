//! EP-036 canonical compute fabric vocabulary (SPEC-016).
//!
//! SPEC-016 canonical terms (DeploymentProfile, NodeManifest,
//! WorkloadManifest, PlacementPlan, Provisioner, BootstrapToken,
//! ReleaseManifest, OfflineBundle, UpdateTransaction) are vocabulary
//! locked; this crate uses them without redefining them.
//!
//! EP-036 owns the compute-fabric vocabulary: compute classes, provider
//! kinds, resource-state ladder, capacity provenance, provider/resource/
//! workload health separation, provisioning outcomes (including the
//! ambiguous UNKNOWN outcome), verification states, delete states,
//! billing states, quota states, and fleet enrollment states. State
//! truthfulness is structural: REQUESTED != PLANNED != SUBMITTED !=
//! PROVISIONING != CREATED != REACHABLE != READY != VERIFIED !=
//! CERTIFIED, PROVIDER API HEALTH != RESOURCE HEALTH != WORKLOAD HEALTH,
//! DECLARED CAPACITY != OBSERVED CAPACITY != CERTIFIED CAPACITY.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ComputeError;

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
            type Err = ComputeError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($wire => Ok($name::$variant),)+
                    other => Err(ComputeError::vocabulary(format!(
                        concat!(stringify!($name), " has unsupported value '{}'"),
                        other
                    ))),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ComputeError;
            fn try_from(s: &str) -> Result<Self, ComputeError> {
                s.parse()
            }
        }
    };
}

enum_vocab! {
    /// Compute target class. Distinct classes are never interchangeable
    /// just because they all run workloads; placement policy selects among
    /// eligible targets (SPEC-016 Compute Fabric).
    ComputeClass {
        Local => "LOCAL",
        Edge => "EDGE",
        Vps => "VPS",
        Cloud => "CLOUD",
        GpuHost => "GPU_HOST",
        RemoteWorker => "REMOTE_WORKER",
    }
}

enum_vocab! {
    /// Canonical provisioning providers (SPEC-016: Contabo, Hetzner,
    /// DigitalOcean, AWS, and generic SSH; fully local remains a first
    /// class path). Unknown providers are rejected, never dynamically
    /// invented from arbitrary strings.
    ProviderKind {
        Contabo => "CONTABO",
        Hetzner => "HETZNER",
        DigitalOcean => "DIGITAL_OCEAN",
        Aws => "AWS",
        GenericSsh => "GENERIC_SSH",
        Local => "LOCAL",
    }
}

enum_vocab! {
    /// Resource-state ladder. REQUESTED != PLANNED != SUBMITTED !=
    /// PROVISIONING != CREATED != REACHABLE != READY != VERIFIED !=
    /// CERTIFIED. Provider acceptance of a request establishes only
    /// SUBMITTED, never READY.
    ResourceState {
        Requested => "REQUESTED",
        Planned => "PLANNED",
        Submitted => "SUBMITTED",
        Provisioning => "PROVISIONING",
        Created => "CREATED",
        Reachable => "REACHABLE",
        Ready => "READY",
        Verified => "VERIFIED",
        Certified => "CERTIFIED",
    }
}

enum_vocab! {
    /// Capacity provenance. A user/config declaration ("16 GB VRAM") is
    /// DECLARED, never OBSERVED; observed values are never CERTIFIED
    /// without workload-level proof.
    CapacityProvenance {
        Declared => "DECLARED",
        Observed => "OBSERVED",
        Certified => "CERTIFIED",
    }
}

enum_vocab! {
    /// Provider API health: the cloud API being reachable does not prove
    /// any created resource is healthy.
    ProviderApiHealth {
        Unknown => "UNKNOWN",
        Reachable => "REACHABLE",
        Degraded => "DEGRADED",
        Unavailable => "UNAVAILABLE",
    }
}

enum_vocab! {
    /// Resource-specific health. Distinct from provider API health and
    /// workload health.
    ResourceHealth {
        Unknown => "UNKNOWN",
        Created => "CREATED",
        Reachable => "REACHABLE",
        Ready => "READY",
        Degraded => "DEGRADED",
        Failed => "FAILED",
    }
}

enum_vocab! {
    /// Workload lifecycle. WORKLOAD ASSIGNED != STARTED != HEALTHY !=
    /// VERIFIED. Scheduler intent never becomes runtime truth.
    WorkloadState {
        Unassigned => "UNASSIGNED",
        Assigned => "ASSIGNED",
        Started => "STARTED",
        Healthy => "HEALTHY",
        Verified => "VERIFIED",
    }
}

enum_vocab! {
    /// Provisioning outcome. A provider request may have succeeded while
    /// the client lost confirmation: that is AMBIGUOUS (UNKNOWN /
    /// VERIFICATION_REQUIRED), never FAILED, and never safe for blind
    /// automatic retry.
    ProvisioningOutcome {
        Pending => "PENDING",
        Succeeded => "SUCCEEDED",
        Failed => "FAILED",
        Ambiguous => "AMBIGUOUS",
    }
}

enum_vocab! {
    /// Verification state for a claimed side effect.
    VerificationState {
        Pending => "PENDING",
        Verified => "VERIFIED",
        Mismatch => "MISMATCH",
    }
}

enum_vocab! {
    /// Deprovision lifecycle. DELETE REQUESTED != DELETE ACCEPTED !=
    /// RESOURCE ABSENT VERIFIED; absence must be independently read back.
    DeleteState {
        NotRequested => "NOT_REQUESTED",
        DeleteRequested => "DELETE_REQUESTED",
        DeleteAccepted => "DELETE_ACCEPTED",
        ResourceAbsentVerified => "RESOURCE_ABSENT_VERIFIED",
    }
}

enum_vocab! {
    /// Cost/budget states. An estimate is never actual billing.
    BillingState {
        NoCost => "NO_COST",
        Estimated => "ESTIMATED",
        Incurred => "INCURRED",
        Settled => "SETTLED",
    }
}

enum_vocab! {
    /// Quota semantics. Provider quota values come from provider readback
    /// only; M1 models semantics, never fabricated defaults.
    QuotaState {
        Unobserved => "UNOBSERVED",
        Observed => "OBSERVED",
        Exceeded => "EXCEEDED",
    }
}

enum_vocab! {
    /// Fleet enrollment ladder. DISCOVERED != ENROLLMENT_REQUESTED !=
    /// IDENTITY_VERIFIED != ENROLLED != TRUSTED (SPEC-016 private mesh).
    FleetEnrollmentState {
        Discovered => "DISCOVERED",
        EnrollmentRequested => "ENROLLMENT_REQUESTED",
        IdentityVerified => "IDENTITY_VERIFIED",
        Enrolled => "ENROLLED",
        Trusted => "TRUSTED",
    }
}

enum_vocab! {
    /// Placement failure class (fail-closed reasons).
    PlacementFailureClass {
        NoEligibleTarget => "NO_ELIGIBLE_TARGET",
        ConstraintUnsatisfiable => "CONSTRAINT_UNSATISFIABLE",
        PrivacyBoundaryViolation => "PRIVACY_BOUNDARY_VIOLATION",
        TenantBoundaryViolation => "TENANT_BOUNDARY_VIOLATION",
        BudgetExceeded => "BUDGET_EXCEEDED",
        QuotaExceeded => "QUOTA_EXCEEDED",
        UnknownCapability => "UNKNOWN_CAPABILITY",
    }
}
