//! Keycloak token mapping and validator construction (EP-007 M2).
//!
//! Maps a normalized Keycloak JWT claims payload into the
//! `nexus-auth::TokenClaims` shape and builds `nexus-auth` validator
//! configuration from realm metadata. Provider wire payloads are
//! normalized at this boundary; they never become domain contracts.

use std::fmt;

use nexus_auth::{AuthenticationStrength, TokenClaims, TokenClass, TokenValidator};
use nexus_domain::{CorrelationId, PrincipalType, TenantId};
use serde::{Deserialize, Serialize};

/// Error returned when mapping Keycloak token claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenMappingError {
    /// A required claim is missing.
    Missing(String),
    /// The tenant claim could not be parsed.
    InvalidTenant,
    /// The subject is missing.
    MissingSubject,
}

impl fmt::Display for TokenMappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(claim) => write!(f, "token missing claim {claim}"),
            Self::InvalidTenant => f.write_str("token tenant claim is not a canonical id"),
            Self::MissingSubject => f.write_str("token missing subject"),
        }
    }
}

impl std::error::Error for TokenMappingError {}

/// Normalized Keycloak access-token claims (wire shape).
///
/// Keycloak 26.7.0 default claims: `sub`, `iss`, `aud`, `scope` (space
/// separated), `iat`, `exp`, `azp`, `sid`, `realm_access.roles`, and
/// custom `tenant`/`device_id` claims added by the Nexus protocol mapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeycloakClaims {
    /// Subject.
    pub sub: String,
    /// Issuer.
    pub iss: String,
    /// Audience.
    pub aud: Vec<String>,
    /// Space-separated scope string.
    #[serde(default)]
    pub scope: String,
    /// Issued-at time, unix seconds.
    pub iat: i64,
    /// Expiry, unix seconds.
    pub exp: i64,
    /// Bound device id (custom claim, optional).
    #[serde(default)]
    pub device_id: Option<String>,
    /// Nexus tenant id (custom claim).
    pub tenant: String,
}

impl KeycloakClaims {
    /// Map into the canonical nexus-auth claims shape.
    pub fn into_token_claims(
        &self,
        correlation: CorrelationId,
        strength: AuthenticationStrength,
        principal_type: PrincipalType,
    ) -> Result<TokenClaims, TokenMappingError> {
        if self.sub.trim().is_empty() {
            return Err(TokenMappingError::MissingSubject);
        }
        let tenant_id =
            TenantId::new(&self.tenant).map_err(|_| TokenMappingError::InvalidTenant)?;
        let scopes: Vec<String> = self
            .scope
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        Ok(TokenClaims {
            token_class: TokenClass::Access,
            subject: self.sub.clone(),
            issuer: self.iss.clone(),
            audiences: self.aud.clone(),
            scopes,
            not_before_unix_s: self.iat,
            expires_at_unix_s: self.exp,
            device_id: self.device_id.clone(),
            strength,
            signature_verified: false, // set by the signature verification layer
            correlation,
            tenant_id,
            principal_type,
        })
    }
}

/// Build a nexus-auth TokenValidator from provider metadata.
///
/// `expected_issuer` must match the discovery issuer; `required_scopes`
/// are the scopes the Nexus boundary requires on accepted tokens.
pub fn validator_from_metadata(
    issuer: &str,
    audience: &str,
    required_scopes: Vec<String>,
    expected_device_id: Option<String>,
) -> Result<TokenValidator, TokenMappingError> {
    TokenValidator::new(issuer, audience, required_scopes, expected_device_id)
        .map_err(|e| TokenMappingError::Missing(format!("validator config: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072";
    const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";

    fn claims() -> KeycloakClaims {
        KeycloakClaims {
            sub: "user-1".into(),
            iss: "https://auth.nexus.local/realms/nexus".into(),
            aud: vec!["nexus-app".into()],
            scope: "openid profile nexus.read".into(),
            iat: 1000,
            exp: 2000,
            device_id: Some("device-7".into()),
            tenant: TENANT.into(),
        }
    }

    #[test]
    fn ep007_unit_keycloak_claims_map_to_nexus_shape() {
        let kc = claims();
        let mapped = kc
            .into_token_claims(
                CorrelationId::new(CORR).unwrap(),
                AuthenticationStrength::MultiFactor,
                PrincipalType::Human,
            )
            .unwrap();
        assert_eq!(mapped.subject, "user-1");
        assert_eq!(mapped.scopes, vec!["openid", "profile", "nexus.read"]);
        assert_eq!(mapped.tenant_id.as_str(), TENANT);
        assert_eq!(mapped.strength, AuthenticationStrength::MultiFactor);
        assert!(!mapped.signature_verified);
    }

    #[test]
    fn ep007_unit_keycloak_claims_reject_missing_subject() {
        let mut kc = claims();
        kc.sub = "".into();
        let res = kc.into_token_claims(
            CorrelationId::new(CORR).unwrap(),
            AuthenticationStrength::SingleFactor,
            PrincipalType::Service,
        );
        assert_eq!(res, Err(TokenMappingError::MissingSubject));
    }

    #[test]
    fn ep007_unit_keycloak_claims_reject_invalid_tenant() {
        let mut kc = claims();
        kc.tenant = "not-a-uuid".into();
        let res = kc.into_token_claims(
            CorrelationId::new(CORR).unwrap(),
            AuthenticationStrength::SingleFactor,
            PrincipalType::Human,
        );
        assert_eq!(res, Err(TokenMappingError::InvalidTenant));
    }

    #[test]
    fn ep007_unit_validator_from_metadata_builds() {
        let v = validator_from_metadata(
            "https://auth.nexus.local/realms/nexus",
            "nexus-app",
            vec!["openid".into(), "nexus.read".into()],
            Some("device-7".into()),
        )
        .unwrap();
        let kc = claims();
        let mapped = kc
            .into_token_claims(
                CorrelationId::new(CORR).unwrap(),
                AuthenticationStrength::MultiFactor,
                PrincipalType::Human,
            )
            .unwrap();
        // Signature not yet verified -> validator rejects with BadSignature.
        match v.validate(&mapped, 1500) {
            nexus_auth::TokenValidationOutcome::Rejected(e) => {
                assert_eq!(e, nexus_auth::OidcError::BadSignature)
            }
            _ => panic!("unverified token must be rejected"),
        }
    }

    #[test]
    fn ep007_unit_keycloak_claims_serde_roundtrip() {
        let kc = claims();
        let json = serde_json::to_string(&kc).unwrap();
        let back: KeycloakClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(back, kc);
    }
}
