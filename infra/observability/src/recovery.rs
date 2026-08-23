//! Bounded recovery command (SPEC-007 behavior 8; EP-037 M4 precedent:
//! deadline-based readiness, bounded backoff, attempt counter, last
//! observed failure).
//!
//! Recovery NEVER retries unboundedly. `RecoveryBudget` carries a
//! monotonic deadline and a maximum attempt count; every attempt
//! records the last observed failure so a budget-exhausted recovery
//! fails closed with a truthful, diagnosable reason.
//!
//! Only proven-transient failure classes are retried. The caller
//! supplies the classifier; this module never silently converts a
//! permanent failure (authorization, policy, malformed input) into a
//! retryable one.

use std::time::{Duration, Instant};

use nexus_observability::ObservabilityErrorCode;

/// A failure that recovery may treat as transient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryVerdict {
    /// The attempt succeeded; recovery is complete.
    Success,
    /// The attempt failed but may be retried inside the budget.
    Retryable { detail: String },
    /// The attempt failed permanently; recovery must stop now.
    Permanent { detail: String },
}

/// Result of a bounded recovery run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryOutcome {
    /// True only when an attempt returned Success.
    pub recovered: bool,
    /// Attempts actually made (0 means the budget was invalid).
    pub attempts: u32,
    /// Last observed failure (never contains secrets by contract of the
    /// caller-supplied classifier).
    pub last_failure: Option<String>,
    /// Total wall time the recovery loop spent.
    pub elapsed: Duration,
    /// True when the budget was exhausted without recovery.
    pub budget_exhausted: bool,
}

impl RecoveryOutcome {
    pub fn describe(&self) -> String {
        if self.recovered {
            format!("recovered after {} attempts", self.attempts)
        } else if self.budget_exhausted {
            format!(
                "budget exhausted after {} attempts; last failure: {}",
                self.attempts,
                self.last_failure.as_deref().unwrap_or("none")
            )
        } else {
            format!(
                "not recovered after {} attempts; last failure: {}",
                self.attempts,
                self.last_failure.as_deref().unwrap_or("none")
            )
        }
    }
}

/// Bounded recovery budget: monotonic deadline + max attempts + backoff.
#[derive(Debug, Clone)]
pub struct RecoveryBudget {
    /// Hard deadline from the moment `run` starts.
    pub max_elapsed: Duration,
    /// Maximum number of attempts (including the first).
    pub max_attempts: u32,
    /// Sleep between attempts (bounded backoff, fixed in this revision).
    pub backoff: Duration,
}

impl Default for RecoveryBudget {
    fn default() -> Self {
        Self {
            max_elapsed: Duration::from_secs(30),
            max_attempts: 6,
            backoff: Duration::from_millis(500),
        }
    }
}

/// Run one attempt, deciding retry vs permanent via `classify`.
pub type AttemptFn<'a> = Box<dyn FnMut() -> RecoveryVerdict + 'a>;

/// Execute bounded recovery.
///
/// `attempt` is called at most `budget.max_attempts` times and only
/// while the monotonic deadline has not passed. Permanent failures stop
/// immediately. Retryable failures continue until the budget is
/// exhausted, then the outcome reports `budget_exhausted`.
pub fn recover_with_budget(budget: &RecoveryBudget, mut attempt: AttemptFn<'_>) -> RecoveryOutcome {
    let start = Instant::now();
    let mut attempts = 0u32;
    let mut last_failure: Option<String> = None;

    if budget.max_attempts == 0 || budget.max_elapsed.is_zero() {
        return RecoveryOutcome {
            recovered: false,
            attempts,
            last_failure: Some("invalid budget".to_string()),
            elapsed: start.elapsed(),
            budget_exhausted: false,
        };
    }

    while attempts < budget.max_attempts {
        if start.elapsed() >= budget.max_elapsed {
            break;
        }
        attempts += 1;
        match attempt() {
            RecoveryVerdict::Success => {
                return RecoveryOutcome {
                    recovered: true,
                    attempts,
                    last_failure,
                    elapsed: start.elapsed(),
                    budget_exhausted: false,
                }
            }
            RecoveryVerdict::Retryable { detail } => {
                last_failure = Some(detail);
                if attempts < budget.max_attempts {
                    std::thread::sleep(budget.backoff);
                }
            }
            RecoveryVerdict::Permanent { detail } => {
                return RecoveryOutcome {
                    recovered: false,
                    attempts,
                    last_failure: Some(detail),
                    elapsed: start.elapsed(),
                    budget_exhausted: false,
                }
            }
        }
    }
    // We exited the loop without Success: the retry budget (deadline or
    // attempt limit) is exhausted. Fail closed with a truthful flag.
    RecoveryOutcome {
        recovered: false,
        attempts,
        last_failure,
        elapsed: start.elapsed(),
        budget_exhausted: true,
    }
}

