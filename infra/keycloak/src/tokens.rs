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

/// Strongly typed JWT audience (RFC 7519 section 4.1.3).
///
/// The `aud` claim is standards-compatible in BOTH forms:
///
/// - a single `StringOrURI` when exactly one audience exists, e.g.
///   `"aud": "account"` (the real Keycloak 26.7.0 wire shape for the
///   default `account` client); and
/// - an array of `StringOrURI` values when multiple audiences exist, e.g.
///   `"aud": ["account", "nexus-api"]`.
///
/// Deserialization FAILS CLOSED on any malformed representation: missing,
/// null, numeric, object, empty string, empty array, nested array, or an
/// array containing non-string members are all rejected rather than
/// coerced. Duplicate audience values are normalized away by the set
/// semantics of `as_set()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audience {
    /// Exactly one audience value.
    One(String),
    /// Multiple audience values (each must be a string; never empty).
    Many(Vec<String>),
}

impl Audience {
    /// Normalize into a set for membership validation.
    ///
    /// The caller's expected audience `E` must satisfy `E` in `as_set()`.
    /// A token's audience is never coerced; `Many([])` is unrepresentable
    /// (deserialization rejects it) so a required audience can never
    /// silently collapse to "accept everything".
    pub fn as_set(&self) -> std::collections::BTreeSet<String> {
        match self {
            Audience::One(a) => std::collections::BTreeSet::from([a.clone()]),
            Audience::Many(list) => list.iter().cloned().collect(),
        }
    }

    /// True when `expected` is a member of this audience.
    pub fn contains(&self, expected: &str) -> bool {
        self.as_set().contains(expected)
    }
}

impl<'de> Deserialize<'de> for Audience {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct AudienceVisitor;

        impl<'de> serde::de::Visitor<'de> for AudienceVisitor {
            type Value = Audience;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or an array of strings")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Audience, E> {
                if v.is_empty() {
                    return Err(E::custom("audience string must not be empty"));
                }
                Ok(Audience::One(v.to_string()))
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Audience, E> {
                if v.is_empty() {
                    return Err(E::custom("audience string must not be empty"));
                }
                Ok(Audience::One(v))
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Audience, A::Error> {
                let mut values: Vec<String> = Vec::new();
                while let Some(item) = seq.next_element::<serde_json::Value>()? {
                    match item {
                        serde_json::Value::String(s) if !s.is_empty() => values.push(s),
                        serde_json::Value::String(_) => {
                            return Err(serde::de::Error::custom(
                                "audience array must not contain empty strings",
                            ));
                        }
                        other => {
                            return Err(serde::de::Error::custom(format!(
                                "audience array must contain only strings, found {}",
                                json_kind(&other)
                            )));
                        }
                    }
                }
                if values.is_empty() {
                    return Err(serde::de::Error::custom("audience array must not be empty"));
                }
                Ok(Audience::Many(values))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                _map: A,
            ) -> Result<Audience, A::Error> {
                Err(serde::de::Error::custom(
                    "audience must be a string or an array of strings, not an object",
                ))
            }
        }

        d.deserialize_any(AudienceVisitor)
    }
}

/// Serialize the audience as the RFC 7519 wire shape: a scalar string for
/// a single audience, an array of strings otherwise. The derived form
/// would emit `{"One": ...}` which is not the JWT claim shape.
impl serde::Serialize for Audience {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Audience::One(a) => serializer.serialize_str(a),
            Audience::Many(list) => {
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(list.len()))?;
                for a in list {
                    seq.serialize_element(a)?;
                }
                seq.end()
            }
        }
    }
}

