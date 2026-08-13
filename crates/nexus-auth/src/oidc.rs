//! OIDC client and token validation contracts (SPEC-005; EP-007).
//!
//! `OidcClient` is the provider-neutral port for authorization-code,
//! refresh, and service-identity flows. `TokenValidator` validates the
//! five token dimensions the node contract requires: issuer, audience,
//! signature, time, scopes, and device context.
//!
//! These are CONTRACTS. Provider implementations (Keycloak in M2) live
//! behind them; no provider-specific behavior appears here.

use std::fmt;

use nexus_domain::{CorrelationId, NexusId, PrincipalType, TenantId};
use serde::{Deserialize, Serialize};

use crate::vocabulary::{AuthenticationStrength, TokenClass};

/// OIDC/OAuth2 authorization grant families (SPEC-005 behavior 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrantFlow {
    /// Authorization code with PKCE (interactive sign-in).
    AuthorizationCode,
    /// Client credentials (service identity / machine).
    ClientCredentials,
    /// Refresh token rotation.
    RefreshToken,
}

impl GrantFlow {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthorizationCode => "AUTHORIZATION_CODE",
            Self::ClientCredentials => "CLIENT_CREDENTIALS",
            Self::RefreshToken => "REFRESH_TOKEN",
        }
    }
}

impl fmt::Display for GrantFlow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned by OIDC client or token validation operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OidcError {
    /// The token's issuer does not match the configured issuer.
    IssuerMismatch,
    /// The token's audience does not include the expected audience.
    AudienceMismatch,
    /// The token signature could not be verified.
    BadSignature,
    /// The token is not yet valid (nbf) or has expired (exp).
    InvalidTime,
    /// Required scopes are missing.
    MissingScope,
    /// The token's device context does not match the expected device.
    DeviceContextMismatch,
    /// The grant flow is not permitted for this client.
    FlowNotPermitted,
    /// A required field is absent or malformed.
    Malformed(String),
}

impl fmt::Display for OidcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IssuerMismatch => f.write_str("token issuer mismatch"),
            Self::AudienceMismatch => f.write_str("token audience mismatch"),
            Self::BadSignature => f.write_str("token signature verification failed"),
            Self::InvalidTime => f.write_str("token time window invalid"),
            Self::MissingScope => f.write_str("token is missing required scopes"),
            Self::DeviceContextMismatch => f.write_str("token device context mismatch"),
            Self::FlowNotPermitted => f.write_str("grant flow not permitted for this client"),
            Self::Malformed(detail) => write!(f, "malformed token: {detail}"),
        }
    }
}

impl std::error::Error for OidcError {}

/// OIDC client registration (the port contract, provider-neutral).
///
/// Carries the issuer URL, client id, redirect URL, permitted flows, and
/// required scopes. Secrets never appear here; the client secret is a
/// secret reference (SPEC-005 behavior 6) resolved by the provider layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OidcClient {
    /// Canonical client id (provider-scoped external identity, SPEC-001
    /// requirement 7: never a Nexus primary key).
    pub client_id: String,
    /// Canonical issuer URL the client is configured against.
    pub issuer_url: String,
    /// Redirect URL for the authorization code flow.
    pub redirect_url: String,
    /// Flows this client may use.
    pub permitted_flows: Vec<GrantFlow>,
    /// Scopes required on every access token this client accepts.
    pub required_scopes: Vec<String>,
}

impl OidcClient {
    /// Construct a validated OIDC client registration.
    pub fn new(
        client_id: impl Into<String>,
        issuer_url: impl Into<String>,
        redirect_url: impl Into<String>,
        permitted_flows: Vec<GrantFlow>,
        required_scopes: Vec<String>,
    ) -> Result<Self, OidcError> {
        let client_id = client_id.into();
        let issuer_url = issuer_url.into();
        let redirect_url = redirect_url.into();
        if client_id.trim().is_empty() {
            return Err(OidcError::Malformed("empty client id".into()));
        }
        if !issuer_url.starts_with("https://") {
            return Err(OidcError::Malformed("issuer must be an https URL".into()));
        }
        if permitted_flows.is_empty() {
            return Err(OidcError::Malformed("no permitted flows".into()));
        }
        if required_scopes.iter().any(|s| s.trim().is_empty()) {
            return Err(OidcError::Malformed("empty required scope".into()));
        }
        Ok(Self {
            client_id,
            issuer_url,
            redirect_url,
            permitted_flows,
            required_scopes,
        })
    }

