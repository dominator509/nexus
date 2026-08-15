//! EP-044 canonical `/healthz` response (ADR-019 `RuntimeHealth`).

use serde::{Deserialize, Serialize};

/// Canonical health response. Must serialize as `{"status":"healthy"}`
/// with HTTP 200 when the runtime is healthy (SPEC-006 health contract).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHealth {
    pub status: String,
}

impl RuntimeHealth {
    /// Healthy response (canonical shape for the runtime smoke).
    pub fn healthy() -> Self {
        Self {
            status: "healthy".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep044_unit_health_shape() {
        let h = RuntimeHealth::healthy();
        assert_eq!(
            serde_json::to_string(&h).unwrap(),
            r#"{"status":"healthy"}"#
        );
    }
}
