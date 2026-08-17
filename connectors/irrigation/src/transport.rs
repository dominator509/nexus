//! EP-024 irrigation transport port (SPEC-011; M4).
//!
//! The transport boundary is provider-neutral: an irrigation zone
//! exposes on/off and scheduling surfaces through a documented
//! transport. The concrete transport composes through the
//! EP-020-certified Home Assistant provider boundary
//! (`nexus-home-assistant::RestTransport`) - EP-020 owns HA
//! authentication, the REST surface, and transport semantics. This
//! crate owns irrigation semantics only.
//!
//! Unbound transports fail closed and never fabricate zones, states,
//! or command acceptance (Reality rule).

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use nexus_home_assistant::{HaTransport, RestTransport};

use crate::error::{IrrigationError, IrrigationErrorCode};

/// Canonical irrigation command (provider-neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IrrigationCommand {
    ZoneOn,
    ZoneOff,
    SetSchedule,
}

impl IrrigationCommand {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ZoneOn => "ZONE_ON",
            Self::ZoneOff => "ZONE_OFF",
            Self::SetSchedule => "SET_SCHEDULE",
        }
    }

    pub fn parse(text: &str) -> Result<Self, IrrigationError> {
        match text {
            "ZONE_ON" => Ok(Self::ZoneOn),
            "ZONE_OFF" => Ok(Self::ZoneOff),
            "SET_SCHEDULE" => Ok(Self::SetSchedule),
            _ => Err(IrrigationError::new(
                IrrigationErrorCode::Vocabulary,
                format!("unknown irrigation command {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Canonical irrigation zone state observed after a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrrigationZoneState {
    /// Exact target zone identity (canonical string form).
    pub zone: String,
    /// Zone state when present: ON, OFF.
    pub state: Option<String>,
}

/// Command receipt: SUBMITTED at most, never VERIFIED.
///
/// COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED. The adapter
/// returns SUBMITTED after transport acceptance; verification is a
/// separate exact-target readback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrrigationCommandReceipt {
    pub zone: String,
    pub command: IrrigationCommand,
    pub state: IrrigationCommandState,
}

/// Irrigation command state (canonical, deterministic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IrrigationCommandState {
    Authorized,
    Submitted,
    Verified,
    VerificationTimeout,
    Unknown,
}

impl IrrigationCommandState {
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

/// One irrigation zone as observed through the provider boundary.
///
/// `entity_id` is the STABLE provider identity (Home Assistant entity
/// ids are stable across restarts and ordering changes); it is never
/// an enumeration index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrrigationZone {
    pub entity_id: String,
    pub domain: String,
    pub state: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, Value>,
}

impl IrrigationZone {
    /// True when the zone is in a usable, on state.
    pub fn is_on(&self) -> bool {
        self.state == "on"
    }

    /// True when the zone reports itself unavailable (never a benign
    /// state such as OFF).
    pub fn is_provider_unavailable(&self) -> bool {
        self.state == "unavailable"
    }

    /// True when the zone reports "unknown" (uninitialized template
    /// state); present + usable but never claimed as OFF.
    pub fn is_state_unknown(&self) -> bool {
        self.state == "unknown"
    }
}

/// Irrigation transport port.
///
/// The default implementations fail closed: an unbound transport is
/// UNAVAILABLE and never fabricates zones, states, or command
/// acceptance (Reality rule).
pub trait IrrigationTransport {
    /// Discover irrigation zones (stable provider identity).
    fn list_zones(&self) -> Result<Vec<IrrigationZone>, IrrigationError> {
        Err(IrrigationError::unavailable(
            "irrigation transport has no implementation bound",
        ))
    }

    /// Read one zone. Unknown targets surface as NotFound (fail-closed;
    /// never Verified/benign).
    fn read_zone(&self, entity_id: &str) -> Result<IrrigationZone, IrrigationError> {
        let _ = entity_id;
        Err(IrrigationError::unavailable(
            "irrigation transport has no implementation bound",
        ))
    }

    /// Invoke a real provider service/action for the target zone.
    fn invoke(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), IrrigationError> {
        let _ = (domain, service, entity_id, data);
        Err(IrrigationError::unavailable(
            "irrigation transport has no implementation bound",
        ))
    }
}

/// Real irrigation transport composing through the EP-020-certified
/// Home Assistant REST transport. Unknown-zone NotFound is proven by
/// real /api/states registry membership (the EP-020 boundary reports
/// 404 as External), never by parsing HTTP status text.
pub struct HaIrrigationTransport {
    inner: Mutex<RestTransport>,
}

impl HaIrrigationTransport {
    /// Compose through the EP-020-certified Home Assistant REST
    /// transport with a BOUNDED request timeout (10s). A stalled or
    /// silent provider must never hang an irrigation command: the
    /// outcome surfaces as TIMEOUT/UNKNOWN, never a fabricated result,
    /// and a closed endpoint surfaces as UNAVAILABLE (distinct).
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
    pub fn auth_check(&self) -> Result<bool, IrrigationError> {
        self.inner
            .lock()
            .expect("HA transport lock")
            .auth_check()
            .map_err(IrrigationError::from)
    }
}

impl IrrigationTransport for HaIrrigationTransport {
    fn list_zones(&self) -> Result<Vec<IrrigationZone>, IrrigationError> {
        let states = self
            .inner
            .lock()
            .expect("HA transport lock")
            .get_states()
            .map_err(IrrigationError::from)?;
        Ok(states.into_iter().map(Into::into).collect())
    }

    fn read_zone(&self, entity_id: &str) -> Result<IrrigationZone, IrrigationError> {
        let mut inner = self.inner.lock().expect("HA transport lock");
        let states = inner.get_states().map_err(IrrigationError::from)?;
        let present = states.iter().any(|s| s.entity_id == entity_id);
        if !present {
            return Err(IrrigationError::not_found(format!(
                "irrigation zone {entity_id:?} is not present in the provider registry"
            )));
        }
        let state = inner.get_state(entity_id).map_err(IrrigationError::from)?;
        Ok(state.into())
    }

    fn invoke(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), IrrigationError> {
        let mut data = data.clone();
        data.insert(
            "entity_id".to_string(),
            Value::String(entity_id.to_string()),
        );
        self.inner
            .lock()
            .expect("HA transport lock")
            .call_service(domain, service, &data)
            .map_err(IrrigationError::from)
    }
}

impl From<nexus_home_assistant::HaEntityState> for IrrigationZone {
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
    fn ep024_unit_irrigation_command_vocabulary_roundtrip() {
        for command in [
            IrrigationCommand::ZoneOn,
            IrrigationCommand::ZoneOff,
            IrrigationCommand::SetSchedule,
        ] {
            let parsed = IrrigationCommand::parse(command.as_str()).expect("parse");
            assert_eq!(parsed, command);
            let ser = serde_json::to_string(&command).expect("serde");
            let de: IrrigationCommand = serde_json::from_str(&ser).expect("de");
            assert_eq!(de, command);
        }
    }

    #[test]
    fn ep024_unit_irrigation_command_rejects_unknown() {
        let err = IrrigationCommand::parse("FLOOD").expect_err("must reject");
        assert_eq!(err.code, IrrigationErrorCode::Vocabulary);
    }

    #[test]
    fn ep024_unit_irrigation_zone_unavailable_never_benign() {
        let zone = IrrigationZone {
            entity_id: "input_boolean.nexus_zone_a".to_string(),
            domain: "input_boolean".to_string(),
            state: "unavailable".to_string(),
            attributes: BTreeMap::new(),
        };
        assert!(!zone.is_on());
        assert!(zone.is_provider_unavailable());
        assert!(!zone.is_state_unknown());
    }

    #[test]
    fn ep024_unit_irrigation_zone_unknown_never_claimed_off() {
        let zone = IrrigationZone {
            entity_id: "input_boolean.nexus_zone_a".to_string(),
            domain: "input_boolean".to_string(),
            state: "unknown".to_string(),
            attributes: BTreeMap::new(),
        };
        assert!(zone.is_state_unknown());
        assert!(!zone.is_provider_unavailable());
        assert!(!zone.is_on());
    }

    #[test]
    fn ep024_unit_unbound_transport_fails_closed() {
        struct Unbound;
        impl IrrigationTransport for Unbound {}
        let t = Unbound;
        assert_eq!(
            t.list_zones().expect_err("fail closed").code,
            IrrigationErrorCode::Unavailable
        );
        assert_eq!(
            t.read_zone("input_boolean.nexus_zone_a")
                .expect_err("fail closed")
                .code,
            IrrigationErrorCode::Unavailable
        );
        assert_eq!(
            t.invoke(
                "input_boolean",
                "turn_on",
                "input_boolean.nexus_zone_a",
                &BTreeMap::new()
            )
            .expect_err("fail closed")
            .code,
            IrrigationErrorCode::Unavailable
        );
    }
}
