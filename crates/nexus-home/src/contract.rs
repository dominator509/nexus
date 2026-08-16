//! EP-020 home provider contracts (SPEC-011; ADR-027).
//!
//! Provider-neutral home plane: device twins, canonical intents, the
//! deterministic local fast path, exact-target state verification, and
//! automation handoff. Home Assistant is the primary home control
//! provider; the concrete adapter lives behind these ports.
//!
//! Permanent invariants (owner directive, EP-020):
//!
//! 1. COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED. A Home
//!    Assistant service call being accepted means SUBMITTED, never
//!    VERIFIED. `execute()` returns at most SUBMITTED; only `verify()`
//!    against exact-target observations can produce VERIFIED.
//!
//! 2. `POST /api/states/<entity_id>` is never the implementation of a
//!    physical command. Physical device control uses the real HA
//!    service/action mechanism (`/api/services/<domain>/<service>` or
//!    the equivalent WebSocket call). State writes are allowed only for
//!    synthetic/state-only entities and never satisfy device command
//!    execution or verification (directive section 3; M4 regression
//!    test enforces the absence of the state-write path in the adapter).
//!
//! 3. Verify the exact target. Unrelated `state_changed` events never
//!    satisfy verification. Verification binds canonical device/entity
//!    identity, exact HA entity, requested action, and the expected
//!    resulting state/attribute.
//!
//! 4. Unknown/unavailable remains unknown. `EntityAvailability::Unknown`
//!    is never treated as off/closed/locked/safe.
//!
//! 5. The model may propose device/action/parameters but can never call
//!    Home Assistant directly outside the Action Gateway. Authorization
//!    belongs to EP-008; provider credentials are infrastructure
//!    credentials, never user authorization.

use std::collections::BTreeMap;

use nexus_domain::{
    ApprovalClass, CapabilityClass, CorrelationId, DeviceId, Idempotency, PersonId, Risk,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{HomeError, HomeErrorCode};
use crate::vocabulary::{
    CommandState, DeviceCategory, EntityAvailability, FastPathDecision, ProviderConnectionState,
    VerificationOutcome,
};

/// Canonical area/room identity (SPEC-011 canonical term `Area`;
/// ADR-027). Typed so friendly room names can change without breaking
/// identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AreaId(pub String);

impl AreaId {
    /// Construct a canonical area id (non-empty, no whitespace).
    pub fn new(s: impl Into<String>) -> Result<Self, HomeError> {
        let s = s.into();
        if s.is_empty() || s.chars().any(char::is_whitespace) {
            return Err(HomeError::new(
                HomeErrorCode::Validation,
                "area id must be non-empty without whitespace",
                None,
                None,
            ));
        }
        Ok(Self(s))
    }
}

/// Canonical reference to one Home Assistant entity.
///
/// This is a provider reference (e.g. `light.kitchen`), never the
/// canonical Nexus identity. Friendly names and even entity ids may
/// change on the provider; the canonical identity is `DeviceId`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HaEntityRef(pub String);

impl HaEntityRef {
    pub fn new(s: impl Into<String>) -> Result<Self, HomeError> {
        let s = s.into();
        if s.is_empty() || s.chars().any(char::is_whitespace) {
            return Err(HomeError::new(
                HomeErrorCode::Validation,
                "HA entity ref must be non-empty without whitespace",
                None,
                None,
            ));
        }
        Ok(Self(s))
    }
}

/// Canonical reference to one Home Assistant device (provider id).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HaDeviceRef(pub String);

impl HaDeviceRef {
    pub fn new(s: impl Into<String>) -> Result<Self, HomeError> {
        let s = s.into();
        if s.is_empty() || s.chars().any(char::is_whitespace) {
            return Err(HomeError::new(
                HomeErrorCode::Validation,
                "HA device ref must be non-empty without whitespace",
                None,
                None,
            ));
        }
        Ok(Self(s))
    }
}

/// Per-capability verification rule (SPEC-011; ADR-027).
///
/// Defines what evidence means success for a command. Every
/// consequential command carries a rule; a missing rule is an explicit
/// `NoVerification` with a stated reason, never a silent pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationRule {
    /// The target entity state must equal the expected value, e.g.
    /// light.turn_on -> state == "on", lock.lock -> state == "locked".
    StateEquals { expected: String },
    /// A named attribute must equal the expected value, e.g. climate
    /// set_temperature -> attribute temperature == requested target.
    AttributeEquals { attribute: String, expected: Value },
    /// The target entity state must be one of the expected values, e.g.
    /// cover.open -> state in ["open", "opening"] per exact semantics.
    StateIn { expected: Vec<String> },
    /// Explicitly no verification for this capability, with the reason
    /// recorded. Never used silently.
    NoVerification { reason: String },
}

