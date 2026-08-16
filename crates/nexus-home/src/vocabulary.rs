//! EP-020 home vocabulary (SPEC-011; ADR-027).
//!
//! Vocabulary-locked enums for the home provider plane. The permanent
//! invariant is:
//!
//! COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED
//!
//! Home Assistant is the primary home control provider; Nexus commands
//! must use the real HA service/action mechanism (`/api/services/...`),
//! never `POST /api/states/<entity_id>` as a shortcut for physical
//! control. A service call being accepted means SUBMITTED, not VERIFIED.
//! Unknown/unavailable HA state is never treated as off/closed/locked/
//! safe. Unknown remains unknown.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Error returned when a home vocabulary string is unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeVocabularyError(pub String);

impl fmt::Display for HomeVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown home vocabulary class: {}", self.0)
    }
}

impl std::error::Error for HomeVocabularyError {}

macro_rules! home_vocabulary_enum {
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
            type Err = HomeVocabularyError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(HomeVocabularyError(other.to_string())),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = HomeVocabularyError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

home_vocabulary_enum! {
    /// Provider-neutral device category (SPEC-011; ADR-027).
    ///
    /// Home Assistant domain names never leak upward; the provider
    /// adapter maps HA domains into these canonical categories.
    DeviceCategory {
        Light = "LIGHT",
        Switch = "SWITCH",
        Lock = "LOCK",
        Climate = "CLIMATE",
        Cover = "COVER",
        Sensor = "SENSOR",
        BinarySensor = "BINARY_SENSOR",
        MediaPlayer = "MEDIA_PLAYER",
        Camera = "CAMERA",
        Fan = "FAN",
        Vacuum = "VACUUM",
        Alarm = "ALARM",
        Scene = "SCENE",
        Button = "BUTTON",
        Number = "NUMBER",
        Select = "SELECT",
        Other = "OTHER",
    }
}

home_vocabulary_enum! {
    /// Command execution state (SPEC-011; ADR-027).
    ///
    /// AUTHORIZED -> SUBMITTED -> VERIFICATION_PENDING -> VERIFIED, or
    /// -> VERIFICATION_TIMEOUT / UNKNOWN. A provider acknowledgement is
    /// SUBMITTED, never VERIFIED. No fabricated success.
    CommandState {
        Authorized = "AUTHORIZED",
        Submitted = "SUBMITTED",
        VerificationPending = "VERIFICATION_PENDING",
        Verified = "VERIFIED",
        VerificationTimeout = "VERIFICATION_TIMEOUT",
        Unknown = "UNKNOWN",
    }
}

home_vocabulary_enum! {
    /// Entity availability observed from the provider (SPEC-011;
    /// ADR-027). Unknown/unavailable HA state maps honestly: never
    /// treated as off/closed/locked/safe.
    EntityAvailability {
        Available = "AVAILABLE",
        Unavailable = "UNAVAILABLE",
        Unknown = "UNKNOWN",
    }
}

home_vocabulary_enum! {
    /// Fast-path decision (SPEC-011; ADR-027).
    ///
    /// Known low-risk commands execute locally without model calls
    /// after authorization. The model may translate intent; it can
    /// never call Home Assistant directly outside the Action Gateway.
    FastPathDecision {
        ExecuteLocally = "EXECUTE_LOCALLY",
        RequiresModel = "REQUIRES_MODEL",
        Denied = "DENIED",
    }
}

home_vocabulary_enum! {
    /// Verification outcome for a device command (SPEC-011; ADR-027).
    ///
    /// Only exact-target observed state satisfies verification; an
    /// unrelated state_changed event never does.
    VerificationOutcome {
        Verified = "VERIFIED",
        Timeout = "TIMEOUT",
        UnrelatedChange = "UNRELATED_CHANGE",
        Mismatch = "MISMATCH",
        Unknown = "UNKNOWN",
    }
}

home_vocabulary_enum! {
    /// Provider connection state (SPEC-011; ADR-027).
    ///
    /// A dropped WebSocket never permanently blinds Nexus; the typed
    /// state is DISCONNECTED, never a silent claim of live cache.
    ProviderConnectionState {
        Connected = "CONNECTED",
        Degraded = "DEGRADED",
        Disconnected = "DISCONNECTED",
        Reconnecting = "RECONNECTING",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep020_unit_vocabulary_parses_all_device_categories() {
        for (text, expected) in [
            ("LIGHT", DeviceCategory::Light),
            ("SWITCH", DeviceCategory::Switch),
            ("LOCK", DeviceCategory::Lock),
            ("CLIMATE", DeviceCategory::Climate),
            ("COVER", DeviceCategory::Cover),
            ("SENSOR", DeviceCategory::Sensor),
            ("BINARY_SENSOR", DeviceCategory::BinarySensor),
            ("MEDIA_PLAYER", DeviceCategory::MediaPlayer),
            ("CAMERA", DeviceCategory::Camera),
            ("FAN", DeviceCategory::Fan),
            ("VACUUM", DeviceCategory::Vacuum),
            ("ALARM", DeviceCategory::Alarm),
            ("SCENE", DeviceCategory::Scene),
            ("BUTTON", DeviceCategory::Button),
            ("NUMBER", DeviceCategory::Number),
            ("SELECT", DeviceCategory::Select),
            ("OTHER", DeviceCategory::Other),
        ] {
            assert_eq!(text.parse::<DeviceCategory>().unwrap(), expected);
            assert_eq!(expected.as_str(), text);
        }
    }

    #[test]
    fn ep020_unit_vocabulary_rejects_unknown_category() {
        assert_eq!(
            "THERMOSTAT".parse::<DeviceCategory>(),
            Err(HomeVocabularyError("THERMOSTAT".to_string()))
        );
        assert_eq!(
            "light".parse::<DeviceCategory>(),
            Err(HomeVocabularyError("light".to_string()))
        );
    }

    #[test]
    fn ep020_unit_vocabulary_command_states_are_distinct() {
        // The permanent invariant: SUBMITTED is never VERIFIED and
        // VERIFICATION_TIMEOUT is a real terminal state.
        assert_ne!(CommandState::Submitted, CommandState::Verified);
        assert_ne!(CommandState::VerificationTimeout, CommandState::Verified);
        assert_ne!(CommandState::Unknown, CommandState::Verified);
        for text in [
            "AUTHORIZED",
            "SUBMITTED",
            "VERIFICATION_PENDING",
            "VERIFIED",
            "VERIFICATION_TIMEOUT",
            "UNKNOWN",
        ] {
            assert_eq!(text.parse::<CommandState>().unwrap().as_str(), text);
        }
        assert_eq!(
            "FIXED".parse::<CommandState>(),
            Err(HomeVocabularyError("FIXED".to_string()))
        );
    }

    #[test]
    fn ep020_unit_vocabulary_availability_is_honest() {
        // Unknown/unavailable are distinct from any safe-sounding
        // state; the vocabulary has no OFF/CLOSED/LOCKED/SAFE value
        // that unknown could be coerced into.
        assert_ne!(EntityAvailability::Unknown, EntityAvailability::Available);
        assert_ne!(
            EntityAvailability::Unavailable,
            EntityAvailability::Available
        );
        assert_eq!(
            "OFF".parse::<EntityAvailability>(),
            Err(HomeVocabularyError("OFF".to_string()))
        );
    }

    #[test]
    fn ep020_unit_vocabulary_verification_has_no_auto_pass() {
        // No vocabulary value means "any change counts".
        assert_ne!(
            VerificationOutcome::UnrelatedChange,
            VerificationOutcome::Verified
        );
        assert_eq!(
            "PASS".parse::<VerificationOutcome>(),
            Err(HomeVocabularyError("PASS".to_string()))
        );
    }

    #[test]
    fn ep020_unit_vocabulary_fast_path_is_typed() {
        for text in ["EXECUTE_LOCALLY", "REQUIRES_MODEL", "DENIED"] {
            assert_eq!(text.parse::<FastPathDecision>().unwrap().as_str(), text);
        }
        assert_eq!(
            "ASK_MODEL".parse::<FastPathDecision>(),
            Err(HomeVocabularyError("ASK_MODEL".to_string()))
        );
    }
}