    /// Whether the client may use the given flow.
    pub fn permits(&self, flow: GrantFlow) -> bool {
        self.permitted_flows.contains(&flow)
    }
}

/// A validated token claims set (the output of `TokenValidator`).
///
/// Provider payloads are normalized at the infrastructure boundary and
/// never become domain contracts (SPEC-005 inputs/outputs rule). This is
/// the canonical, versioned representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedToken {
    /// Token class.
    pub token_class: TokenClass,
    /// Principal reference that the token authenticates.
    pub principal_type: PrincipalType,
    /// Tenant boundary the token is scoped to.
    pub tenant_id: TenantId,
    /// Opaque subject identifier from the issuer.
    pub subject: String,
    /// Issuer that signed the token (validated).
    pub issuer: String,
    /// Audiences the token is intended for (validated).
    pub audiences: Vec<String>,
    /// Granted scopes (validated against the required set).
    pub scopes: Vec<String>,
    /// Unix seconds when the token becomes valid.
    pub not_before_unix_s: i64,
    /// Unix seconds when the token expires.
    pub expires_at_unix_s: i64,
    /// Device binding, when the token carries a device context.
    pub device_id: Option<String>,
    /// Authentication strength asserted by the token.
    pub strength: AuthenticationStrength,
    /// Correlation of the authentication event.
    pub correlation: CorrelationId,
}

impl ValidatedToken {
    /// Whether the token is time-valid at the given instant.
    pub fn is_valid_at(&self, now_unix_s: i64) -> bool {
        self.not_before_unix_s <= now_unix_s && now_unix_s < self.expires_at_unix_s
    }

    /// Whether the token carries every required scope.
    pub fn has_all_scopes(&self, required: &[String]) -> bool {
        required.iter().all(|r| self.scopes.contains(r))
    }
}

/// Token validation outcome: either the token is accepted with claims,
/// or rejected with the specific failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenValidationOutcome {
    /// Token accepted; claims are normalized and validated.
    Accepted(ValidatedToken),
    /// Token rejected for the given reason.
    Rejected(OidcError),
}

/// Token validator configuration (provider-neutral).
///
/// The validator checks the six token dimensions the node contract
/// requires: issuer, audience, signature, time, scopes, and device
/// context. A token is accepted only when every dimension passes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenValidator {
    /// Expected issuer.
    pub expected_issuer: String,
    /// Expected audience.
    pub expected_audience: String,
    /// Scopes required on every accepted access token.
    pub required_scopes: Vec<String>,
    /// Expected device context, when the caller binds tokens to a device.
    pub expected_device_id: Option<String>,
}

impl TokenValidator {
    /// Construct a validated token validator.
    pub fn new(
        expected_issuer: impl Into<String>,
        expected_audience: impl Into<String>,
        required_scopes: Vec<String>,
        expected_device_id: Option<String>,
    ) -> Result<Self, OidcError> {
        let expected_issuer = expected_issuer.into();
        let expected_audience = expected_audience.into();
        if expected_issuer.trim().is_empty() {
            return Err(OidcError::Malformed("empty expected issuer".into()));
        }
        if expected_audience.trim().is_empty() {
            return Err(OidcError::Malformed("empty expected audience".into()));
        }
        if required_scopes.iter().any(|s| s.trim().is_empty()) {
            return Err(OidcError::Malformed("empty required scope".into()));
        }
        Ok(Self {
            expected_issuer,
            expected_audience,
            required_scopes,
            expected_device_id,
        })
    }