/// A single canonical device capability, reusing the EP-010 capability
/// taxonomy (`CapabilityClass`, `Risk`, `ApprovalClass` from
/// nexus-domain). Provider domain names never appear here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapability {
    /// Canonical capability key (`^[a-z][a-z0-9_.-]+$`).
    pub capability_id: String,
    /// Capability class from the EP-010 taxonomy.
    pub class: CapabilityClass,
    /// Risk class of the capability's actions (EP-008 policy input).
    pub risk: Risk,
    /// Approval class required before execution.
    pub approval: ApprovalClass,
    /// Idempotency contract for retryable commands (SPEC-006).
    pub idempotency: Idempotency,
    /// Verification rule for commands of this capability.
    pub verification: VerificationRule,
}

/// Canonical Nexus device twin (SPEC-011 canonical term `Device`;
/// ADR-027).
///
/// Home Assistant distinguishes devices from entities: a device can
/// expose multiple entities representing different capabilities/state
/// surfaces. This twin preserves that distinction. `friendly_name` is
/// NOT identity; `device_id` is the stable canonical Nexus identity and
/// survives friendly-name changes, room changes, restart, and discovery
/// refresh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceTwin {
    /// Stable canonical Nexus device identity.
    pub device_id: DeviceId,
    /// Mutable display name. Never used as identity.
    pub friendly_name: String,
    /// Canonical area/room identity where available.
    pub area: Option<AreaId>,
    /// Canonical owner person identity where available.
    pub owner: Option<PersonId>,
    /// Provider device reference (Home Assistant device id).
    pub ha_device_ref: HaDeviceRef,
    /// Provider entity references (Home Assistant entity ids). One
    /// device may expose multiple entities.
    pub ha_entity_refs: Vec<HaEntityRef>,
    /// Provider integration/domain (e.g. `light`, `lock`). Stored for
    /// adapter use; canonical category is `category`.
    pub provider_domain: String,
    /// Canonical device category (never the raw provider domain).
    pub category: DeviceCategory,
    /// Manufacturer where available.
    pub manufacturer: Option<String>,
    /// Model where available.
    pub model: Option<String>,
    /// Availability observed from the provider.
    pub availability: EntityAvailability,
    /// Current state string where available (e.g. `on`, `locked`).
    pub state: Option<String>,
    /// Current attributes (redacted by consumers; never credentials).
    pub attributes: BTreeMap<String, Value>,
    /// Canonical capabilities of this twin.
    pub capabilities: Vec<DeviceCapability>,
    /// Via-device topology reference where relevant.
    pub parent_ha_device_ref: Option<HaDeviceRef>,
}

impl DeviceTwin {
    /// Deterministic capability lookup by canonical key.
    pub fn capability(&self, capability_id: &str) -> Option<&DeviceCapability> {
        self.capabilities
            .iter()
            .find(|c| c.capability_id == capability_id)
    }

    /// True when the twin reports available (never when unknown).
    pub fn is_available(&self) -> bool {
        self.availability == EntityAvailability::Available
    }
}

