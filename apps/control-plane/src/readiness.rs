//! EP-044 canonical `/readyz` response (ADR-019 `RuntimeReadiness`).

use serde::{Deserialize, Serialize};

/// Canonical readiness response. Must serialize as `{"ready":true}`
/// with HTTP 200 when the runtime is ready (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeReadiness {
    pub ready: bool,
}

impl RuntimeReadiness {
    /// Ready response (canonical shape for the runtime smoke).
    pub fn ready() -> Self {
        Self { ready: true }
    }

    /// Not-ready response (fail closed).
    pub fn not_ready() -> Self {
        Self { ready: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep044_unit_readiness_shape() {
        let r = RuntimeReadiness::ready();
        assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"ready":true}"#);
        let nr = RuntimeReadiness::not_ready();
        assert_eq!(serde_json::to_string(&nr).unwrap(), r#"{"ready":false}"#);
    }
}
