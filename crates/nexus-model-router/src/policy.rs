//! Deterministic route policy (SPEC-009; ADR-022).
//!
//! `RoutePolicy` selects the canonical `Route` from `RoutingFeatures`
//! with deterministic safety floors. It is the only component that can
//! override learned routing for security (SPEC-009 behavior 7; node
//! acceptance obligation 3). The policy is pure: no clock, no random,
//! no network.

use crate::config::RouterPolicyConfig;
use crate::error::RouterError;
use crate::features::RoutingFeatures;
use nexus_domain::vocabulary::{Privacy, Risk, Route};

/// Deterministic route selection policy.
///
/// The policy table comes from `RouterPolicyConfig` (canonical artifact
/// `config/models/router/policy.json`; `new()` uses the code defaults
/// that M2 proves equal the artifact).
#[derive(Debug, Clone, PartialEq)]
pub struct RoutePolicy {
    config: RouterPolicyConfig,
}

/// Risk rank helper (nexus-domain Risk lacks PartialOrd; EP-008
/// precedent). R0=0 .. R4=4.
pub fn risk_rank(risk: Risk) -> u8 {
    match risk {
        Risk::R0 => 0,
        Risk::R1 => 1,
        Risk::R2 => 2,
        Risk::R3 => 3,
        Risk::R4 => 4,
    }
}

impl RoutePolicy {
    pub fn new() -> Self {
        Self::from_config(&RouterPolicyConfig::default())
    }

    pub fn from_config(config: &RouterPolicyConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub fn config(&self) -> &RouterPolicyConfig {
        &self.config
    }

    /// Select the canonical route for the features.
    ///
    /// Order of precedence (safety first):
    /// 1. R4 risk -> REJECT (never routed to a model).
    /// 2. local-only work -> REFLEX (local plane) unless fully
    ///    deterministic (complexity 0) -> DETERMINISTIC.
    /// 3. Deterministic task (complexity 0) -> DETERMINISTIC.
    /// 4. Specialist capability requirement -> SPECIALIST_AGENT.
    /// 5. SECRET privacy or R3 risk -> FRONTIER_API (never CHEAP_API).
    /// 6. Low risk + low complexity + non-sensitive -> CHEAP_API.
    /// 7. High complexity or high availability/certification needs ->
    ///    FRONTIER_API.
    /// 8. Default -> REFLEX.
    pub fn select(&self, features: &RoutingFeatures) -> Result<Route, RouterError> {
        features.validate()?;

        // 1. R4 never routes to a model.
        if features.risk == Risk::R4 {
            return self.config.routes.r4_route.parse().map_err(|_| {
                RouterError::validation(
                    "policy table r4_route invalid",
                    Some("router-policy".into()),
                )
            });
        }

        // 2. Local-only work stays in the local plane.
        if features.local_only {
            if features.complexity == 0.0 {
                return self.config.routes.deterministic_route.parse().map_err(|_| {
                    RouterError::validation(
                        "policy table deterministic_route invalid",
                        Some("router-policy".into()),
                    )
                });
            }
            return self.config.routes.local_route.parse().map_err(|_| {
                RouterError::validation(
                    "policy table local_route invalid",
                    Some("router-policy".into()),
                )
            });
        }

        // 3. Deterministic tasks bypass the model entirely.
        if features.complexity == 0.0 {
            return self.config.routes.deterministic_route.parse().map_err(|_| {
                RouterError::validation(
                    "policy table deterministic_route invalid",
                    Some("router-policy".into()),
                )
            });
        }

        // 4. Specialist capability -> specialist agent.
        if let Some(cap) = &features.capability {
            if cap.starts_with(&self.config.routes.specialist_prefix) {
                return Ok(Route::SpecialistAgent);
            }
        }

        // 5. SECRET privacy or R3 risk -> frontier, never cheap.
        if features.privacy == Privacy::Secret || features.risk == Risk::R3 {
            return self.config.routes.secret_route.parse().map_err(|_| {
                RouterError::validation(
                    "policy table secret_route invalid",
                    Some("router-policy".into()),
                )
            });
        }

        // 6. Low risk + low complexity + non-sensitive -> cheap API.
        if risk_rank(features.risk)
            <= risk_rank(
                self.config
                    .thresholds
                    .cheap_max_risk
                    .parse()
                    .unwrap_or(Risk::R1),
            )
            && features.complexity <= self.config.thresholds.cheap_max_complexity
            && features.cost < self.config.thresholds.cheap_max_cost
        {
            return Ok(Route::CheapApi);
        }

        // 7. High complexity or strong availability/certification needs.
        if features.complexity >= self.config.thresholds.frontier_min_complexity
            || features.availability >= self.config.thresholds.frontier_min_availability
            || features.requires_certified
        {
            return Ok(Route::FrontierApi);
        }

        // 8. Default reflex plane.
        self.config.routes.default_route.parse().map_err(|_| {
            RouterError::validation(
                "policy table default_route invalid",
                Some("router-policy".into()),
            )
        })
    }

    /// Security override: given a route proposed by a learned scorer or
    /// other advisory strategy, return the policy-correct route when the
    /// proposal violates a safety floor. The policy engine can override
    /// learned routing for security (acceptance obligation 3).
    pub fn override_security(
        &self,
        features: &RoutingFeatures,
        proposed: Route,
    ) -> Result<Route, RouterError> {
        let policy_route = self.select(features)?;

        // R4 never routes to a model regardless of what any learned
        // scorer proposes.
        if features.risk == Risk::R4 {
            return Ok(Route::Reject);
        }

        // SECRET privacy never routes to CHEAP_API.
        if features.privacy == Privacy::Secret && proposed == Route::CheapApi {
            return Ok(policy_route);
        }

        // R3 risk never routes to CHEAP_API.
        if features.risk == Risk::R3 && proposed == Route::CheapApi {
            return Ok(policy_route);
        }

        // Local-only work is never served by a remote route.
        if features.local_only && matches!(proposed, Route::CheapApi | Route::FrontierApi) {
            return Ok(policy_route);
        }

        // Deterministic tasks never route to a model.
        if features.complexity == 0.0 && proposed != Route::Deterministic {
            return Ok(Route::Deterministic);
        }

        Ok(proposed)
    }
}

impl Default for RoutePolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features() -> RoutingFeatures {
        RoutingFeatures::new(
            "contacts.query",
            0.2,
            Privacy::Personal,
            Risk::R1,
            Some("contacts.query".into()),
            0.3,
            500,
            false,
            0.99,
            0.95,
            true,
            Some(1000),
        )
    }

