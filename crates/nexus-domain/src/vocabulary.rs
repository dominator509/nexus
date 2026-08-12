//! Canonical vocabulary tables (SPEC-001, SPEC-003, SPEC-022).
//!
//! These enums encode the vocabulary-locked classes from the accepted
//! specs. Every enum parses from its canonical string and rejects unknown
//! values, satisfying EP-002 acceptance obligation 3. Names are locked; a
//! new synonym requires an ADR and a schema update.

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
    /// Risk class of an action (SPEC-006; schema `nexus-control-object`).
    Risk {
        R0 = "R0",
        R1 = "R1",
        R2 = "R2",
        R3 = "R3",
        R4 = "R4",
    }
}

vocabulary_enum! {
    /// Privacy classification (SPEC-001; schema `nexus-control-object`).
    Privacy {
        Public = "PUBLIC",
        Household = "HOUSEHOLD",
        Personal = "PERSONAL",
        Sensitive = "SENSITIVE",
        BusinessConfidential = "BUSINESS_CONFIDENTIAL",
        Security = "SECURITY",
        Secret = "SECRET",
    }
}

vocabulary_enum! {
    /// Routing class for a request (SPEC-009; schema `nexus-control-object`).
    Route {
        Deterministic = "DETERMINISTIC",
        Reflex = "REFLEX",
        CheapApi = "CHEAP_API",
        FrontierApi = "FRONTIER_API",
        SpecialistAgent = "SPECIALIST_AGENT",
        Clarify = "CLARIFY",
        Reject = "REJECT",
    }
}

vocabulary_enum! {
    /// Principal type (SPEC-001/005; schema `invocation-context`).
    PrincipalType {
        Human = "HUMAN",
        Service = "SERVICE",
        Agent = "AGENT",
        Device = "DEVICE",
        System = "SYSTEM",
    }
}

vocabulary_enum! {
    /// Capability class (SPEC-003; schema `capability-descriptor`).
    CapabilityClass {
        Query = "QUERY",
        Command = "COMMAND",
        Workflow = "WORKFLOW",
        Stream = "STREAM",
        Administrative = "ADMINISTRATIVE",
    }
}

vocabulary_enum! {
    /// Approval class for a command (SPEC-006; schema `action-request`).
    ApprovalClass {
        None = "NONE",
        Policy = "POLICY",
        Human = "HUMAN",
        StrongHuman = "STRONG_HUMAN",
        FourEyes = "FOUR_EYES",
    }
}

vocabulary_enum! {
    /// Reversal semantics of an action (SPEC-006; schema `action-request`).
    Reversal {
        None = "NONE",
        Compensating = "COMPENSATING",
        Snapshot = "SNAPSHOT",
        Irreversible = "IRREVERSIBLE",
    }
}

vocabulary_enum! {
    /// Idempotency contract (SPEC-006; schema `capability-descriptor`).
    Idempotency {
        NotApplicable = "NOT_APPLICABLE",
        Optional = "OPTIONAL",
        Required = "REQUIRED",
    }
}

vocabulary_enum! {
    /// Availability state of a capability (SPEC-022).
    Availability {
        Available = "AVAILABLE",
        Degraded = "DEGRADED",
        Unavailable = "UNAVAILABLE",
        Uncertified = "UNCERTIFIED",
    }
}

vocabulary_enum! {
    /// Execution locality preference (SPEC-016; schema `capability-descriptor`).
    Locality {
        Any = "ANY",
        ControlPlane = "CONTROL_PLANE",
        HomeEdge = "HOME_EDGE",
        ClientDevice = "CLIENT_DEVICE",
        HardwareNode = "HARDWARE_NODE",
    }
}

vocabulary_enum! {
    /// Connector tier (SPEC-022; schema `connector-manifest`).
    Tier {
        Tier1 = "TIER1",
        Tier2 = "TIER2",
        Tier3 = "TIER3",
    }
}

vocabulary_enum! {
    /// Connector runtime type (SPEC-022; schema `connector-manifest`).
    ConnectorRuntime {
        Rust = "RUST",
        Python = "PYTHON",
        TypeScript = "TYPESCRIPT",
        Wasm = "WASM",
        Sidecar = "SIDECAR",
        Appliance = "APPLIANCE",
    }
}

