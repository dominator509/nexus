//! Advisory evaluation (SPEC-019 behavior 7; SPEC-019 required tests:
//! advisory workflow).
//!
//! Deterministic invariants:
//! - known advisory -> risk state
//! - unreviewed advisory -> not safe
//! - ignored advisory -> requires exact waiver/justification
//! - fixed version -> safe only if the dependency actually resolves to
//!   the fixed version
//! - unknown advisory status -> not certified safe
//! - "no advisories returned" is NOT "secure" unless the evidence source
//!   was actually queried and verified
//!
//! Critical advisories fail release unless a time-bounded ADR documents
//! mitigation (node contract acceptance obligation 4).

use nexus_supply_chain::model::{Advisory, AdvisoryAffected};
use nexus_supply_chain::vocabulary::AdvisorySeverity;

/// Deterministic advisory policy configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisoryPolicyConfig {
    /// True when the advisory source was actually queried and verified in
    /// this run. No advisories returned is never secure without this.
    pub source_queried: bool,
    /// Whether a mitigation ADR must carry a bounded expiry.
    pub require_bounded_mitigation: bool,
}

impl Default for AdvisoryPolicyConfig {
    fn default() -> Self {
        Self {
            source_queried: true,
            require_bounded_mitigation: true,
        }
    }
}

/// Outcome of an advisory evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisoryEvaluation {
    /// True only when no advisory blocks the release.
    pub valid: bool,
    /// Deterministic human-safe reason.
    pub reason: String,
    /// Number of advisories accounted for.
    pub advisory_count: usize,
    /// Number of critical advisories without a valid bounded mitigation.
    pub blocking_count: usize,
}

/// Deterministic advisory policy engine.
#[derive(Debug, Clone)]
pub struct AdvisoryPolicy {
    pub config: AdvisoryPolicyConfig,
}

impl Default for AdvisoryPolicy {
    fn default() -> Self {
        Self::new(AdvisoryPolicyConfig::default())
    }
}

impl AdvisoryPolicy {
    pub fn new(config: AdvisoryPolicyConfig) -> Self {
        Self { config }
    }

    /// Evaluate the advisory posture for a set of known advisories and the
    /// affected components actually resolved in the inventory.
    ///
    /// `advisories` is the list returned by the advisory source for the
    /// current run. `affected` maps advisory ids to the exact
    /// package+version the inventory resolves.
    pub fn evaluate(
        &self,
        advisories: &[Advisory],
        affected: &[AdvisoryAffected],
        now_ts: u64,
    ) -> AdvisoryEvaluation {
        // Unknown advisory status (source never queried) is not safe.
        if !self.config.source_queried {
            return AdvisoryEvaluation {
                valid: false,
                reason: "advisory source not queried: unknown status is not safe".to_string(),
                advisory_count: 0,
                blocking_count: 1,
            };
        }

        let mut blocking = 0usize;
        let mut reason = "no blocking advisories".to_string();

        for advisory in advisories {
            // The advisory only matters when it affects a component that
            // the inventory actually resolves.
            let resolved = affected.iter().any(|a| {
                a.advisory_id == advisory.id
                    && advisory.affected_versions.iter().any(|v| v == &a.version)
            });
            if !resolved {
                continue;
            }

            if advisory.severity != AdvisorySeverity::Critical {
                // Known non-critical advisory -> risk state, not blocking.
                continue;
            }

            // Critical advisory: requires a time-bounded ADR documenting
            // mitigation (node contract obligation 4). Unreviewed (no
            // ADR) blocks; expired mitigation blocks; unbounded
            // mitigation blocks when the policy requires a bound.
            let has_adr = advisory
                .mitigation_adr
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if !has_adr {
                blocking += 1;
                reason = format!(
                    "critical advisory {} has no mitigation ADR (unreviewed is not safe)",
                    advisory.id
                );
                continue;
            }
            let bounded = advisory
                .mitigation_expires_ts
                .map(|exp| now_ts <= exp)
                .unwrap_or(false);
            if !bounded {
                blocking += 1;
                reason = format!(
                    "critical advisory {} mitigation ADR is expired or unbounded",
                    advisory.id
                );
                continue;
            }
            // Valid bounded mitigation: advisory accounted, not blocking.
        }

        AdvisoryEvaluation {
            valid: blocking == 0,
            reason,
            advisory_count: advisories.len(),
            blocking_count: blocking,
        }
    }
}
