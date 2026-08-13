//! Canonical Nexus realm import for Keycloak (EP-007 M2).
//!
//! The realm config is the deterministic deployment contract: which
//! clients exist, which flows they may use, and which scopes they
//! require. Keycloak owns identities; this realm import defines the
//! Nexus boundaries (SPEC-005 behavior 1).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Error returned when a realm client configuration is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealmError {
    /// A required field is empty.
    Empty(String),
    /// The client id is not a canonical slug.
    InvalidClientId(String),
    /// The flow is not permitted for the client class.
    FlowNotPermitted,
    /// No redirect URLs configured for an interactive client.
    MissingRedirect,
}

impl fmt::Display for RealmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty(field) => write!(f, "realm client {field} must not be empty"),
            Self::InvalidClientId(id) => write!(f, "invalid realm client id: {id}"),
            Self::FlowNotPermitted => f.write_str("realm flow not permitted for client class"),
            Self::MissingRedirect => f.write_str("interactive client requires redirect URLs"),
        }
    }
}

impl std::error::Error for RealmError {}

/// OAuth2 flows a realm client may use (mirrors nexus-auth GrantFlow).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RealmFlow {
    /// Authorization code with PKCE (interactive sign-in).
    AuthorizationCode,
    /// Client credentials (service identity / machine).
    ClientCredentials,
    /// Refresh token rotation.
    RefreshToken,
}

impl RealmFlow {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthorizationCode => "AUTHORIZATION_CODE",
            Self::ClientCredentials => "CLIENT_CREDENTIALS",
            Self::RefreshToken => "REFRESH_TOKEN",
        }
    }
}

impl fmt::Display for RealmFlow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A realm client registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealmClient {
    /// Canonical client id (slug).
    pub client_id: String,
    /// Whether the client is confidential (holds a secret) or public.
    pub confidential: bool,
    /// Flows the client may use.
    pub flows: Vec<RealmFlow>,
    /// Redirect URLs for interactive clients.
    pub redirect_urls: Vec<String>,
    /// Default scopes for the client.
    pub default_scopes: Vec<String>,
}

impl RealmClient {
    /// Construct a validated realm client.
    pub fn new(
        client_id: impl Into<String>,
        confidential: bool,
        flows: Vec<RealmFlow>,
        redirect_urls: Vec<String>,
        default_scopes: Vec<String>,
    ) -> Result<Self, RealmError> {
        let client_id = client_id.into();
        if client_id.trim().is_empty() {
            return Err(RealmError::Empty("client_id".into()));
        }
        let slug_ok = client_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !slug_ok {
            return Err(RealmError::InvalidClientId(client_id));
        }
        if flows.is_empty() {
            return Err(RealmError::Empty("flows".into()));
        }
        let interactive = flows.contains(&RealmFlow::AuthorizationCode);
        if interactive && redirect_urls.is_empty() {
            return Err(RealmError::MissingRedirect);
        }
        if redirect_urls.iter().any(|u| !u.starts_with("https://")) {
            return Err(RealmError::MissingRedirect);
        }
        if default_scopes.iter().any(|s| s.trim().is_empty()) {
            return Err(RealmError::Empty("default_scopes".into()));
        }
        Ok(Self {
            client_id,
            confidential,
            flows,
            redirect_urls,
            default_scopes,
        })
    }

    /// Whether the client may use the given flow.
    pub fn permits(&self, flow: RealmFlow) -> bool {
        self.flows.contains(&flow)
    }
}

/// The canonical Nexus realm import (deterministic deployment contract).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NexusRealm {
    /// Realm name (canonical slug).
    pub realm_name: String,
    /// Clients registered in the realm.
    pub clients: Vec<RealmClient>,
}

impl NexusRealm {
    /// Construct a validated realm.
    pub fn new(
        realm_name: impl Into<String>,
        clients: Vec<RealmClient>,
    ) -> Result<Self, RealmError> {
        let realm_name = realm_name.into();
        if realm_name.trim().is_empty() {
            return Err(RealmError::Empty("realm_name".into()));
        }
        if clients.is_empty() {
            return Err(RealmError::Empty("clients".into()));
        }
        let mut seen = std::collections::HashSet::new();
        for client in &clients {
            if !seen.insert(client.client_id.clone()) {
                return Err(RealmError::InvalidClientId(format!(
                    "duplicate client {}",
                    client.client_id
                )));
            }
        }
        Ok(Self {
            realm_name,
            clients,
        })
    }

