//! EP-024 appliance transport port (SPEC-011; M3).
//!
//! The transport boundary is provider-neutral: an appliance exposes
//! power/mode/status surfaces through a documented transport. The
//! adapter core is real and deterministic; the concrete transport
//! composes through the EP-020-certified Home Assistant provider
//! boundary (`nexus-home-assistant::RestTransport`) - EP-020 owns HA
//! authentication, the REST surface, service/action calls, and
//! transport semantics. This crate owns appliance semantics only; it
//! does NOT implement a second HA OAuth/REST client.
//!
//! Unbound transports fail closed and never fabricate devices, states,
//! or command acceptance (Reality rule).

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use nexus_home_assistant::{HaTransport, RestTransport};

use crate::error::{ApplianceError, ApplianceErrorCode};

/// Canonical appliance command (provider-neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApplianceCommand {
    PowerOn,
    PowerOff,
    SetMode,
}

impl ApplianceCommand {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PowerOn => "POWER_ON",
            Self::PowerOff => "POWER_OFF",
            Self::SetMode => "SET_MODE",
        }
    }

    pub fn parse(text: &str) -> Result<Self, ApplianceError> {
        match text {
            "POWER_ON" => Ok(Self::PowerOn),
            "POWER_OFF" => Ok(Self::PowerOff),
            "SET_MODE" => Ok(Self::SetMode),
            _ => Err(ApplianceError::new(
                ApplianceErrorCode::Vocabulary,
                format!("unknown appliance command {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Canonical appliance state observed after a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplianceState {
    /// Exact target device identity (canonical string form).
    pub device: String,
    /// Power state when present: ON, OFF.
    pub power: Option<String>,
    /// Mode state when present (e.g. fan percentage "37", preset).
    pub mode: Option<String>,
}

/// Command receipt: SUBMITTED at most, never VERIFIED.
///
/// COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED. The adapter
/// returns SUBMITTED after transport acceptance; verification is a
/// separate exact-target readback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplianceCommandReceipt {
    pub device: String,
    pub command: ApplianceCommand,
    pub state: ApplianceCommandState,
}

/// Appliance command state (canonical, deterministic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApplianceCommandState {
    Authorized,
    Submitted,
    Verified,
    VerificationTimeout,
    Unknown,
}

impl ApplianceCommandState {
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

/// One appliance entity as observed through the provider boundary.
///
/// `entity_id` is the STABLE provider identity (Home Assistant entity
/// ids are stable across restarts and ordering changes); it is never
/// an enumeration index. `domain` and `attributes` drive capability
/// mapping from real entity features.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplianceEntity {
    pub entity_id: String,
    pub domain: String,
    pub state: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, Value>,
}

impl ApplianceEntity {
    /// True when the entity is in a usable, powered state (canonical
    /// power observation ON).
    pub fn is_on(&self) -> bool {
        self.state == "on"
    }

    /// True when the entity is present but reports itself unavailable
    /// (Home Assistant "unavailable" state is a real provider signal,
    /// never mapped to a benign device state such as OFF).
    pub fn is_provider_unavailable(&self) -> bool {
        self.state == "unavailable"
    }

    /// True when the entity reports "unknown" (e.g. a template entity
    /// that has never been actuated). Unknown is never claimed as OFF;
    /// the entity is still present and usable.
    pub fn is_state_unknown(&self) -> bool {
        self.state == "unknown"
    }
}

/// Appliance transport port.
///
/// The default implementations fail closed: an unbound transport is
/// UNAVAILABLE and never fabricates devices, states, or command
/// acceptance (Reality rule).
pub trait ApplianceTransport {
    /// Discover appliance entities (stable provider identity).
    fn list_appliances(&self) -> Result<Vec<ApplianceEntity>, ApplianceError> {
        Err(ApplianceError::unavailable(
            "appliance transport has no implementation bound",
        ))
    }

    /// Read one appliance entity. Unknown targets must surface as
    /// NotFound (fail-closed; never Verified/benign).
    fn read_appliance(&self, entity_id: &str) -> Result<ApplianceEntity, ApplianceError> {
        let _ = entity_id;
        Err(ApplianceError::unavailable(
            "appliance transport has no implementation bound",
        ))
    }

    /// Invoke a real provider service/action for the target entity.
    fn invoke(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), ApplianceError> {
        let _ = (domain, service, entity_id, data);
        Err(ApplianceError::unavailable(
            "appliance transport has no implementation bound",
        ))
    }
}

/// Real appliance transport composing through the EP-020-certified
/// Home Assistant REST transport.
///
/// Authentication is fully owned by EP-020 (fresh OAuth token minted
/// per run by the fixture bootstrap, passed as `token`); this wrapper
/// reuses `nexus-home-assistant::RestTransport` for every HTTP call.
/// It adds the narrow appliance mapping: stable entity identity,
/// membership-based NotFound detection (the EP-020 boundary reports
/// 404 as External, so absence is proven from the real /api/states
/// registry rather than by parsing messages), and appliance service
/// invocation.
pub struct HaApplianceTransport {
    /// The EP-020-certified REST transport. Interior mutability lets
    /// the `&self` appliance port drive the stateful provider transport
    /// (the adapter holds an outer Mutex for cross-call consistency).
    inner: Mutex<RestTransport>,
}

impl HaApplianceTransport {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            inner: Mutex::new(RestTransport::new(base_url, token)),
        }
    }

    /// Prove the credential against the real instance (GET /api/).
    pub fn auth_check(&self) -> Result<bool, ApplianceError> {
        self.inner
            .lock()
            .expect("HA transport lock")
            .auth_check()
            .map_err(ApplianceError::from)
    }
}

