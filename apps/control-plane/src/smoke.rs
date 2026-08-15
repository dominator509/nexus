//! EP-044 runtime smoke contract (ADR-020 `RuntimeSmoke`).

use crate::error::{RuntimeError, RuntimeErrorCode};

/// Canonical runtime smoke contract, owned by EP-044.
///
/// The smoke gate activates only at `at-least EP-044`; before the owner
/// is DONE the stage is `not-applicable-before EP-044`; after the owner
/// is DONE the smoke is mandatory and fails closed when the runtime is
/// absent or unhealthy. The assertions themselves are never weakened.
pub struct RuntimeSmoke {
    /// Canonical base URL derived from the runtime config.
    base_url: String,
}

/// Runtime smoke result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeResult {
    pub health_ok: bool,
    pub readiness_ok: bool,
    pub capabilities_ok: bool,
    pub all_ok: bool,
}

impl SmokeResult {
    pub fn ok() -> Self {
        Self {
            health_ok: true,
            readiness_ok: true,
            capabilities_ok: true,
            all_ok: true,
        }
    }
}

/// Runtime smoke error (fail closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSmokeError(pub String);

impl std::fmt::Display for RuntimeSmokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime smoke: {}", self.0)
    }
}

impl std::error::Error for RuntimeSmokeError {}

impl From<RuntimeSmokeError> for RuntimeError {
    fn from(value: RuntimeSmokeError) -> Self {
        RuntimeError::new(RuntimeErrorCode::Unavailable, value.0, None)
    }
}

impl RuntimeSmoke {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Deterministic in-process smoke evaluation against provided probe
    /// results. The real live-fire proof (`LF-029`) drives the real
    /// server over real HTTP; this contract method is the typed decision
    /// surface used by tests and the composition root.
    pub fn evaluate(
        &self,
        health_ok: bool,
        readiness_ok: bool,
        capabilities_ok: bool,
    ) -> Result<SmokeResult, RuntimeSmokeError> {
        let result = SmokeResult {
            health_ok,
            readiness_ok,
            capabilities_ok,
            all_ok: health_ok && readiness_ok && capabilities_ok,
        };
        if result.all_ok {
            Ok(result)
        } else {
            Err(RuntimeSmokeError(
                "one or more canonical runtime probes failed (fail closed)".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep044_unit_smoke_ok() {
        let smoke = RuntimeSmoke::new("https://nexus.test");
        assert_eq!(smoke.base_url(), "https://nexus.test");
        let result = smoke.evaluate(true, true, true).unwrap();
        assert!(result.all_ok);
        assert_eq!(result, SmokeResult::ok());
    }

    #[test]
    fn ep044_unit_smoke_fails_closed_on_any_probe() {
        let smoke = RuntimeSmoke::new("https://nexus.test");
        let err = smoke.evaluate(true, false, true).unwrap_err();
        assert!(err.0.contains("fail closed"));
    }
}
