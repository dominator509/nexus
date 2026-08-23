//! Dependency waiver validation (SPEC-019 behavior 8; SPEC-019 required
//! tests: advisory workflow, waiver bounds).
//!
//! Deterministic invariants:
//! - WAIVER PRESENT != WAIVER ACTIVE
//! - waiver absent -> denied where required
//! - expired waiver -> denied
//! - revoked waiver -> denied
//! - wrong package waiver -> denied
//! - wrong version waiver -> denied
//! - wrong scope waiver -> denied
//! - broad wildcard waiver -> denied unless policy explicitly permits it
//! - valid waiver -> permits only the exact bounded decision
//!
//! Waiver existence is never global approval.

use nexus_supply_chain::model::DependencyWaiver;
use nexus_supply_chain::vocabulary::WaiverState;

/// Scope of a waiver: the exact bounded decision it may permit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WaiverScope {
    /// Build-time only dependency.
    BuildTime,
    /// Runtime linked/embedded dependency.
    Runtime,
    /// Test fixture dependency.
    TestFixture,
    /// External service/provider dependency.
    ExternalService,
}

/// Deterministic waiver policy configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaiverPolicyConfig {
    /// Permitted scopes for waivers (exact bounded decision).
    pub permitted_scopes: Vec<WaiverScope>,
    /// Whether a wildcard package/version is permitted at all. Default
    /// denies wildcards unless the policy explicitly permits them.
    pub allow_wildcard: bool,
}

impl Default for WaiverPolicyConfig {
    fn default() -> Self {
        Self {
            permitted_scopes: vec![WaiverScope::Runtime],
            allow_wildcard: false,
        }
    }
}

/// Outcome of a waiver validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaiverEvaluation {
    /// True only when the waiver is active, exact, and in scope.
    pub valid: bool,
    /// Deterministic human-safe reason.
    pub reason: String,
    /// The waiver state when a waiver was supplied.
    pub state: Option<WaiverState>,
}

/// Deterministic waiver policy engine.
#[derive(Debug, Clone)]
pub struct WaiverPolicy {
    pub config: WaiverPolicyConfig,
}

impl Default for WaiverPolicy {
    fn default() -> Self {
        Self::new(WaiverPolicyConfig::default())
    }
}

impl WaiverPolicy {
    pub fn new(config: WaiverPolicyConfig) -> Self {
        Self { config }
    }

    /// Validate a waiver for an exact package+version+scope decision.
    /// `waiver` is None when no waiver exists for the package.
    pub fn validate(
        &self,
        waiver: Option<&DependencyWaiver>,
        package: &str,
        version: &str,
        scope: &WaiverScope,
        now_ts: u64,
    ) -> WaiverEvaluation {
        let waiver = match waiver {
            Some(w) => w,
            None => {
                return WaiverEvaluation {
                    valid: false,
                    reason: "waiver absent: denied where required".to_string(),
                    state: None,
                }
            }
        };

        // WAIVER PRESENT != WAIVER ACTIVE.
        if waiver.state != WaiverState::Active {
            return WaiverEvaluation {
                valid: false,
                reason: format!("waiver state {}: not active", waiver.state),
                state: Some(waiver.state),
            };
        }

        // Expired waiver -> denied.
        if now_ts > waiver.expires_at_ts {
            return WaiverEvaluation {
                valid: false,
                reason: "waiver expired".to_string(),
                state: Some(waiver.state),
            };
        }

        // Broad wildcard -> denied unless the policy explicitly permits it.
        let package_wildcard = waiver.package == "*";
        let version_wildcard = waiver.version == "*";
        if !self.config.allow_wildcard && (package_wildcard || version_wildcard) {
            return WaiverEvaluation {
                valid: false,
                reason: "wildcard waiver denied unless policy explicitly permits it".to_string(),
                state: Some(waiver.state),
            };
        }

        // Wrong package -> denied (a wildcard package matches any when the
        // policy explicitly permits wildcards).
        if !(package_wildcard && self.config.allow_wildcard) && waiver.package != package {
            return WaiverEvaluation {
                valid: false,
                reason: "waiver is for a different package".to_string(),
                state: Some(waiver.state),
            };
        }

        // Wrong version -> denied (a wildcard version matches any when the
        // policy explicitly permits wildcards).
        if !(version_wildcard && self.config.allow_wildcard) && waiver.version != version {
            return WaiverEvaluation {
                valid: false,
                reason: "waiver is for a different version".to_string(),
                state: Some(waiver.state),
            };
        }

        // Wrong scope -> denied: the waiver may only permit the exact
        // bounded decision configured in policy.
        if !self.config.permitted_scopes.contains(scope) {
            return WaiverEvaluation {
                valid: false,
                reason: "waiver scope not permitted for this decision".to_string(),
                state: Some(waiver.state),
            };
        }

        WaiverEvaluation {
            valid: true,
            reason: "waiver active, exact, and in scope".to_string(),
            state: Some(waiver.state),
        }
    }
}
