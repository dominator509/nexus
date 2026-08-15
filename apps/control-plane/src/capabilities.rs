//! EP-044 canonical `/v1/capabilities` response (ADR-019 `CapabilityList`).

use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeErrorCode};

/// Canonical capability list response. Must serialize as
/// `{"capabilities":[...]}` with a non-empty list when the runtime is
/// ready (SPEC-003 discovery contract).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityList {
    pub capabilities: Vec<String>,
}

impl CapabilityList {
    pub fn new(capabilities: Vec<String>) -> Self {
        Self { capabilities }
    }

    /// True when the list is non-empty (readiness requirement).
    pub fn is_non_empty(&self) -> bool {
        !self.capabilities.is_empty()
    }
}

/// Provider-neutral capability list source (the composition root wires a
/// real implementation; the server never fabricates capabilities).
pub trait CapabilityListSource {
    /// Resolve the canonical capability keys for the runtime. Returns an
    /// error when the source is unavailable (fail closed).
    fn list(&self) -> Result<Vec<String>, RuntimeError>;
}

/// Deterministic in-memory capability list source seeded from the
/// canonical runtime config. This is a real composition-root adapter:
/// the list is derived from the configured source descriptor and never
/// invented at request time.
#[derive(Debug, Clone)]
pub struct ConfiguredCapabilityList {
    source: String,
    keys: Vec<String>,
}

impl ConfiguredCapabilityList {
    pub fn new(source: impl Into<String>, keys: Vec<String>) -> Self {
        Self {
            source: source.into(),
            keys,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

impl CapabilityListSource for ConfiguredCapabilityList {
    fn list(&self) -> Result<Vec<String>, RuntimeError> {
        if self.keys.is_empty() {
            return Err(RuntimeError::new(
                RuntimeErrorCode::Unavailable,
                "capability source is empty",
                None,
            ));
        }
        Ok(self.keys.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep044_unit_capability_list_shape() {
        let list = CapabilityList::new(vec!["health".to_string()]);
        assert_eq!(
            serde_json::to_string(&list).unwrap(),
            r#"{"capabilities":["health"]}"#
        );
        assert!(list.is_non_empty());
    }

    #[test]
    fn ep044_unit_capability_source_returns_keys() {
        let source =
            ConfiguredCapabilityList::new("core", vec!["health".into(), "capabilities".into()]);
        assert_eq!(source.source(), "core");
        let keys = source.list().unwrap();
        assert_eq!(keys, vec!["health", "capabilities"]);
    }

    #[test]
    fn ep044_unit_capability_source_empty_fails_closed() {
        let source = ConfiguredCapabilityList::new("core", vec![]);
        let err = source.list().unwrap_err();
        assert_eq!(err.code, RuntimeErrorCode::Unavailable);
    }
}