    /// Find a client by id.
    pub fn client(&self, client_id: &str) -> Option<&RealmClient> {
        self.clients.iter().find(|c| c.client_id == client_id)
    }
}

/// The canonical Nexus realm definition (deployment default).
pub fn nexus_realm_default() -> Result<NexusRealm, RealmError> {
    NexusRealm::new(
        "nexus",
        vec![
            RealmClient::new(
                "nexus-app",
                false,
                vec![RealmFlow::AuthorizationCode, RealmFlow::RefreshToken],
                vec!["https://app.nexus.local/callback".into()],
                vec!["openid".into(), "profile".into(), "nexus.read".into()],
            )?,
            RealmClient::new(
                "nexus-scheduler",
                true,
                vec![RealmFlow::ClientCredentials, RealmFlow::RefreshToken],
                vec![],
                vec!["openid".into(), "nexus.workflow".into()],
            )?,
            RealmClient::new(
                "nexus-connector-runtime",
                true,
                vec![RealmFlow::ClientCredentials],
                vec![],
                vec!["openid".into(), "nexus.connector".into()],
            )?,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep007_unit_realm_default_constructs() {
        let realm = nexus_realm_default().unwrap();
        assert_eq!(realm.realm_name, "nexus");
        assert_eq!(realm.clients.len(), 3);
        let app = realm.client("nexus-app").unwrap();
        assert!(app.permits(RealmFlow::AuthorizationCode));
        assert!(!app.permits(RealmFlow::ClientCredentials));
        assert!(!app.confidential);
    }

    #[test]
    fn ep007_unit_realm_client_rejects_invalid_slug() {
        assert!(matches!(
            RealmClient::new(
                "Bad Client!",
                false,
                vec![RealmFlow::AuthorizationCode],
                vec!["https://app.nexus.local/callback".into()],
                vec!["openid".into()]
            ),
            Err(RealmError::InvalidClientId(_))
        ));
        assert_eq!(
            RealmClient::new(
                "",
                false,
                vec![RealmFlow::AuthorizationCode],
                vec!["https://app.nexus.local/callback".into()],
                vec!["openid".into()]
            ),
            Err(RealmError::Empty("client_id".into()))
        );
    }

    #[test]
    fn ep007_unit_realm_client_requires_redirect_for_interactive() {
        assert_eq!(
            RealmClient::new(
                "nexus-app",
                false,
                vec![RealmFlow::AuthorizationCode],
                vec![],
                vec!["openid".into()]
            ),
            Err(RealmError::MissingRedirect)
        );
        // Client-credentials clients need no redirect.
        let ok = RealmClient::new(
            "nexus-scheduler",
            true,
            vec![RealmFlow::ClientCredentials],
            vec![],
            vec!["openid".into()],
        )
        .unwrap();
        assert!(ok.permits(RealmFlow::ClientCredentials));
    }

    #[test]
    fn ep007_unit_realm_client_rejects_non_https_redirect() {
        assert_eq!(
            RealmClient::new(
                "nexus-app",
                false,
                vec![RealmFlow::AuthorizationCode],
                vec!["http://insecure/callback".into()],
                vec!["openid".into()]
            ),
            Err(RealmError::MissingRedirect)
        );
    }

    #[test]
    fn ep007_unit_realm_rejects_duplicate_clients() {
        let dup = RealmClient::new(
            "nexus-app",
            false,
            vec![RealmFlow::AuthorizationCode],
            vec!["https://app.nexus.local/callback".into()],
            vec!["openid".into()],
        )
        .unwrap();
        let res = NexusRealm::new("nexus", vec![dup.clone(), dup]);
        assert!(matches!(res, Err(RealmError::InvalidClientId(_))));
    }

    #[test]
    fn ep007_unit_realm_serde_roundtrip() {
        let realm = nexus_realm_default().unwrap();
        let json = serde_json::to_string(&realm).unwrap();
        assert!(json.contains("\"nexus-app\""));
        let back: NexusRealm = serde_json::from_str(&json).unwrap();
        assert_eq!(back, realm);
    }
}
