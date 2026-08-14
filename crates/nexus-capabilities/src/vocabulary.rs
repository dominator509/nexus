//! EP-010 vocabulary classes (SPEC-003, SPEC-022).
//!
//! The domain crate already locks `CapabilityClass` (`QUERY`,
//! `COMMAND`, `WORKFLOW`, `STREAM`, `ADMINISTRATIVE`), `Idempotency`,
//! `Availability`, `Locality`, `Tier`, and `ConnectorRuntime`; EP-010
//! re-uses those tables and adds the classes below that are owned by
//! the capability/connector contract (ADR-015). Every enum parses from
//! its canonical string and rejects unknown values.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Error returned when a vocabulary string is not a known canonical class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyError(pub String);

impl fmt::Display for VocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown canonical class: {}", self.0)
    }
}

impl std::error::Error for VocabularyError {}

macro_rules! vocabulary_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            /// Canonical wire string for this class.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = VocabularyError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(VocabularyError(other.to_string())),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = VocabularyError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

vocabulary_enum! {
    /// Health state of a capability or connector (SPEC-022; schema
    /// `connector-manifest`).
    HealthState {
        Healthy = "HEALTHY",
        Degraded = "DEGRADED",
        Unavailable = "UNAVAILABLE",
        Unknown = "UNKNOWN",
    }
}

vocabulary_enum! {
    /// Provider certification state (SPEC-022 canonical term
    /// `ProviderCertification`; schema `connector-manifest`).
    Certification {
        Uncertified = "UNCERTIFIED",
        Lab = "LAB",
        Certified = "CERTIFIED",
        Deprecated = "DEPRECATED",
    }
}

/// Reference to a canonical JSON Schema (2020-12) by URI.
///
/// Schema references are stable contract identifiers: capabilities
/// advertise `input_schema` and `output_schema` by URI so that
/// generated bindings and cross-language clients resolve one
/// canonical definition (SPEC-003 behavior 1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaRef(pub String);

impl SchemaRef {
    /// Construct a schema reference. The reference must be a non-empty
    /// URI that points at a canonical schema path (either the
    /// `schemas/` relative form or the `https://schemas.nexus.local/`
    /// canonical form).
    pub fn new(value: impl Into<String>) -> Result<Self, VocabularyError> {
        let s = value.into();
        if s.is_empty() {
            return Err(VocabularyError("empty schema reference".to_string()));
        }
        if !s.starts_with("schemas/") && !s.starts_with("https://schemas.nexus.local/") {
            return Err(VocabularyError(format!(
                "schema reference must use canonical schemas/ or https://schemas.nexus.local/ form: {s}"
            )));
        }
        Ok(Self(s))
    }

    /// Raw canonical URI string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SchemaRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep010_unit_vocabulary_health_state_round_trip() {
        assert_eq!(HealthState::Healthy.as_str(), "HEALTHY");
        assert_eq!(HealthState::Degraded.as_str(), "DEGRADED");
        assert_eq!(HealthState::Unavailable.as_str(), "UNAVAILABLE");
        assert_eq!(HealthState::Unknown.as_str(), "UNKNOWN");
        assert_eq!(
            "DEGRADED".parse::<HealthState>().unwrap(),
            HealthState::Degraded
        );
    }

    #[test]
    fn ep010_unit_vocabulary_health_state_rejects_unknown() {
        let err = "HEALTHY_PLUS".parse::<HealthState>().unwrap_err();
        assert!(err.0.contains("HEALTHY_PLUS"));
        let err = "healthy".parse::<HealthState>().unwrap_err();
        assert!(err.0.contains("healthy"));
    }

    #[test]
    fn ep010_unit_vocabulary_certification_round_trip() {
        assert_eq!(Certification::Uncertified.as_str(), "UNCERTIFIED");
        assert_eq!(Certification::Lab.as_str(), "LAB");
        assert_eq!(Certification::Certified.as_str(), "CERTIFIED");
        assert_eq!(Certification::Deprecated.as_str(), "DEPRECATED");
        assert_eq!(
            "CERTIFIED".parse::<Certification>().unwrap(),
            Certification::Certified
        );
    }

    #[test]
    fn ep010_unit_vocabulary_certification_rejects_unknown() {
        let err = "CERTIFIED_NOW".parse::<Certification>().unwrap_err();
        assert!(err.0.contains("CERTIFIED_NOW"));
    }

    #[test]
    fn ep010_unit_vocabulary_schema_ref_accepts_canonical_forms() {
        let rel = SchemaRef::new("schemas/capability-descriptor.schema.json").unwrap();
        assert_eq!(rel.as_str(), "schemas/capability-descriptor.schema.json");
        let abs = SchemaRef::new("https://schemas.nexus.local/capability-descriptor/v1").unwrap();
        assert_eq!(
            abs.as_str(),
            "https://schemas.nexus.local/capability-descriptor/v1"
        );
    }

    #[test]
    fn ep010_unit_vocabulary_schema_ref_rejects_non_canonical() {
        let err = SchemaRef::new("").unwrap_err();
        assert!(err.0.contains("empty"));
        let err = SchemaRef::new("https://example.com/capability.schema.json").unwrap_err();
        assert!(err.0.contains("canonical"));
        let err = SchemaRef::new("data:application/schema+json").unwrap_err();
        assert!(err.0.contains("canonical"));
    }
}