    /// Validate normalized claims at the given instant.
    ///
    /// Checks, in order: signature, issuer, audience, time, scopes, and
    /// device context. Returns the validated token on success or the
    /// exact rejection reason.
    pub fn validate(&self, claims: &TokenClaims, now_unix_s: i64) -> TokenValidationOutcome {
        if !claims.signature_verified {
            return TokenValidationOutcome::Rejected(OidcError::BadSignature);
        }
        if claims.issuer != self.expected_issuer {
            return TokenValidationOutcome::Rejected(OidcError::IssuerMismatch);
        }
        if !claims
            .audiences
            .iter()
            .any(|a| a == &self.expected_audience)
        {
            return TokenValidationOutcome::Rejected(OidcError::AudienceMismatch);
        }
        if now_unix_s < claims.not_before_unix_s || now_unix_s >= claims.expires_at_unix_s {
            return TokenValidationOutcome::Rejected(OidcError::InvalidTime);
        }
        if !self
            .required_scopes
            .iter()
            .all(|s| claims.scopes.contains(s))
        {
            return TokenValidationOutcome::Rejected(OidcError::MissingScope);
        }
        match (&self.expected_device_id, claims.device_id.as_deref()) {
            (Some(expected), actual) if actual != Some(expected.as_str()) => {
                return TokenValidationOutcome::Rejected(OidcError::DeviceContextMismatch);
            }
            _ => {}
        }
        TokenValidationOutcome::Accepted(ValidatedToken {
            token_class: claims.token_class,
            principal_type: claims.principal_type,
            tenant_id: claims.tenant_id.clone(),
            subject: claims.subject.clone(),
            issuer: claims.issuer.clone(),
            audiences: claims.audiences.clone(),
            scopes: claims.scopes.clone(),
            not_before_unix_s: claims.not_before_unix_s,
            expires_at_unix_s: claims.expires_at_unix_s,
            device_id: claims.device_id.clone(),
            strength: claims.strength,
            correlation: claims.correlation.clone(),
        })
    }
}

/// A normalized token payload fed to the validator.
///
/// Provider adapters decode and normalize their wire format into this
/// shape; the validator never touches raw JWT/JWS material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Token class.
    pub token_class: TokenClass,
    /// Subject identifier.
    pub subject: String,
    /// Issuer claim.
    pub issuer: String,
    /// Audience claim (may be empty for some token classes).
    pub audiences: Vec<String>,
    /// Granted scopes.
    pub scopes: Vec<String>,
    /// Not-before time, unix seconds.
    pub not_before_unix_s: i64,
    /// Expiry time, unix seconds.
    pub expires_at_unix_s: i64,
    /// Device context (optional; e.g. bound device id).
    pub device_id: Option<String>,
    /// Authentication strength asserted at issuance.
    pub strength: AuthenticationStrength,
    /// Signature verified flag set by the provider's cryptographic layer.
    pub signature_verified: bool,
    /// Correlation of the authentication event.
    pub correlation: CorrelationId,
    /// Tenant boundary resolved by the provider layer.
    pub tenant_id: TenantId,
    /// Principal type resolved by the provider layer.
    pub principal_type: PrincipalType,
}

/// A service identity registration (SPEC-005 behavior 1; machine principal).
///
/// Service identities use the client-credentials flow and are scoped to a
/// tenant with a declared purpose. The credential itself is a secret
/// reference, never a stored secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceIdentity {
    /// Nexus principal id of the service.
    pub principal_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Canonical OIDC client id.
    pub client_id: String,
    /// Declared purpose (audit-facing).
    pub purpose: String,
    /// Secret reference (SPEC-005 behavior 6), never a plaintext secret.
    pub secret_ref: String,
}