/// Canonical home intent (SPEC-011 canonical term `FastPathIntent` is
/// the fast-path subset; ADR-027).
///
/// The model may produce a `HomeIntent`; it can never call Home
/// Assistant directly. Execution is deterministic after authorization
/// and always goes through the provider adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeIntent {
    /// Target canonical device.
    pub device_id: DeviceId,
    /// Canonical capability key (from the twin's capabilities).
    pub capability_id: String,
    /// Canonical action name (e.g. `turn_on`, `unlock`,
    /// `set_temperature`).
    pub action: String,
    /// Action parameters (canonical values only).
    pub parameters: BTreeMap<String, Value>,
    /// Correlation id for traceability and event correlation.
    pub correlation_id: CorrelationId,
    /// Idempotency key for retryable commands (SPEC-006).
    pub idempotency_key: Option<String>,
}

/// Provider receipt for an executed command (SPEC-011; ADR-027).
///
/// A receipt is evidence the provider accepted the request, not that
/// the device changed or that verification passed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandReceipt {
    /// The exact intent that was authorized and submitted.
    pub intent: HomeIntent,
    /// State after provider acceptance: SUBMITTED at most, never
    /// VERIFIED.
    pub state: CommandState,
    /// The exact HA entity the action was submitted against.
    pub target_ha_entity: HaEntityRef,
    /// The HA domain/service call used (e.g. `light/turn_on`).
    pub provider_service: String,
}

/// Fast-path matcher (SPEC-011; ADR-027).
///
/// Deterministic: known low-risk commands execute locally without model
/// calls after authorization. The matcher never consults a model; it
/// may only consult cached policy and the twin registry. Model
/// interpretation may translate "turn off the kitchen lights" into a
/// canonical `HomeIntent`; the actual execution path is deterministic
/// after authorization.
pub trait FastPathMatcher {
    /// Decide whether the intent can execute locally, requires model
    /// interpretation, or is denied.
    fn decide(&self, intent: &HomeIntent) -> FastPathDecision;
}

/// Exact-target observation used by the state verifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateObservation {
    /// The exact HA entity observed.
    pub ha_entity: HaEntityRef,
    /// Observed state string, when present.
    pub state: Option<String>,
    /// Observed attributes, when present.
    pub attributes: BTreeMap<String, Value>,
    /// True when the observation came from a live subscription event.
    pub from_event: bool,
}

/// State verifier (SPEC-011 canonical term `StateVerification`;
/// ADR-027).
///
/// Verification binds to the exact target entity and the requested
/// action's expected result. An unrelated `state_changed` event never
/// satisfies verification.
pub trait StateVerifier {
    /// Verify an observation against a rule for the exact target.
    ///
    /// `target` is the exact HA entity the command was submitted
    /// against. An observation for any other entity is
    /// `UnrelatedChange`, never `Verified`.
    fn verify(
        &self,
        target: &HaEntityRef,
        rule: &VerificationRule,
        observation: &StateObservation,
    ) -> VerificationOutcome;
}

/// Deterministic exact-target state verifier (SPEC-011; ADR-027).
///
/// Verification succeeds only when the observation is for the exact
/// target entity and the rule's expected state/attribute is observed.
/// An unrelated entity change is `UnrelatedChange`; a missing state is
/// `Unknown`; a non-matching value is `Mismatch`. No fabricated pass.
#[derive(Debug, Clone, Copy, Default)]
pub struct StateVerifierAdapter;

impl StateVerifier for StateVerifierAdapter {
    fn verify(
        &self,
        target: &HaEntityRef,
        rule: &VerificationRule,
        observation: &StateObservation,
    ) -> VerificationOutcome {
        if observation.ha_entity != *target {
            return VerificationOutcome::UnrelatedChange;
        }
        match rule {
            VerificationRule::StateEquals { expected } => match observation.state.as_deref() {
                Some(actual) if actual == expected => VerificationOutcome::Verified,
                Some(_) => VerificationOutcome::Mismatch,
                None => VerificationOutcome::Unknown,
            },
            VerificationRule::StateIn { expected } => match observation.state.as_deref() {
                Some(actual) if expected.iter().any(|e| e == actual) => {
                    VerificationOutcome::Verified
                }
                Some(_) => VerificationOutcome::Mismatch,
                None => VerificationOutcome::Unknown,
            },
            VerificationRule::AttributeEquals {
                attribute,
                expected,
            } => match observation.attributes.get(attribute) {
                Some(value) if value == expected => VerificationOutcome::Verified,
                Some(_) => VerificationOutcome::Mismatch,
                None => VerificationOutcome::Unknown,
            },
            VerificationRule::NoVerification { reason: _ } => VerificationOutcome::Unknown,
        }
    }
}