    #[test]
    fn ep015_unit_policy_low_risk_low_complexity_routes_cheap() {
        assert_eq!(
            RoutePolicy::new().select(&features()).unwrap(),
            Route::CheapApi
        );
    }

    #[test]
    fn ep015_unit_policy_deterministic_task_bypasses_model() {
        let mut f = features();
        f.complexity = 0.0;
        assert_eq!(RoutePolicy::new().select(&f).unwrap(), Route::Deterministic);
    }

    #[test]
    fn ep015_unit_policy_r4_never_routes_to_model() {
        let mut f = features();
        f.risk = Risk::R4;
        assert_eq!(RoutePolicy::new().select(&f).unwrap(), Route::Reject);
        // A learned proposal cannot override the R4 floor.
        let policy = RoutePolicy::new();
        assert_eq!(
            policy.override_security(&f, Route::FrontierApi).unwrap(),
            Route::Reject
        );
    }

    #[test]
    fn ep015_unit_policy_secret_privacy_never_cheap() {
        let mut f = features();
        f.privacy = Privacy::Secret;
        let policy = RoutePolicy::new();
        assert_eq!(policy.select(&f).unwrap(), Route::FrontierApi);
        // Learned proposal for CHEAP_API is overridden for security.
        assert_eq!(
            policy.override_security(&f, Route::CheapApi).unwrap(),
            Route::FrontierApi
        );
    }

    #[test]
    fn ep015_unit_policy_local_only_stays_local() {
        let mut f = features();
        f.local_only = true;
        assert_eq!(RoutePolicy::new().select(&f).unwrap(), Route::Reflex);
        let policy = RoutePolicy::new();
        assert_eq!(
            policy.override_security(&f, Route::FrontierApi).unwrap(),
            Route::Reflex
        );
    }

    #[test]
    fn ep015_unit_policy_specialist_capability_routes_specialist() {
        let mut f = features();
        f.capability = Some("specialist.legal".into());
        f.complexity = 0.6;
        assert_eq!(
            RoutePolicy::new().select(&f).unwrap(),
            Route::SpecialistAgent
        );
    }

    #[test]
    fn ep015_unit_policy_high_complexity_routes_frontier() {
        let mut f = features();
        f.complexity = 0.8;
        assert_eq!(RoutePolicy::new().select(&f).unwrap(), Route::FrontierApi);
    }

    #[test]
    fn ep015_unit_policy_rejects_out_of_range_features() {
        let mut f = features();
        f.complexity = 2.0;
        assert!(RoutePolicy::new().select(&f).is_err());
    }
}