/// Human-readable JSON kind for fail-closed diagnostics (never a coercion
/// path; only used to describe what was rejected).
fn json_kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Object(_) => "object",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::String(_) => "string",
    }
}

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
    /// Audience (RFC 7519: single string OR array of strings).
    pub aud: Audience,
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
        // Normalize the strongly typed audience into the canonical set; the
        // nexus-auth validator enforces expected-audience membership on it.
        let audiences: Vec<String> = self.aud.as_set().into_iter().collect();
        Ok(TokenClaims {
            token_class: TokenClass::Access,
            subject: self.sub.clone(),
            issuer: self.iss.clone(),
            audiences,
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
            aud: Audience::One("nexus-app".into()),
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

    // Directive E: audience representation regression tests

    fn parse_aud(json: &str) -> Result<Audience, serde_json::Error> {
        serde_json::from_str::<Audience>(json)
    }

    #[test]
    fn ep007_unit_audience_accepts_single_string() {
        // E1: aud = "nexus-api"
        let aud = parse_aud(r#""nexus-api""#).expect("scalar audience must parse");
        assert!(aud.contains("nexus-api"));
        assert!(!aud.contains("account"));
    }

    #[test]
    fn ep007_unit_audience_accepts_single_element_array() {
        // E2: aud = ["nexus-api"]
        let aud = parse_aud(r#"["nexus-api"]"#).expect("single-element array must parse");
        assert!(aud.contains("nexus-api"));
        assert_eq!(aud.as_set().len(), 1);
    }

    #[test]
    fn ep007_unit_audience_accepts_multi_element_array() {
        // E3: aud = ["account", "nexus-api"]
        let aud = parse_aud(r#"["account", "nexus-api"]"#).expect("multi array must parse");
        assert!(aud.contains("nexus-api"));
        assert!(aud.contains("account"));
        assert_eq!(aud.as_set().len(), 2);
    }

    #[test]
    fn ep007_unit_audience_accepts_duplicate_members_normalized() {
        // Duplicates normalize away through set semantics.
        let aud = parse_aud(r#"["nexus-api", "nexus-api"]"#).unwrap();
        assert_eq!(aud.as_set().len(), 1);
        assert!(aud.contains("nexus-api"));
    }

    #[test]
    fn ep007_unit_audience_rejects_empty_array() {
        // E4: aud = [] when audience is required -> reject
        assert!(parse_aud("[]").is_err(), "empty array must be rejected");
    }

    #[test]
    fn ep007_unit_audience_rejects_null() {
        // E5: aud = null -> reject
        assert!(parse_aud("null").is_err(), "null must be rejected");
    }

    #[test]
    fn ep007_unit_audience_rejects_number() {
        // E6: aud = 123 -> reject
        assert!(
            parse_aud("123").is_err(),
            "numeric audience must be rejected"
        );
    }

    #[test]
    fn ep007_unit_audience_rejects_mixed_array() {
        // E7: aud = ["nexus-api", 123] -> reject
        assert!(
            parse_aud(r#"["nexus-api", 123]"#).is_err(),
            "array with non-string member must be rejected"
        );
    }

    #[test]
    fn ep007_unit_audience_rejects_object() {
        // E8: aud = {} -> reject
        assert!(parse_aud("{}").is_err(), "object audience must be rejected");
    }

    #[test]
    fn ep007_unit_audience_rejects_empty_string() {
        assert!(
            parse_aud(r#""""#).is_err(),
            "empty audience string must be rejected"
        );
    }

    #[test]
    fn ep007_unit_audience_rejects_nested_array() {
        assert!(
            parse_aud(r#"[["nexus-api"]]"#).is_err(),
            "nested array must be rejected"
        );
    }

    #[test]
    fn ep007_unit_audience_rejects_boolean() {
        assert!(
            parse_aud("true").is_err(),
            "boolean audience must be rejected"
        );
    }

    // Directive E: authorization membership tests (strict semantics)

    #[test]
    fn ep007_unit_audience_authz_single_expected_pass() {
        // E9: expected nexus-api, aud = "nexus-api" -> PASS
        let aud = parse_aud(r#""nexus-api""#).unwrap();
        assert!(aud.contains("nexus-api"), "exact scalar audience must pass");
    }

    #[test]
    fn ep007_unit_audience_authz_multi_expected_pass() {
        // E10: expected nexus-api, aud = ["account", "nexus-api"] -> PASS
        let aud = parse_aud(r#"["account", "nexus-api"]"#).unwrap();
        assert!(
            aud.contains("nexus-api"),
            "member of multi audience must pass"
        );
    }

    #[test]
    fn ep007_unit_audience_authz_wrong_scalar_fail() {
        // E11: expected nexus-api, aud = "account" -> FAIL
        let aud = parse_aud(r#""account""#).unwrap();
        assert!(
            !aud.contains("nexus-api"),
            "unrelated scalar audience must fail"
        );
    }

    #[test]
    fn ep007_unit_audience_authz_wrong_single_element_fail() {
        // E12: expected nexus-api, aud = ["account"] -> FAIL
        let aud = parse_aud(r#"["account"]"#).unwrap();
        assert!(
            !aud.contains("nexus-api"),
            "unrelated single-element audience must fail"
        );
    }

    #[test]
    fn ep007_unit_audience_mapping_preserves_normalized_set() {
        // The canonical TokenClaims audiences must reflect the typed form.
        let mut kc = claims();
        kc.aud = parse_aud(r#"["account", "nexus-app", "account"]"#).unwrap();
        let mapped = kc
            .into_token_claims(
                CorrelationId::new(CORR).unwrap(),
                AuthenticationStrength::MultiFactor,
                PrincipalType::Human,
            )
            .unwrap();
        assert_eq!(mapped.audiences.len(), 2);
        assert!(mapped.audiences.contains(&"account".to_string()));
        assert!(mapped.audiences.contains(&"nexus-app".to_string()));
    }
}
