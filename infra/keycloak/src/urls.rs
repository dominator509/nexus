//! Deterministic OIDC endpoint URL construction (EP-007 M2).
//!
//! Builds authorize URLs with PKCE parameters and token endpoint grant
//! bodies. Deterministic and unit-testable; no HTTP transport here
//! (that belongs to the M3 integration boundary).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Error returned when building OIDC URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    /// A required parameter is empty.
    Empty(String),
    /// The authorization endpoint is not https.
    Insecure,
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty(param) => write!(f, "OIDC parameter {param} must not be empty"),
            Self::Insecure => f.write_str("OIDC endpoint must be https"),
        }
    }
}

impl std::error::Error for UrlError {}

/// A built authorize URL with its PKCE parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizeUrl {
    /// Full authorization endpoint URL with query.
    pub url: String,
    /// State parameter (CSRF binding).
    pub state: String,
    /// PKCE code challenge.
    pub code_challenge: String,
    /// PKCE challenge method (always S256).
    pub code_challenge_method: String,
}

/// Build an authorization-code URL with PKCE (RFC 7636).
///
/// `state` and `code_challenge` are caller-supplied random values; the
/// adapter never generates randomness (deterministic by construction).
pub fn authorize_url(
    authorization_endpoint: &str,
    client_id: &str,
    redirect_url: &str,
    scope: &str,
    state: &str,
    code_challenge: &str,
) -> Result<AuthorizeUrl, UrlError> {
    if !authorization_endpoint.starts_with("https://") {
        return Err(UrlError::Insecure);
    }
    for (param, value) in [
        ("client_id", client_id),
        ("redirect_url", redirect_url),
        ("scope", scope),
        ("state", state),
        ("code_challenge", code_challenge),
    ] {
        if value.trim().is_empty() {
            return Err(UrlError::Empty(param.into()));
        }
    }
    let url = format!(
        "{authorization_endpoint}?response_type=code&client_id={client_id}&\
         redirect_uri={redirect_url}&scope={scope}&state={state}&\
         code_challenge={code_challenge}&code_challenge_method=S256"
    );
    Ok(AuthorizeUrl {
        url,
        state: state.to_string(),
        code_challenge: code_challenge.to_string(),
        code_challenge_method: "S256".into(),
    })
}

/// A refresh-token rotation request body (deterministic shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshBody {
    /// Grant type (always refresh_token).
    pub grant_type: String,
    /// Refresh token handle.
    pub refresh_token: String,
    /// Client id.
    pub client_id: String,
}

impl RefreshBody {
    /// Construct a validated refresh body.
    pub fn new(
        refresh_token: impl Into<String>,
        client_id: impl Into<String>,
    ) -> Result<Self, UrlError> {
        let refresh_token = refresh_token.into();
        let client_id = client_id.into();
        if refresh_token.trim().is_empty() {
            return Err(UrlError::Empty("refresh_token".into()));
        }
        if client_id.trim().is_empty() {
            return Err(UrlError::Empty("client_id".into()));
        }
        Ok(Self {
            grant_type: "refresh_token".into(),
            refresh_token,
            client_id,
        })
    }
}

/// A client-credentials token request body (service identity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientCredentialsBody {
    /// Grant type (always client_credentials).
    pub grant_type: String,
    /// Client id.
    pub client_id: String,
    /// Scope string.
    pub scope: String,
}

impl ClientCredentialsBody {
    /// Construct a validated client-credentials body.
    pub fn new(client_id: impl Into<String>, scope: impl Into<String>) -> Result<Self, UrlError> {
        let client_id = client_id.into();
        let scope = scope.into();
        if client_id.trim().is_empty() {
            return Err(UrlError::Empty("client_id".into()));
        }
        if scope.trim().is_empty() {
            return Err(UrlError::Empty("scope".into()));
        }
        Ok(Self {
            grant_type: "client_credentials".into(),
            client_id,
            scope,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep007_unit_authorize_url_builds_with_pkce() {
        let built = authorize_url(
            "https://auth.nexus.local/realms/nexus/protocol/openid-connect/auth",
            "nexus-app",
            "https://app.nexus.local/callback",
            "openid profile nexus.read",
            "state-abc",
            "challenge-xyz",
        )
        .unwrap();
        assert!(built.url.contains("response_type=code"));
        assert!(built.url.contains("code_challenge_method=S256"));
        assert!(built.url.contains("state=state-abc"));
        assert_eq!(built.code_challenge_method, "S256");
    }

    #[test]
    fn ep007_unit_authorize_url_rejects_insecure_endpoint() {
        assert_eq!(
            authorize_url(
                "http://auth.nexus.local/auth",
                "nexus-app",
                "https://app.nexus.local/callback",
                "openid",
                "s",
                "c"
            ),
            Err(UrlError::Insecure)
        );
    }

    #[test]
    fn ep007_unit_authorize_url_rejects_empty_state() {
        assert_eq!(
            authorize_url(
                "https://auth.nexus.local/auth",
                "nexus-app",
                "https://app.nexus.local/callback",
                "openid",
                "",
                "c"
            ),
            Err(UrlError::Empty("state".into()))
        );
    }

    #[test]
    fn ep007_unit_refresh_body_constructs_and_serde() {
        let body = RefreshBody::new("rot-1", "nexus-app").unwrap();
        assert_eq!(body.grant_type, "refresh_token");
        let json = serde_json::to_string(&body).unwrap();
        let back: RefreshBody = serde_json::from_str(&json).unwrap();
        assert_eq!(back, body);
    }

    #[test]
    fn ep007_unit_client_credentials_body_constructs_and_serde() {
        let body = ClientCredentialsBody::new("nexus-scheduler", "openid nexus.workflow").unwrap();
        assert_eq!(body.grant_type, "client_credentials");
        let json = serde_json::to_string(&body).unwrap();
        let back: ClientCredentialsBody = serde_json::from_str(&json).unwrap();
        assert_eq!(back, body);
    }

    #[test]
    fn ep007_unit_refresh_body_rejects_empty() {
        assert_eq!(
            RefreshBody::new("", "nexus-app"),
            Err(UrlError::Empty("refresh_token".into()))
        );
    }
}
