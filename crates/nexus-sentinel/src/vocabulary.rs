//! EP-030 canonical sentinel core vocabulary (SPEC-013).
//!
//! SPEC-013 canonical terms (Sentinel, DeviceFingerprint, Baseline,
//! SecurityEvent, Incident, Quarantine, OPNsense, OpenWrt, AdGuard,
//! Suricata, Zeek, CrowdSec, EndpointSensor) are vocabulary locked.
//! This crate owns the provider-neutral sentinel vocabulary:
//! segments, trust classes, baselines, findings, containment, and
//! profiles. Nexus-wide identifiers (TenantId, BusinessId, PersonId,
//! DeviceId, IncidentId, ApprovalId) and ApprovalClass come from
//! nexus-domain and are never redefined.
//!
//! Permanent invariants (SPEC-013):
//! - IoT, trusted, guest, camera, and quarantine segments are
//!   modeled; unknown segments are rejected.
//! - Every device has a trust class.
//! - Automated containment is limited to preauthorized
//!   high-confidence reversible rules; quarantine is a proposal until
//!   approved and verified.
//! - Core Sentinel is light enough for a normal home.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{SentinelError, SentinelErrorCode};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, SentinelError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(SentinelError::new(
                        SentinelErrorCode::Validation,
                        concat!(stringify!($name), " must be 1..=128 characters"),
                        None,
                        None,
                        None,
                        None,
                    ));
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

typed_id!(NetworkDeviceId);
typed_id!(DeviceFingerprintId);
typed_id!(BaselineId);
typed_id!(NetworkFindingId);
typed_id!(QuarantineProposalId);

/// Error returned when a vocabulary string is not a known canonical
/// class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentinelVocabularyError(pub String);

impl fmt::Display for SentinelVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown canonical sentinel class: {}", self.0)
    }
}

impl std::error::Error for SentinelVocabularyError {}

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
    /// Network segment (SPEC-013: IoT, trusted, guest, camera, and
    /// quarantine segments are modeled; unknown rejected).
    NetworkSegment {
        Iot = "IOT",
        Trusted = "TRUSTED",
        Guest = "GUEST",
        Camera = "CAMERA",
        Quarantine = "QUARANTINE",
    }
}

vocabulary_enum! {
    /// Trust class of a device (SPEC-013: every device has expected
    /// protocols, destinations, internal access, baseline, owner,
    /// firmware, provider, and trust class).
    TrustClass {
        Trusted = "TRUSTED",
        Known = "KNOWN",
        Unknown = "UNKNOWN",
        Untrusted = "UNTRUSTED",
    }
}

vocabulary_enum! {
    /// Baseline lifecycle state (SPEC-013 flow baselines).
    BehaviorBaselineState {
        Learning = "LEARNING",
        Established = "ESTABLISHED",
        Stale = "STALE",
        Revoked = "REVOKED",
    }
}

vocabulary_enum! {
    /// Finding severity (SPEC-013 evidence correlation).
    FindingSeverity {
        Info = "INFO",
        Low = "LOW",
        Medium = "MEDIUM",
        High = "HIGH",
        Critical = "CRITICAL",
    }
}

vocabulary_enum! {
    /// Finding lifecycle state.
    FindingState {
        Open = "OPEN",
        Triaged = "TRIAGED",
        Suppressed = "SUPPRESSED",
        Resolved = "RESOLVED",
        FalsePositive = "FALSE_POSITIVE",
    }
}

vocabulary_enum! {
    /// Finding kind (SPEC-013: unknown-device inventory, DNS anomaly,
    /// controlled-scan quarantine, false-positive release, profile
    /// conformance, endpoint isolation, sentinel-offline behavior).
    FindingKind {
        UnknownDevice = "UNKNOWN_DEVICE",
        DnsAnomaly = "DNS_ANOMALY",
        ScanDetected = "SCAN_DETECTED",
        BaselineViolation = "BASELINE_VIOLATION",
        EndpointIsolation = "ENDPOINT_ISOLATION",
        QuarantineProposed = "QUARANTINE_PROPOSED",
    }
}

vocabulary_enum! {
    /// Firewall action on a containment rule (reversible rules only
    /// for automated containment).
    FirewallAction {
        Allow = "ALLOW",
        Deny = "DENY",
        Drop = "DROP",
    }
}

vocabulary_enum! {
    /// Quarantine proposal lifecycle (SPEC-013: automated containment
    /// is limited to preauthorized high-confidence reversible rules
    /// and always notifies the owner; quarantine is a proposal until
    /// approved and verified).
    QuarantineState {
        Proposed = "PROPOSED",
        Approved = "APPROVED",
        Applied = "APPLIED",
        Verified = "VERIFIED",
        Revoked = "REVOKED",
        Rejected = "REJECTED",
    }
}

