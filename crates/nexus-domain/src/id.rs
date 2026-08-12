//! Canonical typed identifiers (SPEC-001).
//!
//! Every Nexus identifier is an opaque UUIDv7 value represented as a
//! lowercase canonical string. Each ID kind is a DISTINCT Rust newtype so
//! IDs are not interchangeable at compile time (SPEC-001 requirement 1;
//! EP-002 acceptance obligation 1).
//!
//! Format validation: canonical UUID form `8-4-4-4-12` in lowercase hex,
//! version nibble `7` at position 14 (0-indexed), variant `8/9/a/b` at
//! position 19. Rejects empty, malformed, uppercase, and non-UUIDv7 input.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Error returned when an ID string is not a canonical UUIDv7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    /// Wrong length or dash placement.
    Malformed,
    /// Non-hex character present.
    NonHex,
    /// Not lowercase canonical form.
    NotLowercase,
    /// Version nibble is not 7 (UUIDv7).
    WrongVersion,
    /// Variant nibble is not 8/9/a/b.
    WrongVariant,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::Malformed => "malformed UUID (expected 8-4-4-4-12 layout)",
            Self::NonHex => "non-hex character in UUID",
            Self::NotLowercase => "UUID must be lowercase canonical form",
            Self::WrongVersion => "not a UUIDv7 (version nibble != 7)",
            Self::WrongVariant => "not a UUIDv7 (variant nibble not in 8/9/a/b)",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for IdError {}

/// Validate a canonical lowercase UUIDv7 string.
fn validate_uuidv7(s: &str) -> Result<(), IdError> {
    let bytes = s.as_bytes();
    if bytes.len() != 36 {
        return Err(IdError::Malformed);
    }
    for (i, &b) in bytes.iter().enumerate() {
        match i {
            8 | 13 | 18 | 23 => {
                if b != b'-' {
                    return Err(IdError::Malformed);
                }
            }
            _ => {
                let ok = b.is_ascii_hexdigit();
                if !ok {
                    return Err(IdError::NonHex);
                }
                // Lowercase only (digits pass; A-F rejected).
                if b.is_ascii_uppercase() {
                    return Err(IdError::NotLowercase);
                }
            }
        }
    }
    // Version nibble: the first hex digit of the 4th group (index 14).
    match bytes[14] {
        b'7' => {}
        _ => return Err(IdError::WrongVersion),
    }
    // Variant nibble: first hex digit of the 5th group (index 19).
    match bytes[19] {
        b'8' | b'9' | b'a' | b'b' => {}
        _ => return Err(IdError::WrongVariant),
    }
    Ok(())
}

macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Construct from a validated canonical UUIDv7 string.
            pub fn new(s: impl Into<String>) -> Result<Self, IdError> {
                let s = s.into();
                validate_uuidv7(&s)?;
                Ok(Self(s))
            }

            /// The canonical lowercase string form.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = IdError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::new(s)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = String::deserialize(deserializer)?;
                Self::new(s).map_err(serde::de::Error::custom)
            }
        }
    };
}

typed_id!(
    /// Nexus-wide opaque identifier (SPEC-001).
    NexusId
);
typed_id!(
    /// Tenant boundary identifier (SPEC-001).
    TenantId
);
typed_id!(
    /// Person identifier (SPEC-001).
    PersonId
);
typed_id!(
    /// Household identifier (SPEC-001).
    HouseholdId
);
typed_id!(
    /// Business identifier (SPEC-001).
    BusinessId
);
typed_id!(
    /// Device identifier (SPEC-001).
    DeviceId
);
typed_id!(
    /// Objective identifier (SPEC-001).
    ObjectiveId
);
typed_id!(
    /// Task identifier (SPEC-001).
    TaskId
);
typed_id!(
    /// Capability identifier (SPEC-003/022).
    CapabilityId
);
typed_id!(
    /// Immutable artifact identifier (SPEC-003).
    ArtifactId
);
typed_id!(
    /// Event identifier (SPEC-022).
    EventId
);
typed_id!(
    /// Correlation identifier (SPEC-003).
    CorrelationId
);

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6071";

    #[test]
    fn ep002_unit_typed_id_accepts_canonical_uuidv7() {
        let id = NexusId::new(VALID).expect("valid UUIDv7 accepted");
        assert_eq!(id.as_str(), VALID);
        assert_eq!(id.to_string(), VALID);
    }

    #[test]
    fn ep002_unit_typed_ids_are_not_interchangeable() {
        // Compile-time distinctness: a TenantId cannot be passed where a
        // PersonId is required. The compiler rejects such a call (see the
        // `only_accepts_tenant` helper below); at runtime we prove each kind
        // parses its own canonical string independently.
        fn only_accepts_tenant(_id: TenantId) -> bool {
            true
        }
        let tenant = TenantId::new(VALID).unwrap();
        let person = PersonId::new(VALID).unwrap();
        assert!(only_accepts_tenant(tenant));
        assert_eq!(person.as_str(), VALID);
        // Same textual value in different kinds must not be comparable: the
        // following line would not compile:
        // assert_eq!(tenant, person);
    }

    #[test]
    fn ep002_unit_typed_id_rejects_malformed() {
        assert_eq!(NexusId::new("not-a-uuid"), Err(IdError::Malformed));
        assert_eq!(NexusId::new(""), Err(IdError::Malformed));
        assert_eq!(
            NexusId::new("0190e1c45c8a7f408a1b2c3d4e5f6071"), // no dashes
            Err(IdError::Malformed)
        );
    }

    #[test]
    fn ep002_unit_typed_id_rejects_uppercase() {
        let upper = VALID.to_ascii_uppercase();
        assert_eq!(NexusId::new(upper), Err(IdError::NotLowercase));
    }

    #[test]
    fn ep002_unit_typed_id_rejects_wrong_version() {
        // Version nibble '1' (a UUIDv1-shaped value) must be rejected.
        let v1 = "0190e1c4-5c8a-1f40-8a1b-2c3d4e5f6071";
        assert_eq!(NexusId::new(v1), Err(IdError::WrongVersion));
    }

    #[test]
    fn ep002_unit_typed_id_rejects_wrong_variant() {
        // Variant nibble 'c' is reserved/not RFC-4122 -> reject.
        let bad = "0190e1c4-5c8a-7f40-ca1b-2c3d4e5f6071";
        assert_eq!(NexusId::new(bad), Err(IdError::WrongVariant));
    }

    #[test]
    fn ep002_unit_typed_id_roundtrips_serde() {
        let id = TenantId::new(VALID).unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{VALID}\""));
        let back: TenantId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn ep002_unit_typed_id_serde_rejects_bad_input() {
        let res: Result<TenantId, _> = serde_json::from_str("\"bogus\"");
        assert!(res.is_err());
    }
}
