//! EP-044 canonical runtime configuration (ADR-019 `ControlPlaneConfig`).

use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeErrorCode};

/// Canonical runtime configuration for the Nexus Control Plane Runtime.
///
/// Provider-neutral. Never carries secrets; secrets travel as
/// references elsewhere (SPEC-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneConfig {
    /// Canonical base domain, e.g. `nexus.test` (PREFLIGHT/`NEXUS_BASE_DOMAIN`).
    pub base_domain: String,
    /// Bind address, e.g. `127.0.0.1:8443`.
    pub bind_address: String,
    /// Default tenant used for the composition root capability list.
    pub tenant_id: String,
    /// Canonical capability list source descriptor (provider-neutral).
    pub capability_source: String,
}

/// Configuration construction/validation error (SPEC-006 typed error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneConfigError(pub String);

impl std::fmt::Display for ControlPlaneConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid control plane config: {}", self.0)
    }
}

impl std::error::Error for ControlPlaneConfigError {}

impl From<ControlPlaneConfigError> for RuntimeError {
    fn from(value: ControlPlaneConfigError) -> Self {
        RuntimeError::new(RuntimeErrorCode::InvalidConfiguration, value.0, None)
    }
}

impl ControlPlaneConfig {
    /// Build and validate a runtime config. Empty or malformed fields are
    /// rejected (fail closed; SPEC-006).
    pub fn new(
        base_domain: impl Into<String>,
        bind_address: impl Into<String>,
        tenant_id: impl Into<String>,
        capability_source: impl Into<String>,
    ) -> Result<Self, ControlPlaneConfigError> {
        let cfg = Self {
            base_domain: base_domain.into(),
            bind_address: bind_address.into(),
            tenant_id: tenant_id.into(),
            capability_source: capability_source.into(),
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate all fields are non-empty and the bind address is
    /// well-formed (`host:port`).
    pub fn validate(&self) -> Result<(), ControlPlaneConfigError> {
        if self.base_domain.trim().is_empty() {
            return Err(ControlPlaneConfigError("base_domain is empty".into()));
        }
        if self.bind_address.trim().is_empty() {
            return Err(ControlPlaneConfigError("bind_address is empty".into()));
        }
        if self.tenant_id.trim().is_empty() {
            return Err(ControlPlaneConfigError("tenant_id is empty".into()));
        }
        if self.capability_source.trim().is_empty() {
            return Err(ControlPlaneConfigError("capability_source is empty".into()));
        }
        if !self.bind_address.contains(':') {
            return Err(ControlPlaneConfigError(
                "bind_address must be host:port".into(),
            ));
        }
        Ok(())
    }

    /// Canonical base URL derived from the configured domain
    /// (`https://<base_domain>`), matching the runtime smoke convention
    /// `NEXUS_SMOKE_URL`/`NEXUS_BASE_DOMAIN`.
    pub fn base_url(&self) -> String {
        format!("https://{}", self.base_domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep044_unit_config_valid() {
        let cfg = ControlPlaneConfig::new(
            "nexus.test",
            "127.0.0.1:8443",
            "018f0f6f-9c1e-7b6e-8000-000000000001",
            "core",
        )
        .unwrap();
        assert_eq!(cfg.base_url(), "https://nexus.test");
        assert_eq!(cfg.bind_address, "127.0.0.1:8443");
    }

    #[test]
    fn ep044_unit_config_rejects_empty_base_domain() {
        let err = ControlPlaneConfig::new(
            "",
            "127.0.0.1:8443",
            "018f0f6f-9c1e-7b6e-8000-000000000001",
            "core",
        )
        .unwrap_err();
        assert!(err.0.contains("base_domain"));
    }

    #[test]
    fn ep044_unit_config_rejects_bad_bind_address() {
        let err = ControlPlaneConfig::new(
            "nexus.test",
            "noport",
            "018f0f6f-9c1e-7b6e-8000-000000000001",
            "core",
        )
        .unwrap_err();
        assert!(err.0.contains("host:port"));
    }

    #[test]
    fn ep044_unit_config_serde_round_trip() {
        let cfg = ControlPlaneConfig::new(
            "nexus.test",
            "127.0.0.1:8443",
            "018f0f6f-9c1e-7b6e-8000-000000000001",
            "core",
        )
        .unwrap();
        let wire = serde_json::to_string(&cfg).unwrap();
        let back: ControlPlaneConfig = serde_json::from_str(&wire).unwrap();
        assert_eq!(back, cfg);
    }
}
