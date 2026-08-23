//! License classification behavior (SPEC-019 behavior 2; LICENSE_POLICY.md).
//!
//! Deterministic invariants:
//! - DEPENDENCY EXISTS != LICENSE APPROVED
//! - LICENSE STRING PRESENT != LICENSE VERIFIED
//! - ALLOWLIST ENTRY != LEGAL APPROVAL FOR ALL USES
//! - UNKNOWN LICENSE != SAFE
//! - MISSING LICENSE != SAFE
//!
//! A GREEN license is permitted only under an exact policy match with an
//! explicit review + approval. REVIEW requires a review/approval state.
//! SIDECAR requires sidecar terms and notice behavior. EXTERNAL is never
//! auto-approved. PROHIBITED, UNKNOWN, and MISSING fail closed. Fuzzy
//! strings never bypass policy.

use nexus_supply_chain::model::Component;
use nexus_supply_chain::vocabulary::{ApprovalState, LicenseClass, LicenseReview};
use nexus_supply_chain::LicenseClassifier;
use nexus_supply_chain::LicenseClassifierPort;

/// Deterministic license policy configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicensePolicyConfig {
    /// Require an exact SPDX match to the canonical policy table. Fuzzy
    /// or partial strings never classify.
    pub exact_match_only: bool,
}

impl Default for LicensePolicyConfig {
    fn default() -> Self {
        Self {
            exact_match_only: true,
        }
    }
}

/// Outcome of a deterministic license evaluation for one component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseEvaluation {
    /// The SPDX string carried by the component (or None when missing).
    pub spdx: Option<String>,
    /// Canonical class when classification succeeded.
    pub class: Option<LicenseClass>,
    /// Review outcome: APPROVED only when the policy permits the exact
    /// component under the exact policy.
    pub review: LicenseReview,
    /// True only when every license gate is explicitly green.
    pub permitted: bool,
    /// Human-safe deterministic reason (never contains secret-shaped
    /// values).
    pub reason: String,
}
/// Deterministic license policy engine.
#[derive(Debug, Clone)]
pub struct LicensePolicy {
    pub config: LicensePolicyConfig,
}

impl Default for LicensePolicy {
    fn default() -> Self {
        Self::new(LicensePolicyConfig::default())
    }
}

impl LicensePolicy {
    pub fn new(config: LicensePolicyConfig) -> Self {
        Self { config }
    }

    /// Evaluate one component's license under the exact policy.
    ///
    /// Fail-closed ladder:
    /// 1. MISSING license -> denied (never safe)
    /// 2. UNKNOWN license  -> denied (never safe)
    /// 3. PROHIBITED class -> denied (never safe)
    /// 4. EXTERNAL         -> denied unless explicitly reviewed+approved
    /// 5. GREEN            -> permitted only when review == APPROVED and
    ///    approval == APPROVED (exact policy match)
    /// 6. REVIEW           -> permitted only when review == APPROVED and
    ///    approval == APPROVED (obligation analysis documented)
    /// 7. SIDECAR          -> permitted only when review == APPROVED and
    ///    approval == APPROVED; the sidecar terms/notice duty is enforced
    ///    by BoundaryPolicy
    pub fn evaluate(&self, component: &Component) -> LicenseEvaluation {
        let classifier = LicenseClassifierPort::new();
        let spdx = component.license_spdx.clone();
        let spdx_ref = match spdx.as_deref() {
            Some(s) if !s.trim().is_empty() => s,
            _ => {
                return LicenseEvaluation {
                    spdx: None,
                    class: None,
                    review: LicenseReview::Denied,
                    permitted: false,
                    reason: "missing license fails closed".to_string(),
                }
            }
        };

        // Exact policy match only: the canonical classifier already
        // rejects anything not in the policy table. Fuzzy strings can
        // never reach a GREEN classification.
        let class = match classifier.classify(spdx_ref) {
            Ok(c) => c,
            Err(_) => {
                return LicenseEvaluation {
                    spdx,
                    class: None,
                    review: LicenseReview::Denied,
                    permitted: false,
                    reason: "unknown license fails closed".to_string(),
                }
            }
        };

        // PROHIBITED fails closed regardless of any other field.
        if class == LicenseClass::Prohibited {
            return LicenseEvaluation {
                spdx,
                class: Some(class),
                review: LicenseReview::Denied,
                permitted: false,
                reason: "prohibited license class fails closed".to_string(),
            };
        }

        // EXTERNAL is never auto-approved.
        if class == LicenseClass::External {
            return LicenseEvaluation {
                spdx,
                class: Some(class),
                review: LicenseReview::Denied,
                permitted: false,
                reason: "external license is never auto-approved".to_string(),
            };
        }

        // GREEN/REVIEW/SIDECAR require the full explicit review ladder:
        // review APPROVED AND approval APPROVED. Presence in an allowlist
        // alone is never approval.
        let reviewed = component.review == LicenseReview::Approved;
        let approved = component.approval == ApprovalState::Approved;
        if !reviewed || !approved {
            let missing = if !reviewed {
                "license review not approved"
            } else {
                "component approval not granted"
            };
            return LicenseEvaluation {
                spdx,
                class: Some(class),
                review: LicenseReview::NeedsReview,
                permitted: false,
                reason: format!("{missing} for {class} license"),
            };
        }

        let reason = match class {
            LicenseClass::Green => "green license permitted under exact policy".to_string(),
            LicenseClass::Review => {
                "review license permitted with documented obligation analysis".to_string()
            }
            LicenseClass::Sidecar => {
                "sidecar license permitted with documented sidecar terms".to_string()
            }
            LicenseClass::External | LicenseClass::Prohibited => {
                "unreachable: handled above".to_string()
            }
        };
        LicenseEvaluation {
            spdx,
            class: Some(class),
            review: LicenseReview::Approved,
            permitted: true,
            reason,
        }
    }
}
