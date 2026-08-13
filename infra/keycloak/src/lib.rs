//! Keycloak adapter (EP-007 M2).
//!
//! Implements the provider-neutral `nexus-auth` contracts on Keycloak
//! 26.7.0 (COMPONENT_REGISTRY.yaml: `keycloak`, identity.oidc, Apache-2.0,
//! replacement contract "OIDC/OAuth2 provider contract; Keycloak owns
//! identities, Nexus owns references"). The adapter is infrastructure:
//! it may import application ports but never the reverse.
//!
//! - `realm`: canonical Nexus realm import (clients, scopes, flows).
//! - `discovery`: OIDC discovery document normalization.
//! - `tokens`: Keycloak JWT claims <-> nexus-auth TokenClaims mapping and
//!   validator construction.
//! - `urls`: deterministic authorize/token endpoint URL construction.
//!
//! INV-004 style boundary: Keycloak is the identity provider, never
//! canonical truth about Nexus domain records. Provider wire payloads
//! are normalized at this boundary and never become domain contracts.

#![forbid(unsafe_code)]

pub mod discovery;
pub mod realm;
pub mod tokens;
pub mod urls;
pub mod verify;

pub use discovery::{DiscoveryError, ProviderMetadata};
pub use realm::{NexusRealm, RealmClient, RealmError, RealmFlow};
pub use tokens::{KeycloakClaims, TokenMappingError};
pub use urls::{AuthorizeUrl, ClientCredentialsBody, RefreshBody, UrlError};
pub use verify::{
    VerifiedJwt, VerifyError, verify_access_token, verify_and_validate, verify_service_token,
};