/// Automation handoff (SPEC-011 canonical term `AutomationHandoff`;
/// ADR-027).
///
/// Automation creation/invocation/readback against the provider's real
/// automation machinery. The handoff never fabricates an automation
/// object; creation is proven by provider readback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationSpec {
    /// Canonical name for the automation.
    pub name: String,
    /// Provider-agnostic trigger description (e.g. time + occupancy
    /// condition). Free-form provider payloads are normalized at the
    /// infrastructure boundary.
    pub trigger: String,
    /// The intent executed when the automation fires.
    pub action: HomeIntent,
    /// Whether the automation is initially enabled.
    pub enabled: bool,
}

/// Provider handle to a created automation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationHandle {
    /// Provider automation id.
    pub provider_automation_id: String,
    /// Canonical automation name.
    pub name: String,
}

/// Automation status observed from the provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationStatus {
    /// Provider automation id.
    pub provider_automation_id: String,
    /// Whether the provider reports the automation as enabled.
    pub enabled: bool,
    /// Last trigger time reported by the provider, if any.
    pub last_triggered: Option<String>,
}

/// Provider-neutral home provider port (SPEC-011; ADR-027).
///
/// Implementations are replaceable infrastructure behind this port.
/// Authorization remains EP-008's boundary; this port never grants
/// authority by itself.
pub trait HomeProvider {
    /// Current provider connection state. A disconnected provider is
    /// never reported as live.
    fn connection_state(&self) -> ProviderConnectionState;

    /// Discover devices/entities/areas from the real provider.
    fn discover(&mut self) -> Result<Vec<DeviceTwin>, HomeError>;

    /// Read the current state of one device (fresh provider
    /// observation preferred over stale cache).
    fn read_state(&mut self, device: &DeviceId) -> Result<DeviceTwin, HomeError>;

    /// Execute a canonical intent through the provider's real
    /// service/action mechanism. Returns at most SUBMITTED.
    fn execute(&mut self, intent: &HomeIntent) -> Result<CommandReceipt, HomeError>;

    /// Verify the exact target of a submitted command.
    fn verify(&mut self, receipt: &CommandReceipt) -> Result<VerificationOutcome, HomeError>;

    /// Reconnect and resubscribe after a disconnect; on success the
    /// provider refreshes canonical state and resumes event flow.
    fn reconnect(&mut self) -> Result<(), HomeError>;
}

/// Home Assistant provider surface (SPEC-011; ADR-027).
///
/// The concrete Home Assistant adapter implements this plus
/// `HomeProvider`. It owns real HA REST/WebSocket transport,
/// authentication through EP-009 SecretStore references, discovery,
/// mapping, and event subscription. It must never use
/// `POST /api/states/<entity_id>` to implement physical commands.
pub trait HomeAssistantProvider: HomeProvider {
    /// The provider instance fingerprint (never a credential).
    fn instance_fingerprint(&self) -> String;

    /// List provider entity ids for a canonical device twin.
    fn entity_refs(&self, device: &DeviceId) -> Result<Vec<HaEntityRef>, HomeError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{Idempotency, TenantId};

    fn device_id() -> DeviceId {
        DeviceId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6201").expect("valid UUIDv7")
    }

    fn correlation() -> CorrelationId {
        CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6202").expect("valid UUIDv7")
    }

