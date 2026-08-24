//! Typed chaos failure classification (EP-040 M5 fence section H).
//! Every injected failure must produce a specific typed class; generic
//! failure is never sufficient for a resilience claim.

use serde::{Deserialize, Serialize};

/// Exact EP-040 chaos failure classes. A scenario outcome must classify
/// the observed failure into one of these; collapsing outcomes into a
/// generic success/failure is a vacuous-gate defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChaosFailureClass {
    /// The owning node's own code regressed (test or gate defect).
    OwnerCodeRegression,
    /// A shared fixture's state leaked into this run.
    FixtureStateLeak,
    /// The host ran out of a resource (disk, memory, inodes).
    ResourceExhaustion,
    /// A runtime ordering assumption failed (startup race etc.).
    RuntimeOrdering,
    /// A failure originated in another node's territory.
    ForeignNode,
    /// The global verify wrapper itself misbehaved.
    GlobalVerifyDefect,
    /// The environment (host, docker, network) caused the failure.
    Environment,
    /// Remote authentication is blocked (no credential / 401).
    AuthBlocked,
    /// A capability is absent; the component honestly reports it.
    CapabilityBlocked,
    /// A bounded operation exceeded its timeout budget.
    Timeout,
    /// A dependency became unavailable (connection refused, gone).
    Unavailable,
    /// A policy decision denied the operation.
    PolicyDenied,
    /// A security control rejected the operation.
    SecurityFailure,
    /// Hardware was not present/asserted for this proof.
    HardwareNotAsserted,
}

impl ChaosFailureClass {
    pub const VOCAB: &'static str = "EP-040 chaos failure class";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OwnerCodeRegression => "OWNER_CODE_REGRESSION",
            Self::FixtureStateLeak => "FIXTURE_STATE_LEAK",
            Self::ResourceExhaustion => "RESOURCE_EXHAUSTION",
            Self::RuntimeOrdering => "RUNTIME_ORDERING",
            Self::ForeignNode => "FOREIGN_NODE",
            Self::GlobalVerifyDefect => "GLOBAL_VERIFY_DEFECT",
            Self::Environment => "ENVIRONMENT",
            Self::AuthBlocked => "AUTH_BLOCKED",
            Self::CapabilityBlocked => "CAPABILITY_BLOCKED",
            Self::Timeout => "TIMEOUT",
            Self::Unavailable => "UNAVAILABLE",
            Self::PolicyDenied => "POLICY_DENIED",
            Self::SecurityFailure => "SECURITY_FAILURE",
            Self::HardwareNotAsserted => "HARDWARE_NOT_ASSERTED",
        }
    }
}

impl std::fmt::Display for ChaosFailureClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ChaosFailureClass {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OWNER_CODE_REGRESSION" => Ok(Self::OwnerCodeRegression),
            "FIXTURE_STATE_LEAK" => Ok(Self::FixtureStateLeak),
            "RESOURCE_EXHAUSTION" => Ok(Self::ResourceExhaustion),
            "RUNTIME_ORDERING" => Ok(Self::RuntimeOrdering),
            "FOREIGN_NODE" => Ok(Self::ForeignNode),
            "GLOBAL_VERIFY_DEFECT" => Ok(Self::GlobalVerifyDefect),
            "ENVIRONMENT" => Ok(Self::Environment),
            "AUTH_BLOCKED" => Ok(Self::AuthBlocked),
            "CAPABILITY_BLOCKED" => Ok(Self::CapabilityBlocked),
            "TIMEOUT" => Ok(Self::Timeout),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            "POLICY_DENIED" => Ok(Self::PolicyDenied),
            "SECURITY_FAILURE" => Ok(Self::SecurityFailure),
            "HARDWARE_NOT_ASSERTED" => Ok(Self::HardwareNotAsserted),
            _ => Err(format!("unknown {VOCAB}: {s}", VOCAB = Self::VOCAB)),
        }
    }
}
