//! Component boundary evaluation (SPEC-019 behavior 2; INV-011).
//!
//! Copyleft components (GPL/AGPL -> SIDECAR) run process-separated as
//! independent processes or external appliances, communicate through
//! documented APIs, and preserve notices and source-offer duties. A
//! declared boundary must satisfy isolation before a component is
//! admissible.
//!
//! Deterministic invariants:
//! - SIDECAR class without a process boundary -> denied
//! - SIDECAR boundary without an API contract -> denied
//! - SIDECAR boundary without a source offer -> denied
//! - TRANSITIVE DEPENDENCY != OUT OF SCOPE (never excluded)
//! - test fixture dependency is not safe by default

use nexus_supply_chain::model::{Component, ComponentBoundary};
use nexus_supply_chain::vocabulary::{IntegrationMode, LicenseClass};

/// Deterministic boundary policy configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryPolicyConfig {
    /// Require a documented API contract for every SIDECAR boundary.
    pub require_api_contract: bool,
    /// Require a source offer for every SIDECAR boundary.
    pub require_source_offer: bool,
}

impl Default for BoundaryPolicyConfig {
    fn default() -> Self {
        Self {
            require_api_contract: true,
            require_source_offer: true,
        }
    }
}

/// Outcome of a boundary evaluation for one component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryEvaluation {
    /// True when the component's declared boundary satisfies isolation.
    pub valid: bool,
    /// The license class driving the boundary requirement.
    pub class: Option<LicenseClass>,
    /// The integration mode the component declares.
    pub integration_mode: IntegrationMode,
    /// Deterministic human-safe reason.
    pub reason: String,
}

/// Deterministic boundary policy engine.
#[derive(Debug, Clone)]
pub struct BoundaryPolicy {
    pub config: BoundaryPolicyConfig,
}

impl Default for BoundaryPolicy {
    fn default() -> Self {
        Self::new(BoundaryPolicyConfig::default())
    }
}

impl BoundaryPolicy {
    pub fn new(config: BoundaryPolicyConfig) -> Self {
        Self { config }
    }

    /// Evaluate a component's boundary. `boundary` is the declared sidecar
    /// boundary record when one exists.
    pub fn evaluate(
        &self,
        component: &Component,
        boundary: Option<&ComponentBoundary>,
    ) -> BoundaryEvaluation {
        // Transitive dependencies are always in scope; the caller is
        // responsible for passing every component. This engine never
        // excludes a component because it is transitive or a fixture.
        let class = component.license_class;
        let integration_mode = component.integration_mode;

        match class {
            // SIDECAR (copyleft): must run process-separated with a
            // documented API contract and a source offer.
            Some(LicenseClass::Sidecar) => {
                if integration_mode != IntegrationMode::ProcessSidecar {
                    return BoundaryEvaluation {
                        valid: false,
                        class,
                        integration_mode,
                        reason: "copyleft component must be process-separated (SIDECAR boundary)"
                            .to_string(),
                    };
                }
                let boundary = match boundary {
                    Some(b) => b,
                    None => {
                        return BoundaryEvaluation {
                            valid: false,
                            class,
                            integration_mode,
                            reason: "copyleft component requires a declared sidecar boundary"
                                .to_string(),
                        }
                    }
                };
                if self.config.require_api_contract && boundary.api_contract.trim().is_empty() {
                    return BoundaryEvaluation {
                        valid: false,
                        class,
                        integration_mode,
                        reason: "copyleft sidecar boundary requires a documented API contract"
                            .to_string(),
                    };
                }
                if self.config.require_source_offer {
                    let offer = &boundary.source_offer;
                    if offer.url.trim().is_empty() || offer.version.trim().is_empty() {
                        return BoundaryEvaluation {
                            valid: false,
                            class,
                            integration_mode,
                            reason: "copyleft sidecar boundary requires a source offer".to_string(),
                        };
                    }
                }
                BoundaryEvaluation {
                    valid: true,
                    class,
                    integration_mode,
                    reason: "copyleft sidecar isolation satisfied".to_string(),
                }
            }
            // REVIEW (MPL/LGPL): obligation analysis documented; may be
            // embedded with documented file-level/dynamic-link boundary.
            Some(LicenseClass::Review) => BoundaryEvaluation {
                valid: true,
                class,
                integration_mode,
                reason: "review license boundary accepted with documented obligations".to_string(),
            },
            // EXTERNAL: provider terms govern; must be an external
            // provider integration, never embedded.
            Some(LicenseClass::External) => {
                if integration_mode != IntegrationMode::ExternalProvider {
                    return BoundaryEvaluation {
                        valid: false,
                        class,
                        integration_mode,
                        reason:
                            "external license component must be an external provider integration"
                                .to_string(),
                    };
                }
                BoundaryEvaluation {
                    valid: true,
                    class,
                    integration_mode,
                    reason: "external component boundary accepted under provider terms".to_string(),
                }
            }
            // GREEN / PROHIBITED / None: GREEN may be embedded; PROHIBITED
            // is handled by LicensePolicy (fail closed); None (unknown
            // class) is handled by LicensePolicy (fail closed).
            Some(LicenseClass::Green) | Some(LicenseClass::Prohibited) | None => {
                BoundaryEvaluation {
                    valid: true,
                    class,
                    integration_mode,
                    reason: "no copyleft boundary required for this class".to_string(),
                }
            }
        }
    }
}