impl ApplianceTransport for HaApplianceTransport {
    fn list_appliances(&self) -> Result<Vec<ApplianceEntity>, ApplianceError> {
        let states = self
            .inner
            .lock()
            .expect("HA transport lock")
            .get_states()
            .map_err(ApplianceError::from)?;
        Ok(states.into_iter().map(Into::into).collect())
    }

    fn read_appliance(&self, entity_id: &str) -> Result<ApplianceEntity, ApplianceError> {
        // Membership is proven from the real registry (GET /api/states)
        // so an unknown entity is NotFound by provider observation,
        // never guessed from an HTTP status message.
        let mut inner = self.inner.lock().expect("HA transport lock");
        let states = inner.get_states().map_err(ApplianceError::from)?;
        let present = states.iter().any(|s| s.entity_id == entity_id);
        if !present {
            return Err(ApplianceError::not_found(format!(
                "appliance entity {entity_id:?} is not present in the provider registry"
            )));
        }
        let state = inner.get_state(entity_id).map_err(ApplianceError::from)?;
        Ok(state.into())
    }

    fn invoke(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), ApplianceError> {
        let mut data = data.clone();
        data.insert(
            "entity_id".to_string(),
            Value::String(entity_id.to_string()),
        );
        self.inner
            .lock()
            .expect("HA transport lock")
            .call_service(domain, service, &data)
            .map_err(ApplianceError::from)
    }
}

impl From<nexus_home_assistant::HaEntityState> for ApplianceEntity {
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
    fn ep024_unit_appliance_command_vocabulary_roundtrip() {
        for command in [
            ApplianceCommand::PowerOn,
            ApplianceCommand::PowerOff,
            ApplianceCommand::SetMode,
        ] {
            let parsed = ApplianceCommand::parse(command.as_str()).expect("parse");
            assert_eq!(parsed, command);
            let ser = serde_json::to_string(&command).expect("serde");
            let de: ApplianceCommand = serde_json::from_str(&ser).expect("de");
            assert_eq!(de, command);
        }
    }

    #[test]
    fn ep024_unit_appliance_command_rejects_unknown() {
        let err = ApplianceCommand::parse("TURBO").expect_err("must reject");
        assert_eq!(err.code, ApplianceErrorCode::Vocabulary);
    }

    #[test]
    fn ep024_unit_appliance_entity_maps_domain_and_power() {
        let entity = ApplianceEntity {
            entity_id: "fan.nexus_app_fan".to_string(),
            domain: "fan".to_string(),
            state: "on".to_string(),
            attributes: BTreeMap::new(),
        };
        assert!(entity.is_on());
        assert!(!entity.is_provider_unavailable());
    }

    #[test]
    fn ep024_unit_appliance_entity_unavailable_never_benign() {
        let entity = ApplianceEntity {
            entity_id: "fan.nexus_app_fan".to_string(),
            domain: "fan".to_string(),
            state: "unavailable".to_string(),
            attributes: BTreeMap::new(),
        };
        // A provider-unavailable entity is NOT off; it is unavailable.
        assert!(!entity.is_on());
        assert!(entity.is_provider_unavailable());
    }

    #[test]
    fn ep024_unit_appliance_entity_unknown_never_claimed_off() {
        // The real template fan reports "unknown" until first
        // actuation. It is present/usable but its state is never
        // claimed as OFF (power observation is None).
        let entity = ApplianceEntity {
            entity_id: "fan.nexus_app_fan".to_string(),
            domain: "fan".to_string(),
            state: "unknown".to_string(),
            attributes: BTreeMap::new(),
        };
        assert!(entity.is_state_unknown());
        assert!(!entity.is_provider_unavailable());
        assert!(!entity.is_on());
    }

    #[test]
    fn ep024_unit_unbound_transport_fails_closed() {
        struct Unbound;
        impl ApplianceTransport for Unbound {}
        let t = Unbound;
        assert_eq!(
            t.list_appliances().expect_err("fail closed").code,
            ApplianceErrorCode::Unavailable
        );
        assert_eq!(
            t.read_appliance("fan.nexus_app_fan")
                .expect_err("fail closed")
                .code,
            ApplianceErrorCode::Unavailable
        );
        assert_eq!(
            t.invoke("fan", "turn_on", "fan.nexus_app_fan", &BTreeMap::new())
                .expect_err("fail closed")
                .code,
            ApplianceErrorCode::Unavailable
        );
    }
}
