//! EP-024 M5 vacuum transport port (SPEC-011; M5).
//!
//! The transport boundary is provider-neutral: a vacuum exposes
//! cleaning/dock capabilities through a documented transport. The
//! concrete transport composes through the EP-020-certified Home
//! Assistant provider boundary (`nexus-home-assistant::RestTransport`);
//! EP-020 owns HA authentication, the REST surface, and transport
//! semantics. This crate owns vacuum semantics only.
//!
//! Unbound transports fail closed and never fabricate vacuums, states,
//! or command acceptance (Reality rule).

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use nexus_home_assistant::{HaTransport, RestTransport};

use crate::error::{VacuumError, VacuumErrorCode};

/// Canonical vacuum command (provider-neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VacuumCommand {
    StartClean,
    Pause,
    ReturnHome,
    Dock,
    MapReadback,
}

impl VacuumCommand {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StartClean => "START_CLEAN",
            Self::Pause => "PAUSE",
            Self::ReturnHome => "RETURN_HOME",
            Self::Dock => "DOCK",
            Self::MapReadback => "MAP_READBACK",
        }
    }

    pub fn parse(text: &str) -> Result<Self, VacuumError> {
        match text {
            "START_CLEAN" => Ok(Self::StartClean),
            "PAUSE" => Ok(Self::Pause),
            "RETURN_HOME" => Ok(Self::ReturnHome),
            "DOCK" => Ok(Self::Dock),
            "MAP_READBACK" => Ok(Self::MapReadback),
            _ => Err(VacuumError::new(
                VacuumErrorCode::Vocabulary,
                format!("unknown vacuum command {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Canonical vacuum activity state observed after a command.
///
/// Mapped from the REAL Home Assistant vacuum state/activity semantics
/// (cleaning/docked/idle/paused/returning/error). Provider
/// unknown/unavailable states are NEVER mapped to a safe state - they
/// are `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VacuumActivityState {
    Cleaning,
    Docked,
    Idle,
    Paused,
    Returning,
    Error,
}

impl VacuumActivityState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cleaning => "CLEANING",
            Self::Docked => "DOCKED",
            Self::Idle => "IDLE",
            Self::Paused => "PAUSED",
            Self::Returning => "RETURNING",
            Self::Error => "ERROR",
        }
    }

    pub fn parse(text: &str) -> Result<Self, VacuumError> {
        match text {
            "CLEANING" => Ok(Self::Cleaning),
            "DOCKED" => Ok(Self::Docked),
            "IDLE" => Ok(Self::Idle),
            "PAUSED" => Ok(Self::Paused),
            "RETURNING" => Ok(Self::Returning),
            "ERROR" => Ok(Self::Error),
            _ => Err(VacuumError::new(
                VacuumErrorCode::Vocabulary,
                format!("unknown vacuum activity state {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Command receipt: SUBMITTED at most, never VERIFIED.
///
/// COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED. The adapter
/// returns SUBMITTED after transport acceptance; verification is a
/// separate exact-target readback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VacuumCommandReceipt {
    pub device: String,
    pub command: VacuumCommand,
    pub state: VacuumCommandState,
}

/// Vacuum command state (canonical, deterministic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VacuumCommandState {
    Authorized,
    Submitted,
    Verified,
    VerificationTimeout,
    Unknown,
}

impl VacuumCommandState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authorized => "AUTHORIZED",
            Self::Submitted => "SUBMITTED",
            Self::Verified => "VERIFIED",
            Self::VerificationTimeout => "VERIFICATION_TIMEOUT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// One vacuum as observed through the provider boundary.
///
/// `entity_id` is the STABLE provider identity (Home Assistant entity
/// ids are stable across restarts and ordering changes); it is never
/// an enumeration index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VacuumDevice {
    pub entity_id: String,
    pub domain: String,
    pub state: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, Value>,
}

impl VacuumDevice {
    /// True when the provider reports the vacuum unavailable (never a
    /// benign state such as DOCKED).
    pub fn is_provider_unavailable(&self) -> bool {
        self.state == "unavailable"
    }

    /// True when the provider reports the vacuum state unknown
    /// (present + usable but never claimed as a safe state).
    pub fn is_state_unknown(&self) -> bool {
        self.state == "unknown"
    }

    /// The real Home Assistant `supported_features` bitmask attribute,
    /// when the provider publishes it. Absent for providers that do
    /// not expose feature bits.
    pub fn supported_features(&self) -> Option<u64> {
        self.attributes
            .get("supported_features")
            .and_then(Value::as_u64)
    }
}

/// Vacuum transport port.
///
/// The default implementations fail closed: an unbound transport is
/// UNAVAILABLE and never fabricates vacuums, states, or command
/// acceptance (Reality rule).
pub trait VacuumTransport {
    /// Discover vacuums (stable provider identity).
    fn list_vacuums(&self) -> Result<Vec<VacuumDevice>, VacuumError> {
        Err(VacuumError::unavailable(
            "vacuum transport has no implementation bound",
        ))
    }

    /// Read one vacuum. Unknown targets surface as NotFound
    /// (fail-closed; never Verified/benign).
    fn read_vacuum(&self, entity_id: &str) -> Result<VacuumDevice, VacuumError> {
        let _ = entity_id;
        Err(VacuumError::unavailable(
            "vacuum transport has no implementation bound",
        ))
    }

    /// Invoke a real provider service/action for the target vacuum.
    fn invoke(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), VacuumError> {
        let _ = (domain, service, entity_id, data);
        Err(VacuumError::unavailable(
            "vacuum transport has no implementation bound",
        ))
    }
}

/// Real vacuum transport composing through the EP-020-certified Home
/// Assistant REST transport with a BOUNDED request timeout (10s).
/// Unknown-vacuum NotFound is proven by real /api/states registry
/// membership (the EP-020 boundary reports 404 as External), never by
/// parsing HTTP status text.
pub struct HaVacuumTransport {
    inner: Mutex<RestTransport>,
}

impl HaVacuumTransport {
    /// Compose through the EP-020-certified Home Assistant REST
    /// transport with a BOUNDED request timeout (10s). A stalled or
    /// silent provider must never hang a vacuum command: the outcome
    /// surfaces as TIMEOUT/UNKNOWN, never a fabricated result, and a
    /// closed endpoint surfaces as UNAVAILABLE (distinct).
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            inner: Mutex::new(RestTransport::with_timeout(
                base_url,
                token,
                std::time::Duration::from_secs(10),
            )),
        }
    }

    /// Prove the credential against the real instance (GET /api/).
    pub fn auth_check(&self) -> Result<bool, VacuumError> {
        self.inner
            .lock()
            .expect("HA transport lock")
            .auth_check()
            .map_err(VacuumError::from)
    }
}

impl VacuumTransport for HaVacuumTransport {
    fn list_vacuums(&self) -> Result<Vec<VacuumDevice>, VacuumError> {
        let states = self
            .inner
            .lock()
            .expect("HA transport lock")
            .get_states()
            .map_err(VacuumError::from)?;
        Ok(states.into_iter().map(Into::into).collect())
    }

    fn read_vacuum(&self, entity_id: &str) -> Result<VacuumDevice, VacuumError> {
        let mut inner = self.inner.lock().expect("HA transport lock");
        let states = inner.get_states().map_err(VacuumError::from)?;
        let present = states.iter().any(|s| s.entity_id == entity_id);
        if !present {
            return Err(VacuumError::not_found(format!(
                "vacuum {entity_id:?} is not present in the provider registry"
            )));
        }
        let state = inner.get_state(entity_id).map_err(VacuumError::from)?;
        Ok(state.into())
    }

    fn invoke(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), VacuumError> {
        let mut data = data.clone();
        data.insert(
            "entity_id".to_string(),
            Value::String(entity_id.to_string()),
        );
        self.inner
            .lock()
            .expect("HA transport lock")
            .call_service(domain, service, &data)
            .map_err(VacuumError::from)
    }
}

impl From<nexus_home_assistant::HaEntityState> for VacuumDevice {
    fn from(state: nexus_home_assistant::HaEntityState) -> Self {
        let domain = state
            .entity_id
            .split_once('.')
            .map(|(d, _)| d.to_string())
            .unwrap_or_default();
        Self {
            entity_id: state.entity_id,
            domain,
            state: state.state,
            attributes: state.attributes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep024_unit_vacuum_command_vocabulary_roundtrip() {
        for command in [
            VacuumCommand::StartClean,
            VacuumCommand::Pause,
            VacuumCommand::ReturnHome,
            VacuumCommand::Dock,
            VacuumCommand::MapReadback,
        ] {
            let parsed = VacuumCommand::parse(command.as_str()).expect("parse");
            assert_eq!(parsed, command);
            let ser = serde_json::to_string(&command).expect("serde");
            let de: VacuumCommand = serde_json::from_str(&ser).expect("de");
            assert_eq!(de, command);
        }
    }

    #[test]
    fn ep024_unit_vacuum_command_rejects_unknown() {
        let err = VacuumCommand::parse("FLOOD_THE_HOUSE").expect_err("must reject");
        assert_eq!(err.code, VacuumErrorCode::Vocabulary);
    }

    #[test]
    fn ep024_unit_vacuum_activity_state_mapping() {
        assert_eq!(
            VacuumActivityState::parse("CLEANING").expect("c"),
            VacuumActivityState::Cleaning
        );
        assert_eq!(
            VacuumActivityState::parse("DOCKED").expect("d"),
            VacuumActivityState::Docked
        );
        assert_eq!(
            VacuumActivityState::parse("PAUSED").expect("p"),
            VacuumActivityState::Paused
        );
        assert_eq!(
            VacuumActivityState::parse("RETURNING").expect("r"),
            VacuumActivityState::Returning
        );
        assert_eq!(
            VacuumActivityState::parse("ERROR").expect("e"),
            VacuumActivityState::Error
        );
        assert_eq!(
            VacuumActivityState::parse("WARP").expect_err("reject").code,
            VacuumErrorCode::Vocabulary
        );
    }

    #[test]
    fn ep024_unit_vacuum_unavailable_never_benign() {
        let device = VacuumDevice {
            entity_id: "vacuum.nexus_vacuum_a".to_string(),
            domain: "vacuum".to_string(),
            state: "unavailable".to_string(),
            attributes: BTreeMap::new(),
        };
        assert!(device.is_provider_unavailable());
        assert!(!device.is_state_unknown());
    }

    #[test]
    fn ep024_unit_vacuum_unknown_never_claimed_safe() {
        let device = VacuumDevice {
            entity_id: "vacuum.nexus_vacuum_a".to_string(),
            domain: "vacuum".to_string(),
            state: "unknown".to_string(),
            attributes: BTreeMap::new(),
        };
        assert!(device.is_state_unknown());
        assert!(!device.is_provider_unavailable());
    }

    #[test]
    fn ep024_unit_vacuum_supported_features_attribute() {
        let mut attributes = BTreeMap::new();
        attributes.insert("supported_features".to_string(), Value::from(2048u64));
        let device = VacuumDevice {
            entity_id: "vacuum.nexus_vacuum_a".to_string(),
            domain: "vacuum".to_string(),
            state: "docked".to_string(),
            attributes,
        };
        assert_eq!(device.supported_features(), Some(2048));
    }

    #[test]
    fn ep024_unit_unbound_transport_fails_closed() {
        struct Unbound;
        impl VacuumTransport for Unbound {}
        let t = Unbound;
        assert_eq!(
            t.list_vacuums().expect_err("fail closed").code,
            VacuumErrorCode::Unavailable
        );
        assert_eq!(
            t.read_vacuum("vacuum.nexus_vacuum_a")
                .expect_err("fail closed")
                .code,
            VacuumErrorCode::Unavailable
        );
        assert_eq!(
            t.invoke("vacuum", "start", "vacuum.nexus_vacuum_a", &BTreeMap::new())
                .expect_err("fail closed")
                .code,
            VacuumErrorCode::Unavailable
        );
    }
}