vocabulary_enum! {
    /// Sentinel profile (SPEC-013: Core; Enhanced adds Suricata;
    /// Advanced adds Zeek; Endpoint adds Wazuh or osquery; CrowdSec is
    /// optional reputation enforcement).
    SentinelProfile {
        Core = "CORE",
        Enhanced = "ENHANCED",
        Advanced = "ADVANCED",
        Endpoint = "ENDPOINT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep030_unit_typed_ids_validate_and_reject() {
        let id = NetworkDeviceId::new("device-1").unwrap();
        assert_eq!(id.as_str(), "device-1");
        assert!(NetworkDeviceId::new("").is_err());
        assert!(NetworkDeviceId::new("x".repeat(129)).is_err());
        assert!(DeviceFingerprintId::new("f").is_ok());
        assert!(BaselineId::new("b").is_ok());
        assert!(NetworkFindingId::new("n").is_ok());
        assert!(QuarantineProposalId::new("q").is_ok());
    }

    #[test]
    fn ep030_unit_typed_ids_serde_cannot_bypass_validation() {
        let json = "\"\"";
        let res: Result<NetworkDeviceId, _> = serde_json::from_str(json);
        assert!(res.is_err());
        let json = "\"valid-id\"";
        let id: NetworkDeviceId = serde_json::from_str(json).unwrap();
        assert_eq!(id.as_str(), "valid-id");
    }

    #[test]
    fn ep030_unit_segments_model_all_five_classes() {
        // SPEC-013: IoT, trusted, guest, camera, and quarantine
        // segments are modeled.
        let segments = [
            NetworkSegment::Iot,
            NetworkSegment::Trusted,
            NetworkSegment::Guest,
            NetworkSegment::Camera,
            NetworkSegment::Quarantine,
        ];
        let mut spelled: Vec<&str> = segments.iter().map(|s| s.as_str()).collect();
        spelled.sort_unstable();
        spelled.dedup();
        assert_eq!(spelled.len(), 5, "five distinct network segments");
        assert_eq!(NetworkSegment::Iot.as_str(), "IOT");
        assert_eq!(NetworkSegment::Quarantine.as_str(), "QUARANTINE");
    }

    #[test]
    fn ep030_unit_vocabulary_wire_spelling_locked() {
        assert_eq!(TrustClass::Untrusted.as_str(), "UNTRUSTED");
        assert_eq!(BehaviorBaselineState::Established.as_str(), "ESTABLISHED");
        assert_eq!(FindingSeverity::Critical.as_str(), "CRITICAL");
        assert_eq!(FindingState::FalsePositive.as_str(), "FALSE_POSITIVE");
        assert_eq!(FindingKind::DnsAnomaly.as_str(), "DNS_ANOMALY");
        assert_eq!(FirewallAction::Drop.as_str(), "DROP");
        assert_eq!(QuarantineState::Verified.as_str(), "VERIFIED");
        assert_eq!(SentinelProfile::Enhanced.as_str(), "ENHANCED");
        let json = serde_json::to_string(&FindingKind::ScanDetected).unwrap();
        assert_eq!(json, "\"SCAN_DETECTED\"");
    }

    #[test]
    fn ep030_unit_vocabulary_rejects_unknown() {
        assert_eq!(
            "FABRICATED".parse::<NetworkSegment>(),
            Err(SentinelVocabularyError("FABRICATED".to_string()))
        );
        let res: Result<FindingKind, _> = serde_json::from_str("\"LLM_GUESS\"");
        assert!(res.is_err());
        let res: Result<QuarantineState, _> = serde_json::from_str("\"DESTROYED\"");
        assert!(res.is_err());
        let res: Result<SentinelProfile, _> = serde_json::from_str("\"HEAVY_SOC\"");
        assert!(res.is_err());
    }

    #[test]
    fn ep030_unit_vocabulary_roundtrips_serde() {
        let seg = NetworkSegment::Camera;
        let json = serde_json::to_string(&seg).unwrap();
        let back: NetworkSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(back, NetworkSegment::Camera);
    }

    #[test]
    fn ep030_unit_quarantine_state_ladder_distinct() {
        // Quarantine is a proposal until approved, applied, and
        // verified. PROPOSED != APPROVED != APPLIED != VERIFIED.
        let states = [
            QuarantineState::Proposed,
            QuarantineState::Approved,
            QuarantineState::Applied,
            QuarantineState::Verified,
            QuarantineState::Revoked,
            QuarantineState::Rejected,
        ];
        let mut spelled: Vec<&str> = states.iter().map(|s| s.as_str()).collect();
        spelled.sort_unstable();
        spelled.dedup();
        assert_eq!(spelled.len(), 6, "six distinct quarantine states");
    }
}
