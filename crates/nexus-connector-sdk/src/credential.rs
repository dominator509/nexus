//! Credential broker boundary (SPEC-022 behavior 7, SPEC-020).
//!
//! Connectors never receive generic credentials and never embed
//! secrets in prompts or manifests. The `CredentialBroker` port
//! hands out references and resolves references to values only
//! inside the sandbox at execution time; values never enter logs,
//! prompts, manifests, or model context.

use serde::{Deserialize, Serialize};

use nexus_capabilities::context::InvocationContext;

use crate::error::{SdkError, SdkErrorCode};

/// A reference to a broker-held credential.
///
/// The reference is the only thing that travels in manifests,
/// requests, and telemetry. The value lives in the broker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialReference {
    /// Broker-scoped reference (`vault:name` or `broker:name`).
    pub reference: String,
    /// Version of the credential.
    pub version: String,
    /// Fingerprint of the stored credential (not the value).
    pub fingerprint: String,
}

impl CredentialReference {
    /// Construct a broker reference; rejects empty or unnamespaced
    /// references.
    pub fn new(
        reference: impl Into<String>,
        version: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Result<Self, CredentialBrokerError> {
        let reference = reference.into();
        if reference.is_empty()
            || !(reference.starts_with("vault:") || reference.starts_with("broker:"))
        {
            return Err(CredentialBrokerError(SdkError::new(
                SdkErrorCode::Validation,
                "credential reference must be namespaced (vault: or broker:)",
                None,
                None,
                None,
                None,
            )));
        }
        let version = version.into();
        if version.is_empty() {
            return Err(CredentialBrokerError(SdkError::new(
                SdkErrorCode::Validation,
                "credential version must not be empty",
                None,
                None,
                None,
                None,
            )));
        }
        let fingerprint = fingerprint.into();
        if fingerprint.is_empty() {
            return Err(CredentialBrokerError(SdkError::new(
                SdkErrorCode::Validation,
                "credential fingerprint must not be empty",
                None,
                None,
                None,
                None,
            )));
        }
        Ok(Self {
            reference,
            version,
            fingerprint,
        })
    }

    /// The namespaced key of this reference.
    pub fn key(&self) -> &str {
        &self.reference
    }
}

/// Error produced by the credential broker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialBrokerError(pub SdkError);

impl std::fmt::Display for CredentialBrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "credential broker: {}", self.0)
    }
}

impl std::error::Error for CredentialBrokerError {}

/// Port for the credential broker.
///
/// Implementations resolve references to values only inside the
/// sandbox at execution time, with least privilege, and never expose
/// values through any observation surface.
pub trait CredentialBroker: Send + Sync {
    /// Resolve a credential reference to a value for one invocation.
    fn resolve(
        &self,
        reference: &CredentialReference,
        context: &InvocationContext,
    ) -> Result<String, CredentialBrokerError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep011_unit_credential_reference_valid() {
        let reference = CredentialReference::new("vault:erp-token", "3", "fp-abc").unwrap();
        assert_eq!(reference.key(), "vault:erp-token");
        let json = serde_json::to_value(&reference).unwrap();
        assert_eq!(json["reference"], "vault:erp-token");
        assert_eq!(json["fingerprint"], "fp-abc");
    }

    #[test]
    fn ep011_unit_credential_reference_rejects_unnamespaced() {
        assert!(CredentialReference::new("erp-token", "3", "fp-abc").is_err());
        assert!(CredentialReference::new("", "3", "fp-abc").is_err());
        assert!(CredentialReference::new("vault:erp-token", "", "fp-abc").is_err());
        assert!(CredentialReference::new("vault:erp-token", "3", "").is_err());
    }

    #[test]
    fn ep011_unit_credential_reference_never_holds_value() {
        let reference = CredentialReference::new("vault:erp-token", "3", "fp-abc").unwrap();
        let json = serde_json::to_value(&reference).unwrap();
        let keys: Vec<String> = json.as_object().unwrap().keys().cloned().collect();
        assert!(
            !keys
                .iter()
                .any(|k| k == "value" || k == "secret" || k == "token")
        );
    }
}