/// Map a SPEC-006 transport failure kind to a recovery verdict.
///
/// Transient (retryable): Unavailable, Timeout, ExternalProvider.
/// Permanent: Authorization, NotFound, RateLimit (policy/permission
/// classes are never auto-retried).
pub fn classify_recovery(kind: &ObservabilityErrorCode, detail: String) -> RecoveryVerdict {
    match kind {
        ObservabilityErrorCode::Unavailable
        | ObservabilityErrorCode::Timeout
        | ObservabilityErrorCode::ExternalProvider => RecoveryVerdict::Retryable { detail },
        _ => RecoveryVerdict::Permanent { detail },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep038_failure_recovery_succeeds_within_budget() {
        let budget = RecoveryBudget {
            max_elapsed: Duration::from_secs(5),
            max_attempts: 3,
            backoff: Duration::from_millis(10),
        };
        let mut calls = 0;
        let outcome = recover_with_budget(
            &budget,
            Box::new(move || {
                calls += 1;
                if calls < 2 {
                    RecoveryVerdict::Retryable {
                        detail: "not yet".to_string(),
                    }
                } else {
                    RecoveryVerdict::Success
                }
            }),
        );
        assert!(outcome.recovered);
        assert_eq!(outcome.attempts, 2);
        assert!(!outcome.budget_exhausted);
    }

    #[test]
    fn ep038_failure_recovery_permanent_stops_immediately() {
        let budget = RecoveryBudget {
            max_elapsed: Duration::from_secs(5),
            max_attempts: 5,
            backoff: Duration::from_millis(10),
        };
        let outcome = recover_with_budget(
            &budget,
            Box::new(|| RecoveryVerdict::Permanent {
                detail: "authorization denied".to_string(),
            }),
        );
        assert!(!outcome.recovered);
        assert_eq!(outcome.attempts, 1);
        assert!(!outcome.budget_exhausted);
        assert_eq!(
            outcome.last_failure.as_deref(),
            Some("authorization denied")
        );
    }

    #[test]
    fn ep038_failure_recovery_budget_exhaustion_fails_closed() {
        let budget = RecoveryBudget {
            max_elapsed: Duration::from_secs(5),
            max_attempts: 3,
            backoff: Duration::from_millis(5),
        };
        let outcome = recover_with_budget(
            &budget,
            Box::new(|| RecoveryVerdict::Retryable {
                detail: "still down".to_string(),
            }),
        );
        assert!(!outcome.recovered);
        assert_eq!(outcome.attempts, 3);
        assert!(outcome.budget_exhausted);
        let d = outcome.describe();
        assert!(d.contains("budget exhausted"));
        assert!(d.contains("still down"));
    }

    #[test]
    fn ep038_failure_recovery_zero_budget_invalid() {
        let budget = RecoveryBudget {
            max_elapsed: Duration::ZERO,
            max_attempts: 0,
            backoff: Duration::ZERO,
        };
        let outcome = recover_with_budget(&budget, Box::new(|| RecoveryVerdict::Success));
        assert!(!outcome.recovered);
        assert_eq!(outcome.attempts, 0);
    }

    #[test]
    fn ep038_failure_classify_transient_vs_permanent() {
        assert!(matches!(
            classify_recovery(&ObservabilityErrorCode::Unavailable, "x".to_string()),
            RecoveryVerdict::Retryable { .. }
        ));
        assert!(matches!(
            classify_recovery(&ObservabilityErrorCode::Timeout, "x".to_string()),
            RecoveryVerdict::Retryable { .. }
        ));
        assert!(matches!(
            classify_recovery(&ObservabilityErrorCode::ExternalProvider, "x".to_string()),
            RecoveryVerdict::Retryable { .. }
        ));
        assert!(matches!(
            classify_recovery(&ObservabilityErrorCode::Authorization, "x".to_string()),
            RecoveryVerdict::Permanent { .. }
        ));
        assert!(matches!(
            classify_recovery(&ObservabilityErrorCode::RedactionDenied, "x".to_string()),
            RecoveryVerdict::Permanent { .. }
        ));
    }
}
