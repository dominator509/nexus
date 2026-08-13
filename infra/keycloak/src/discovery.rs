//! OIDC discovery document normalization (EP-007 M2).
//!
//! Keycloak exposes `.well-known/openid-configuration`. This module
//! parses the discovery document into a canonical `ProviderMetadata`
//! and validates the fields the adapter depends on. Raw provider JSON
//! is normalized here and never becomes a domain contract.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Error returned when a discovery document is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// The document is missing a required field.
    Missing(String),
    /// A URL field is not https.
    InsecureUrl(String),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "discovery document missing {field}"),
            Self::InsecureUrl(field) => write!(f, "discovery URL {field} must be https"),
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Parsed provider metadata (canonical, provider-neutral shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderMetadata {
    /// Issuer identifier.
    pub issuer: String,
    /// Authorization endpoint.
    pub authorization_endpoint: String,
    /// Token endpoint.
    pub token_endpoint: String,
    /// JWKS endpoint for signature verification.
    pub jwks_uri: String,
    /// End session endpoint (optional).
    pub end_session_endpoint: Option<String>,
    /// Supported grant flows.
    pub grant_types_supported: Vec<String>,
}

impl ProviderMetadata {
    /// Whether the provider advertises the given grant type.
    pub fn supports_grant(&self, grant: &str) -> bool {
        self.grant_types_supported
            .iter()
            .any(|g| g.eq_ignore_ascii_case(grant))
    }
}

/// Parse and validate a raw OIDC discovery document.
pub fn parse_discovery(json: &str) -> Result<ProviderMetadata, DiscoveryError> {
    #[derive(Deserialize)]
    struct Raw {
        issuer: Option<String>,
        authorization_endpoint: Option<String>,
        token_endpoint: Option<String>,
        jwks_uri: Option<String>,
        #[serde(default)]
        end_session_endpoint: Option<String>,
        #[serde(default)]
        grant_types_supported: Vec<String>,
    }
    let raw: Raw =
        serde_json::from_str(json).map_err(|e| DiscoveryError::Missing(e.to_string()))?;
    let issuer = raw
        .issuer
        .ok_or_else(|| DiscoveryError::Missing("issuer".into()))?;
    let authorization_endpoint = raw
        .authorization_endpoint
        .ok_or_else(|| DiscoveryError::Missing("authorization_endpoint".into()))?;
    let token_endpoint = raw
        .token_endpoint
        .ok_or_else(|| DiscoveryError::Missing("token_endpoint".into()))?;
    let jwks_uri = raw
        .jwks_uri
        .ok_or_else(|| DiscoveryError::Missing("jwks_uri".into()))?;
    for (field, value) in [
        ("issuer", &issuer),
        ("authorization_endpoint", &authorization_endpoint),
        ("token_endpoint", &token_endpoint),
        ("jwks_uri", &jwks_uri),
    ] {
        if !value.starts_with("https://") {
            return Err(DiscoveryError::InsecureUrl(field.into()));
        }
    }
    Ok(ProviderMetadata {
        issuer,
        authorization_endpoint,
        token_endpoint,
        jwks_uri,
        end_session_endpoint: raw.end_session_endpoint,
        grant_types_supported: raw.grant_types_supported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> &'static str {
        r#"{
            "issuer": "https://auth.nexus.local/realms/nexus",
            "authorization_endpoint": "https://auth.nexus.local/realms/nexus/protocol/openid-connect/auth",
            "token_endpoint": "https://auth.nexus.local/realms/nexus/protocol/openid-connect/token",
            "jwks_uri": "https://auth.nexus.local/realms/nexus/protocol/openid-connect/certs",
            "end_session_endpoint": "https://auth.nexus.local/realms/nexus/protocol/openid-connect/logout",
            "grant_types_supported": ["authorization_code", "refresh_token", "client_credentials"]
        }"#
    }

    #[test]
    fn ep007_unit_discovery_parses_canonical_document() {
        let meta = parse_discovery(doc()).unwrap();
        assert_eq!(meta.issuer, "https://auth.nexus.local/realms/nexus");
        assert!(meta.supports_grant("authorization_code"));
        assert!(meta.supports_grant("client_credentials"));
        assert!(!meta.supports_grant("implicit"));
    }

    #[test]
    fn ep007_unit_discovery_rejects_missing_field() {
        assert!(matches!(
            parse_discovery(r#"{"issuer":"https://x"}"#),
            Err(DiscoveryError::Missing(_))
        ));
    }

    #[test]
    fn ep007_unit_discovery_rejects_insecure_url() {
        let bad = r#"{
            "issuer": "http://auth.nexus.local/realms/nexus",
            "authorization_endpoint": "https://x",
            "token_endpoint": "https://x",
            "jwks_uri": "https://x"
        }"#;
        assert_eq!(
            parse_discovery(bad),
            Err(DiscoveryError::InsecureUrl("issuer".into()))
        );
    }

    #[test]
    fn ep007_unit_discovery_rejects_garbage() {
        assert!(matches!(
            parse_discovery("not json"),
            Err(DiscoveryError::Missing(_))
        ));
    }

    #[test]
    fn ep007_unit_discovery_serde_roundtrip() {
        let meta = parse_discovery(doc()).unwrap();
        let json = serde_json::to_string(&meta).unwrap();
        let back: ProviderMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back, meta);
    }
}
