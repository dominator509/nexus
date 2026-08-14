//! Credential broker reference scope (directive N).
//!
//! Connectors never receive generic credentials. The sidecar holds an
//! explicit scope table: which connector may reference which
//! credential reference. Scope enforcement happens before provider
//! invocation; the provider resolves the actual value only inside its
//! sandbox. A credential VALUE never enters the sidecar, the wire,
//! telemetry, or error bodies.

use std::collections::BTreeMap;

use crate::error::{SidecarError, SidecarErrorKind};

/// Reference-scoped credential table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CredentialScope {
    /// connector_id -> allowed reference prefixes.
    allowed: BTreeMap<String, Vec<String>>,
}

impl CredentialScope {
    /// Construct an empty scope.
    pub fn new() -> Self {
        Self {
            allowed: BTreeMap::new(),
        }
    }

    /// Grant a connector access to one credential reference (exact or
    /// prefix; canonical references are `vault:name` / `broker:name`).
    pub fn grant(&mut self, connector_id: impl Into<String>, reference_prefix: impl Into<String>) {
        self.allowed
            .entry(connector_id.into())
            .or_default()
            .push(reference_prefix.into());
    }

    /// True when the connector may reference the credential.
    pub fn permits(&self, connector_id: &str, reference: &str) -> bool {
        self.allowed
            .get(connector_id)
            .map(|prefixes| {
                prefixes
                    .iter()
                    .any(|p| reference == p || reference.starts_with(p))
            })
            .unwrap_or(false)
    }

    /// Enforce scope for a credential reference (directive N).
    ///
    /// The connector must be permitted to reference this credential;
    /// otherwise a typed credential denial is returned and the
    /// provider is never invoked.
    pub fn enforce(
        &self,
        connector_id: &str,
        reference: &str,
        correlation_id: Option<&str>,
    ) -> Result<(), SidecarError> {
        if !reference.starts_with("vault:") && !reference.starts_with("broker:") {
            return Err(SidecarError::new(
                SidecarErrorKind::CredentialDenied,
                "credential reference must be namespaced (vault: or broker:)",
                correlation_id.map(str::to_string),
                None,
                Some(reference.to_string()),
            ));
        }
        if self.permits(connector_id, reference) {
            Ok(())
        } else {
            Err(SidecarError::new(
                SidecarErrorKind::CredentialDenied,
                "credential reference not permitted for this connector",
                correlation_id.map(str::to_string),
                None,
                Some(reference.to_string()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> CredentialScope {
        let mut s = CredentialScope::new();
        s.grant("fixture-connector", "vault:fixture-token");
        s
    }

    #[test]
    fn ep011_unit_sidecar_credential_scope_permits_own_reference() {
        let s = scope();
        assert!(
            s.enforce("fixture-connector", "vault:fixture-token", None)
                .is_ok()
        );
    }

    #[test]
    fn ep011_unit_sidecar_credential_scope_denies_wrong_connector() {
        let s = scope();
        let err = s
            .enforce("other-connector", "vault:fixture-token", None)
            .unwrap_err();
        assert_eq!(err.kind, SidecarErrorKind::CredentialDenied);
    }

    #[test]
    fn ep011_unit_sidecar_credential_scope_denies_other_reference() {
        let s = scope();
        let err = s
            .enforce("fixture-connector", "vault:other-secret", None)
            .unwrap_err();
        assert_eq!(err.kind, SidecarErrorKind::CredentialDenied);
    }

    #[test]
    fn ep011_unit_sidecar_credential_scope_rejects_unnamespaced() {
        let s = scope();
        let err = s
            .enforce("fixture-connector", "plain-token", None)
            .unwrap_err();
        assert!(err.message.contains("namespaced"));
    }
}