vocabulary_enum! {
    /// Memory record type (SPEC-002; schema `memory-record`).
    MemoryType {
        Working = "WORKING",
        Episodic = "EPISODIC",
        Semantic = "SEMANTIC",
        Entity = "ENTITY",
        Procedural = "PROCEDURAL",
        Decision = "DECISION",
        Skill = "SKILL",
        System = "SYSTEM",
    }
}

vocabulary_enum! {
    /// Notification delivery channel (SPEC-014; schema `notification-envelope`).
    NotificationChannel {
        MobilePush = "MOBILE_PUSH",
        Desktop = "DESKTOP",
        Speaker = "SPEAKER",
        Sms = "SMS",
        Email = "EMAIL",
        Phone = "PHONE",
        Watch = "WATCH",
        Car = "CAR",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep002_unit_vocabulary_parses_all_risk_classes() {
        for (text, expected) in [
            ("R0", Risk::R0),
            ("R1", Risk::R1),
            ("R2", Risk::R2),
            ("R3", Risk::R3),
            ("R4", Risk::R4),
        ] {
            assert_eq!(text.parse::<Risk>().unwrap(), expected);
            assert_eq!(expected.as_str(), text);
        }
    }

    #[test]
    fn ep002_unit_vocabulary_rejects_unknown_risk() {
        assert_eq!("R5".parse::<Risk>(), Err(VocabularyError("R5".to_string())));
        assert_eq!("r0".parse::<Risk>(), Err(VocabularyError("r0".to_string())));
        assert_eq!("".parse::<Risk>(), Err(VocabularyError("".to_string())));
    }

    #[test]
    fn ep002_unit_vocabulary_rejects_unknown_privacy_route_principal() {
        assert_eq!(
            "TOP_SECRET".parse::<Privacy>(),
            Err(VocabularyError("TOP_SECRET".to_string()))
        );
        assert_eq!(
            "FAST_PATH".parse::<Route>(),
            Err(VocabularyError("FAST_PATH".to_string()))
        );
        assert_eq!(
            "ROBOT".parse::<PrincipalType>(),
            Err(VocabularyError("ROBOT".to_string()))
        );
    }

    #[test]
    fn ep002_unit_vocabulary_roundtrips_serde() {
        let risk = Risk::R3;
        let json = serde_json::to_string(&risk).unwrap();
        assert_eq!(json, "\"R3\"");
        let back: Risk = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Risk::R3);
    }

    #[test]
    fn ep002_unit_vocabulary_serde_rejects_unknown() {
        let res: Result<Risk, _> = serde_json::from_str("\"R9\"");
        assert!(res.is_err());
    }

    #[test]
    fn ep002_unit_vocabulary_no_vendor_brand_leaks() {
        // Acceptance obligation 4: no provider brand in canonical names.
        let all = [
            Risk::R0.as_str(),
            Privacy::Public.as_str(),
            Route::Deterministic.as_str(),
            PrincipalType::Human.as_str(),
            CapabilityClass::Query.as_str(),
            ApprovalClass::None.as_str(),
            Reversal::None.as_str(),
            Idempotency::NotApplicable.as_str(),
            Availability::Available.as_str(),
            Locality::Any.as_str(),
            Tier::Tier1.as_str(),
            ConnectorRuntime::Rust.as_str(),
            MemoryType::Working.as_str(),
            NotificationChannel::MobilePush.as_str(),
        ];
        for value in all {
            let lower = value.to_ascii_lowercase();
            for brand in [
                "alexa", "google", "apple", "samsung", "philips", "tuya", "aws", "azure", "gcp",
            ] {
                assert!(
                    !lower.contains(brand),
                    "canonical class {value} leaks provider brand {brand}"
                );
            }
        }
    }

    #[test]
    fn ep002_unit_vocabulary_covers_connector_and_memory_classes() {
        assert_eq!(Tier::Tier2.as_str(), "TIER2");
        assert_eq!(ConnectorRuntime::Sidecar.as_str(), "SIDECAR");
        assert_eq!(MemoryType::Episodic.as_str(), "EPISODIC");
        assert_eq!(NotificationChannel::Car.as_str(), "CAR");
    }
}
