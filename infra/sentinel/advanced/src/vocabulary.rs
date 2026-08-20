//! EP-031 canonical advanced sentinel vocabulary (SPEC-013).
//!
//! SPEC-013 canonical terms (Suricata, Zeek, CrowdSec, EndpointSensor,
//! Honeypot, Incident) are vocabulary locked. This crate owns the
//! provider-neutral advanced detection vocabulary: optional sensor
//! profiles, alerts, incidents, triage, investigation, response, and
//! verification. Nexus-wide identifiers (TenantId, DeviceId,
//! IncidentId, ApprovalId) and ApprovalClass come from nexus-domain;
//! sentinel core ids and classes come from nexus-sentinel; they are
//! never redefined.
//!
//! Permanent invariants (SPEC-013):
//! - Advanced sensors are optional profiles (Enhanced adds Suricata;
//!   Advanced adds Zeek; Endpoint adds Wazuh or osquery; CrowdSec is
//!   optional reputation enforcement; honeypots are optional
//!   high-signal sensors isolated from real data).
//! - Alerts correlate into incidents instead of flooding users.
//! - High-confidence bounded quarantine can be preauthorized.
//! - Destructive response remains human controlled (SPEC-013 behavior
//!   6: destructive remediation, credential rotation, wipes, factory
//!   resets, and broad lockouts require human procedure).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::AdvancedSentinelError;
use nexus_sentinel::SentinelVocabularyError;

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, AdvancedSentinelError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(AdvancedSentinelError::validation(concat!(
                        stringify!($name),
                        " must be 1..=128 characters"
                    )));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        // Deserialization must run the same contract check as `new`;
        // otherwise a malformed wire value could construct an invalid
        // id through serde (fail closed, never bypass).
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(SecurityEventId);
typed_id!(IncidentCorrelationId);
typed_id!(HoneypotId);
typed_id!(TriageCaseId);
typed_id!(InvestigationCaseId);
typed_id!(ResponsePlanId);
typed_id!(VerificationRecordId);

macro_rules! vocabulary_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
            type Err = SentinelVocabularyError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(SentinelVocabularyError(other.to_string())),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = SentinelVocabularyError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

vocabulary_enum! {
    /// Optional advanced sensor profile (SPEC-013 behavior 3:
    /// Enhanced profile adds Suricata; Advanced adds Zeek; Endpoint
    /// adds Wazuh or osquery; CrowdSec is optional reputation
    /// enforcement). Honeypots are optional high-signal sensors
    /// isolated from real data (SPEC-013 behavior 7).
    AdvancedSensorProfile {
        Suricata = "SURICATA",
        Zeek = "ZEEK",
        Crowdsec = "CROWDSEC",
        Wazuh = "WAZUH",
        Osquery = "OSQUERY",
        Honeypot = "HONEYPOT",
    }
}

vocabulary_enum! {
    /// Alert lifecycle state (SPEC-013 evidence correlation).
    AlertState {
        Open = "OPEN",
        Correlated = "CORRELATED",
        Triaged = "TRIAGED",
        Investigating = "INVESTIGATING",
        Responding = "RESPONDING",
        Resolved = "RESOLVED",
        FalsePositive = "FALSE_POSITIVE",
    }
}

vocabulary_enum! {
    /// Incident lifecycle state (SPEC-013: alerts correlate into
    /// incidents instead of flooding users).
    IncidentState {
        Open = "OPEN",
        Triaged = "TRIAGED",
        Investigating = "INVESTIGATING",
        Responding = "RESPONDING",
        Resolved = "RESOLVED",
        Closed = "CLOSED",
        FalsePositive = "FALSE_POSITIVE",
    }
}

vocabulary_enum! {
    /// Correlation confidence for grouping alerts into an incident.
    /// Bounded: correlation is derived from observed shared evidence
    /// (device, indicator, window), never fabricated.
    CorrelationConfidence {
        Low = "LOW",
        Medium = "MEDIUM",
        High = "HIGH",
    }
}

vocabulary_enum! {
    /// Honeypot kind (SPEC-013 behavior 7: honeypots and honeytokens
    /// are optional high-signal sensors isolated from real data).
    HoneypotKind {
        Network = "NETWORK",
        Service = "SERVICE",
        HoneyToken = "HONEYTOKEN",
    }
}