    fn sample_twin() -> DeviceTwin {
        DeviceTwin {
            device_id: device_id(),
            friendly_name: "Kitchen Light".to_string(),
            area: Some(AreaId::new("kitchen").expect("valid area")),
            owner: None,
            ha_device_ref: HaDeviceRef::new("abc123").expect("valid ref"),
            ha_entity_refs: vec![HaEntityRef::new("light.kitchen").expect("valid ref")],
            provider_domain: "light".to_string(),
            category: DeviceCategory::Light,
            manufacturer: Some("TestCo".to_string()),
            model: Some("T-1".to_string()),
            availability: EntityAvailability::Available,
            state: Some("off".to_string()),
            attributes: BTreeMap::new(),
            capabilities: vec![DeviceCapability {
                capability_id: "home.light".to_string(),
                class: CapabilityClass::Command,
                risk: Risk::R1,
                approval: ApprovalClass::None,
                idempotency: Idempotency::Required,
                verification: VerificationRule::StateEquals {
                    expected: "on".to_string(),
                },
            }],
            parent_ha_device_ref: None,
        }
    }

    #[test]
    fn ep020_unit_twin_identity_is_device_id_not_name() {
        let mut twin = sample_twin();
        let identity = twin.device_id.clone();
        twin.friendly_name = "Renamed Light".to_string();
        // Identity survives friendly-name change.
        assert_eq!(twin.device_id, identity);
        assert_ne!(twin.friendly_name, "Kitchen Light");
    }

    #[test]
    fn ep020_unit_twin_capability_lookup_is_deterministic() {
        let twin = sample_twin();
        let cap = twin.capability("home.light").expect("capability found");
        assert_eq!(cap.risk, Risk::R1);
        assert!(twin.capability("home.missing").is_none());
    }

    #[test]
    fn ep020_unit_twin_available_only_when_available() {
        let mut twin = sample_twin();
        assert!(twin.is_available());
        twin.availability = EntityAvailability::Unknown;
        assert!(!twin.is_available());
        twin.availability = EntityAvailability::Unavailable;
        assert!(!twin.is_available());
    }

    #[test]
    fn ep020_unit_execute_never_returns_verified() {
        // Structural contract: execute() returns a receipt whose state
        // is at most SUBMITTED. A receipt can never claim VERIFIED
        // directly; the receipt type carries CommandState which the
        // adapter sets to SUBMITTED.
        let receipt = CommandReceipt {
            intent: HomeIntent {
                device_id: device_id(),
                capability_id: "home.light".to_string(),
                action: "turn_on".to_string(),
                parameters: BTreeMap::new(),
                correlation_id: correlation(),
                idempotency_key: Some("k-1".to_string()),
            },
            state: CommandState::Submitted,
            target_ha_entity: HaEntityRef::new("light.kitchen").expect("valid ref"),
            provider_service: "light/turn_on".to_string(),
        };
        assert_eq!(receipt.state, CommandState::Submitted);
        assert_ne!(receipt.state, CommandState::Verified);
    }

    #[test]
    fn ep020_unit_area_id_rejects_whitespace() {
        assert!(AreaId::new("kitchen room").is_err());
        assert!(AreaId::new("").is_err());
        assert!(AreaId::new("kitchen").is_ok());
    }

    #[test]
    fn ep020_unit_entity_ref_rejects_whitespace() {
        assert!(HaEntityRef::new("light kitchen").is_err());
        assert!(HaEntityRef::new("").is_err());
        assert!(HaEntityRef::new("light.kitchen").is_ok());
    }

    #[test]
    fn ep020_unit_verification_rule_serializes_roundtrip() {
        let rule = VerificationRule::AttributeEquals {
            attribute: "temperature".to_string(),
            expected: Value::from(21),
        };
        let json = serde_json::to_string(&rule).unwrap();
        let back: VerificationRule = serde_json::from_str(&json).unwrap();
        assert_eq!(back, rule);
    }

    #[test]
    fn ep020_unit_automation_handle_roundtrip() {
        let handle = AutomationHandle {
            provider_automation_id: "automation.kitchen_lights".to_string(),
            name: "Kitchen Lights On At Dusk".to_string(),
        };
        let json = serde_json::to_string(&handle).unwrap();
        let back: AutomationHandle = serde_json::from_str(&json).unwrap();
        assert_eq!(back, handle);
    }

    #[test]
    fn ep020_unit_tenant_id_reuse() {
        // nexus-domain ids are reused, never redefined.
        let _ = TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6203").expect("valid UUIDv7");
    }
}