impl ServiceIdentity {
    /// Construct a validated service identity.
    pub fn new(
        principal_id: NexusId,
        tenant_id: TenantId,
        client_id: impl Into<String>,
        purpose: impl Into<String>,
        secret_ref: impl Into<String>,
    ) -> Result<Self, OidcError> {
        let client_id = client_id.into();
        let purpose = purpose.into();
        let secret_ref = secret_ref.into();
        if client_id.trim().is_empty() {
            return Err(OidcError::Malformed("empty service client id".into()));
        }
        if purpose.trim().is_empty() {
            return Err(OidcError::Malformed("empty service purpose".into()));
        }
        if secret_ref.trim().is_empty() {
            return Err(OidcError::Malformed("empty secret reference".into()));
        }
        Ok(Self {
            principal_id,
            tenant_id,
            client_id,
            purpose,
            secret_ref,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072";
    const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";
    const SID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";

    fn client() -> OidcClient {
        OidcClient::new(
            "nexus-app",
            "https://auth.nexus.local",
            "https://app.nexus.local/callback",
            vec![GrantFlow::AuthorizationCode, GrantFlow::RefreshToken],
            vec!["openid".into(), "nexus.read".into()],
        )
        .unwrap()
    }

    #[test]
    fn ep007_unit_oidc_client_constructs_and_permits_flows() {
        let c = client();
        assert!(c.permits(GrantFlow::AuthorizationCode));
        assert!(c.permits(GrantFlow::RefreshToken));
        assert!(!c.permits(GrantFlow::ClientCredentials));
    }

    #[test]
    fn ep007_unit_oidc_client_rejects_malformed() {
        assert_eq!(
            OidcClient::new(
                "",
                "https://auth.nexus.local",
                "https://app/cb",
                vec![],
                vec![]
            ),
            Err(OidcError::Malformed("empty client id".into()))
        );
        assert_eq!(
            OidcClient::new(
                "x",
                "http://insecure",
                "https://app/cb",
                vec![GrantFlow::AuthorizationCode],
                vec!["openid".into()]
            ),
            Err(OidcError::Malformed("issuer must be an https URL".into()))
        );
        assert_eq!(
            OidcClient::new(
                "x",
                "https://auth.nexus.local",
                "https://app/cb",
                vec![],
                vec!["openid".into()]
            ),
            Err(OidcError::Malformed("no permitted flows".into()))
        );
    }

    #[test]
    fn ep007_unit_oidc_client_serde_roundtrip() {
        let c = client();
        let json = serde_json::to_string(&c).unwrap();
        // Enum variants serialize in canonical SCREAMING_SNAKE_CASE.
        assert!(json.contains("\"AUTHORIZATION_CODE\""));
        let back: OidcClient = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn ep007_unit_token_claims_time_window() {
        let claims = TokenClaims {
            token_class: TokenClass::Access,
            subject: "user-1".into(),
            issuer: "https://auth.nexus.local".into(),
            audiences: vec!["nexus-app".into()],
            scopes: vec!["openid".into(), "nexus.read".into()],
            not_before_unix_s: 1000,
            expires_at_unix_s: 2000,
            device_id: None,
            strength: AuthenticationStrength::MultiFactor,
            signature_verified: true,
            correlation: CorrelationId::new(CORR).unwrap(),
            tenant_id: TenantId::new(TENANT).unwrap(),
            principal_type: PrincipalType::Human,
        };
        let token = ValidatedToken {
            token_class: claims.token_class,
            principal_type: claims.principal_type,
            tenant_id: claims.tenant_id.clone(),
            subject: claims.subject.clone(),
            issuer: claims.issuer.clone(),
            audiences: claims.audiences.clone(),
            scopes: claims.scopes.clone(),
            not_before_unix_s: claims.not_before_unix_s,
            expires_at_unix_s: claims.expires_at_unix_s,
            device_id: claims.device_id.clone(),
            strength: claims.strength,
            correlation: claims.correlation.clone(),
        };
        assert!(token.is_valid_at(1500));
        assert!(!token.is_valid_at(999));
        assert!(!token.is_valid_at(2000));
        assert!(token.has_all_scopes(&["nexus.read".to_string()]));
        assert!(!token.has_all_scopes(&["nexus.admin".to_string()]));
    }

    #[test]
    fn ep007_unit_service_identity_constructs_and_serde() {
        let s = ServiceIdentity::new(
            NexusId::new(SID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "nexus-scheduler",
            "schedules durable workflows",
            "vault://secret/nexus-scheduler",
        )
        .unwrap();
        assert_eq!(s.secret_ref, "vault://secret/nexus-scheduler");
        let json = serde_json::to_string(&s).unwrap();
        let back: ServiceIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn ep007_unit_service_identity_rejects_missing_secret_ref() {
        let res = ServiceIdentity::new(
            NexusId::new(SID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "nexus-scheduler",
            "purpose",
            "",
        );
        assert_eq!(
            res,
            Err(OidcError::Malformed("empty secret reference".into()))
        );
    }

    fn claims() -> TokenClaims {
        TokenClaims {
            token_class: TokenClass::Access,
            subject: "user-1".into(),
            issuer: "https://auth.nexus.local".into(),
            audiences: vec!["nexus-app".into()],
            scopes: vec!["openid".into(), "nexus.read".into()],
            not_before_unix_s: 1000,
            expires_at_unix_s: 2000,
            device_id: Some("device-7".into()),
            strength: AuthenticationStrength::MultiFactor,
            signature_verified: true,
            correlation: CorrelationId::new(CORR).unwrap(),
            tenant_id: TenantId::new(TENANT).unwrap(),
            principal_type: PrincipalType::Human,
        }
    }

    fn validator() -> TokenValidator {
        TokenValidator::new(
            "https://auth.nexus.local",
            "nexus-app",
            vec!["openid".into(), "nexus.read".into()],
            Some("device-7".into()),
        )
        .unwrap()
    }

    #[test]
    fn ep007_unit_token_validator_accepts_valid_token() {
        let outcome = validator().validate(&claims(), 1500);
        match outcome {
            TokenValidationOutcome::Accepted(token) => {
                assert_eq!(token.issuer, "https://auth.nexus.local");
                assert!(token.has_all_scopes(&["nexus.read".to_string()]));
                assert_eq!(token.strength, AuthenticationStrength::MultiFactor);
            }
            TokenValidationOutcome::Rejected(e) => panic!("token rejected: {e}"),
        }
    }

    #[test]
    fn ep007_unit_token_validator_rejects_bad_signature() {
        let mut c = claims();
        c.signature_verified = false;
        assert_eq!(
            validator().validate(&c, 1500),
            TokenValidationOutcome::Rejected(OidcError::BadSignature)
        );
    }

    #[test]
    fn ep007_unit_token_validator_rejects_issuer_mismatch() {
        let mut c = claims();
        c.issuer = "https://evil.example".into();
        assert_eq!(
            validator().validate(&c, 1500),
            TokenValidationOutcome::Rejected(OidcError::IssuerMismatch)
        );
    }

    #[test]
    fn ep007_unit_token_validator_rejects_audience_mismatch() {
        let mut c = claims();
        c.audiences = vec!["other-app".into()];
        assert_eq!(
            validator().validate(&c, 1500),
            TokenValidationOutcome::Rejected(OidcError::AudienceMismatch)
        );
    }

    #[test]
    fn ep007_unit_token_validator_rejects_time_window() {
        assert_eq!(
            validator().validate(&claims(), 999),
            TokenValidationOutcome::Rejected(OidcError::InvalidTime)
        );
        assert_eq!(
            validator().validate(&claims(), 2000),
            TokenValidationOutcome::Rejected(OidcError::InvalidTime)
        );
    }

    #[test]
    fn ep007_unit_token_validator_rejects_missing_scope() {
        let mut c = claims();
        c.scopes = vec!["openid".into()];
        assert_eq!(
            validator().validate(&c, 1500),
            TokenValidationOutcome::Rejected(OidcError::MissingScope)
        );
    }

    #[test]
    fn ep007_unit_token_validator_rejects_device_context_mismatch() {
        let mut c = claims();
        c.device_id = Some("device-other".into());
        assert_eq!(
            validator().validate(&c, 1500),
            TokenValidationOutcome::Rejected(OidcError::DeviceContextMismatch)
        );
    }

    #[test]
    fn ep007_unit_token_validator_ignores_device_when_unbound() {
        let v = TokenValidator::new(
            "https://auth.nexus.local",
            "nexus-app",
            vec!["openid".into()],
            None,
        )
        .unwrap();
        let mut c = claims();
        c.device_id = None;
        match v.validate(&c, 1500) {
            TokenValidationOutcome::Accepted(_) => {}
            TokenValidationOutcome::Rejected(e) => panic!("token rejected: {e}"),
        }
    }

    #[test]
    fn ep007_unit_token_validator_rejects_empty_config() {
        assert!(matches!(
            TokenValidator::new("", "nexus-app", vec!["openid".into()], None),
            Err(OidcError::Malformed(_))
        ));
        assert!(matches!(
            TokenValidator::new("https://auth.nexus.local", "", vec!["openid".into()], None),
            Err(OidcError::Malformed(_))
        ));
    }
}