vocabulary_enum! {
    /// Honeypot lifecycle state.
    HoneypotState {
        Armed = "ARMED",
        Triggered = "TRIGGERED",
        Disarmed = "DISARMED",
    }
}

vocabulary_enum! {
    /// Triage priority for an incident (SPEC-013: bounded, derived
    /// from observed severity and confidence; never invented).
    TriagePriority {
        Low = "LOW",
        Medium = "MEDIUM",
        High = "HIGH",
        Critical = "CRITICAL",
    }
}

vocabulary_enum! {
    /// Investigation lifecycle state.
    InvestigationState {
        Open = "OPEN",
        Gathering = "GATHERING",
        Analyzing = "ANALYZING",
        Concluded = "CONCLUDED",
    }
}

vocabulary_enum! {
    /// Response kind. Automated containment is limited to
    /// preauthorized high-confidence reversible rules (SPEC-013
    /// behavior 5). Destructive response (wipes, factory resets,
    /// broad lockouts, credential rotation) requires human procedure
    /// (SPEC-013 behavior 6) and is never auto-applicable.
    ResponseKind {
        Notify = "NOTIFY",
        Quarantine = "QUARANTINE",
        Block = "BLOCK",
        IsolateEndpoint = "ISOLATE_ENDPOINT",
        CredentialRotation = "CREDENTIAL_ROTATION",
        Wipe = "WIPE",
        FactoryReset = "FACTORY_RESET",
        BroadLockout = "BROAD_LOCKOUT",
    }
}

impl ResponseKind {
    /// Whether this response kind is destructive and therefore always
    /// requires human procedure (SPEC-013 behavior 6). Destructive
    /// response is never auto-applicable.
    pub const fn is_destructive(self) -> bool {
        matches!(
            self,
            Self::CredentialRotation | Self::Wipe | Self::FactoryReset | Self::BroadLockout
        )
    }

    /// Whether this response kind is a bounded reversible network
    /// containment that may be preauthorized under SPEC-013 behavior
    /// 5 (high-confidence, reversible).
    pub const fn is_bounded_containment(self) -> bool {
        matches!(self, Self::Quarantine | Self::Block | Self::IsolateEndpoint)
    }
}

vocabulary_enum! {
    /// Response plan lifecycle state.
    ResponsePlanState {
        Proposed = "PROPOSED",
        Approved = "APPROVED",
        Executing = "EXECUTING",
        Completed = "COMPLETED",
        Failed = "FAILED",
        Rejected = "REJECTED",
    }
}

