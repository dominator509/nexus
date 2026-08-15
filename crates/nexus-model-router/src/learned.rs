//! Learned router adapter port (SPEC-009; ADR-022).
//!
//! RouteLLM and LLMRouter are replaceable strategies behind this port.
//! A learned adapter contributes advisory scores only; the deterministic
//! `RoutePolicy` can override them for security (acceptance obligation
//! 3). The port is provider-neutral and versioned.

use crate::error::RouterError;
use crate::features::RoutingFeatures;
use crate::vocabulary::RouterStrategyClass;
use serde::{Deserialize, Serialize};

/// Learned scores for the candidate routes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearnedScores {
    pub strategy: RouterStrategyClass,
    /// Per-route scores in 0..=1 (higher = preferred).
    pub scores: Vec<RouteScore>,
    /// Out-of-distribution flag: the scorer is uncertain and the
    /// request should escalate rather than trust the scores.
    pub out_of_distribution: bool,
}

/// One scored candidate route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteScore {
    pub route: String,
    pub score: u64,
}

impl LearnedScores {
    pub fn new(
        strategy: RouterStrategyClass,
        scores: Vec<RouteScore>,
        out_of_distribution: bool,
    ) -> Self {
        Self {
            strategy,
            scores,
            out_of_distribution,
        }
    }

    /// Best-scoring route by score, if any.
    pub fn best(&self) -> Option<&RouteScore> {
        self.scores.iter().max_by_key(|s| s.score)
    }
}

/// Learned router adapter port (RouteLLM / LLMRouter replaceable).
pub trait LearnedRouterAdapter: std::fmt::Debug + Send {
    /// Score the features for candidate routes. Advisory only.
    fn score(&mut self, features: &RoutingFeatures) -> Result<LearnedScores, RouterError>;

    /// Strategy identity recorded on routing decisions.
    fn strategy(&self) -> RouterStrategyClass;
}

/// Test probe adapters shared with the router tests (TESTING.md test
/// zone; never used in production paths).
#[cfg(test)]
pub mod tests_probe {
    use super::*;

    #[derive(Debug)]
    pub struct OkAdapter;

    impl LearnedRouterAdapter for OkAdapter {
        fn score(&mut self, _features: &RoutingFeatures) -> Result<LearnedScores, RouterError> {
            Ok(LearnedScores::new(
                RouterStrategyClass::RouteLlm,
                vec![RouteScore {
                    route: "REFLEX".into(),
                    score: 80,
                }],
                false,
            ))
        }

        fn strategy(&self) -> RouterStrategyClass {
            RouterStrategyClass::RouteLlm
        }
    }

    #[derive(Debug)]
    pub struct CheapProposingAdapter;

    impl LearnedRouterAdapter for CheapProposingAdapter {
        fn score(&mut self, _features: &RoutingFeatures) -> Result<LearnedScores, RouterError> {
            Ok(LearnedScores::new(
                RouterStrategyClass::RouteLlm,
                vec![RouteScore {
                    route: "CHEAP_API".into(),
                    score: 90,
                }],
                false,
            ))
        }

        fn strategy(&self) -> RouterStrategyClass {
            RouterStrategyClass::RouteLlm
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep015_unit_learned_scores_best() {
        let scores = LearnedScores::new(
            RouterStrategyClass::RouteLlm,
            vec![
                RouteScore {
                    route: "REFLEX".into(),
                    score: 70,
                },
                RouteScore {
                    route: "FRONTIER_API".into(),
                    score: 90,
                },
            ],
            false,
        );
        assert_eq!(scores.best().unwrap().route, "FRONTIER_API");
    }

    #[test]
    fn ep015_unit_learned_scores_serde_round_trip() {
        let scores = LearnedScores::new(
            RouterStrategyClass::LlmRouter,
            vec![RouteScore {
                route: "CHEAP_API".into(),
                score: 50,
            }],
            true,
        );
        let v = serde_json::to_value(&scores).unwrap();
        let back: LearnedScores = serde_json::from_value(v).unwrap();
        assert_eq!(back, scores);
        assert!(back.out_of_distribution);
    }
}