vocabulary_enum! {
    /// Verification lifecycle state (SPEC-013: returns the network to
    /// verified safe state; exact-target evidence).
    VerificationState {
        Pending = "PENDING",
        Verified = "VERIFIED",
        Failed = "FAILED",
        Revoked = "REVOKED",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep031_unit_advanced_typed_ids_validate_and_reject() {
        let id = SecurityEventId::new("evt-1").unwrap();
        assert_eq!(id.as_str(), "evt-1");
        assert!(SecurityEventId::new("").is_err());
        assert!(SecurityEventId::new("x".repeat(129)).is_err());
        assert!(IncidentCorrelationId::new("c").is_ok());
        assert!(HoneypotId::new("h").is_ok());
        assert!(TriageCaseId::new("t").is_ok());
        assert!(InvestigationCaseId::new("i").is_ok());
        assert!(ResponsePlanId::new("r").is_ok());
        assert!(VerificationRecordId::new("v").is_ok());
    }

    #[test]
    fn ep031_unit_advanced_typed_ids_serde_cannot_bypass_validation() {
        let res: Result<SecurityEventId, _> = serde_json::from_str("\"\"");
        assert!(res.is_err());
        let id: SecurityEventId = serde_json::from_str("\"valid-evt\"").unwrap();
        assert_eq!(id.as_str(), "valid-evt");
    }

    #[test]
    fn ep031_unit_sensor_profiles_model_all_optional_classes() {
        // SPEC-013 behavior 3: Enhanced adds Suricata; Advanced adds
        // Zeek; Endpoint adds Wazuh or osquery; CrowdSec optional;
        // honeypots optional high-signal sensors (behavior 7).
        let profiles = [
            AdvancedSensorProfile::Suricata,
            AdvancedSensorProfile::Zeek,
            AdvancedSensorProfile::Crowdsec,
            AdvancedSensorProfile::Wazuh,
            AdvancedSensorProfile::Osquery,
            AdvancedSensorProfile::Honeypot,
        ];
        let mut spelled: Vec<&str> = profiles.iter().map(|p| p.as_str()).collect();
        spelled.sort_unstable();
        spelled.dedup();
        assert_eq!(spelled.len(), 6, "six distinct optional sensor profiles");
        assert_eq!(AdvancedSensorProfile::Suricata.as_str(), "SURICATA");
        assert_eq!(AdvancedSensorProfile::Honeypot.as_str(), "HONEYPOT");
    }

    #[test]
    fn ep031_unit_advanced_vocabulary_wire_spelling_locked() {
        assert_eq!(AlertState::FalsePositive.as_str(), "FALSE_POSITIVE");
        assert_eq!(IncidentState::Investigating.as_str(), "INVESTIGATING");
        assert_eq!(CorrelationConfidence::High.as_str(), "HIGH");
        assert_eq!(HoneypotKind::HoneyToken.as_str(), "HONEYTOKEN");
        assert_eq!(HoneypotState::Triggered.as_str(), "TRIGGERED");
        assert_eq!(TriagePriority::Critical.as_str(), "CRITICAL");
        assert_eq!(InvestigationState::Analyzing.as_str(), "ANALYZING");
        assert_eq!(ResponseKind::IsolateEndpoint.as_str(), "ISOLATE_ENDPOINT");
        assert_eq!(ResponsePlanState::Rejected.as_str(), "REJECTED");
        assert_eq!(VerificationState::Verified.as_str(), "VERIFIED");
        let json = serde_json::to_string(&ResponseKind::BroadLockout).unwrap();
        assert_eq!(json, "\"BROAD_LOCKOUT\"");
    }

    #[test]
    fn ep031_unit_response_kind_destructive_and_containment_classes() {
        // SPEC-013 behavior 5/6: bounded reversible containment may be
        // preauthorized; destructive response is never auto-applicable.
        assert!(ResponseKind::Quarantine.is_bounded_containment());
        assert!(ResponseKind::Block.is_bounded_containment());
        assert!(ResponseKind::IsolateEndpoint.is_bounded_containment());
        assert!(!ResponseKind::Notify.is_bounded_containment());
        assert!(!ResponseKind::Quarantine.is_destructive());
        assert!(ResponseKind::CredentialRotation.is_destructive());
        assert!(ResponseKind::Wipe.is_destructive());
        assert!(ResponseKind::FactoryReset.is_destructive());
        assert!(ResponseKind::BroadLockout.is_destructive());
        // No class is both bounded containment and destructive.
        for k in [
            ResponseKind::Notify,
            ResponseKind::Quarantine,
            ResponseKind::Block,
            ResponseKind::IsolateEndpoint,
            ResponseKind::CredentialRotation,
            ResponseKind::Wipe,
            ResponseKind::FactoryReset,
            ResponseKind::BroadLockout,
        ] {
            assert!(
                !(k.is_bounded_containment() && k.is_destructive()),
                "{} cannot be both containment and destructive",
                k
            );
        }
    }

    #[test]
    fn ep031_unit_advanced_vocabulary_rejects_unknown() {
        assert_eq!(
            "FABRICATED".parse::<AdvancedSensorProfile>(),
            Err(SentinelVocabularyError("FABRICATED".to_string()))
        );
        let res: Result<ResponseKind, _> = serde_json::from_str("\"LLM_GUESS\"");
        assert!(res.is_err());
        let res: Result<VerificationState, _> = serde_json::from_str("\"DESTROYED\"");
        assert!(res.is_err());
        let res: Result<IncidentState, _> = serde_json::from_str("\"HEAVY_SOC\"");
        assert!(res.is_err());
    }

    #[test]
    fn ep031_unit_advanced_vocabulary_roundtrips_serde() {
        let kind = HoneypotKind::Network;
        let json = serde_json::to_string(&kind).unwrap();
        let back: HoneypotKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HoneypotKind::Network);
    }
}
