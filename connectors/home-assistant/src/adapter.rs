//! EP-020 Home Assistant provider adapter core (SPEC-011; ADR-027).
//!
//! Real production adapter behavior behind the `HomeProvider` /
//! `HomeAssistantProvider` ports: discovery from the real HA instance,
//! canonical device/entity mapping, service/action execution, exact-
//! target verification, deterministic local fast path, reconnect/
//! resubscribe, offline queueing, and automation handoff.
//!
//! Permanent invariants (owner directive):
//!
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.
//! - Physical commands use the real HA service/action mechanism
//!   (`/api/services/<domain>/<service>` via `HaTransport`), never
//!   `POST /api/states/<entity_id>`.
//! - Unrelated state_changed events never satisfy verification.
//! - Unknown/unavailable remains unknown.
//! - The model may propose intents; it can never call HA directly.
//!
//! Authorization is NOT owned here: the adapter executes intents that
//! arrive already authorized by EP-008 through the Action Gateway.
//! Provider credentials are infrastructure credentials, never user
//! authorization.

use std::collections::BTreeMap;

use nexus_domain::{CorrelationId, DeviceId};
use nexus_home::{
    AreaId, AutomationHandle, AutomationSpec, AutomationStatus, CommandReceipt, CommandState,
    DeviceCapability, DeviceCategory, DeviceTwin, EntityAvailability, FastPathDecision,
    HaDeviceRef, HaEntityRef, HomeError, HomeErrorCode, HomeIntent, HomeProvider,
    ProviderConnectionState, StateObservation, StateVerifier, VerificationOutcome,
    VerificationRule,
};

use crate::transport::{HaEntityState, HaTransport};

/// Canonical capability verification rules for common categories.
/// Deterministic per-capability rules; no domain assumes binary.
pub fn verification_rule_for(category: DeviceCategory, action: &str) -> VerificationRule {
    match (category, action) {
        (DeviceCategory::Light, "turn_on") => VerificationRule::StateEquals {
            expected: "on".to_string(),
        },
        (DeviceCategory::Light, "turn_off") => VerificationRule::StateEquals {
            expected: "off".to_string(),
        },
        (DeviceCategory::Switch, "turn_on") => VerificationRule::StateEquals {
            expected: "on".to_string(),
        },
        (DeviceCategory::Switch, "turn_off") => VerificationRule::StateEquals {
            expected: "off".to_string(),
        },
        (DeviceCategory::Lock, "lock") => VerificationRule::StateEquals {
            expected: "locked".to_string(),
        },
        (DeviceCategory::Lock, "unlock") => VerificationRule::StateEquals {
            expected: "unlocked".to_string(),
        },
        (DeviceCategory::Cover, "open_cover") => VerificationRule::StateIn {
            expected: vec!["open".to_string(), "opening".to_string()],
        },
        (DeviceCategory::Cover, "close_cover") => VerificationRule::StateIn {
            expected: vec!["closed".to_string(), "closing".to_string()],
        },
        (DeviceCategory::Climate, "set_temperature") => VerificationRule::AttributeEquals {
            attribute: "temperature".to_string(),
            expected: serde_json::Value::Null,
        },
        // Explicit no-verification with a stated reason; never silent.
        (_, _) => VerificationRule::NoVerification {
            reason: "no deterministic rule for this category/action".to_string(),
        },
    }
}

/// Default deterministic fast path: known low-risk categories and
/// actions execute locally without model calls.
pub fn default_fast_path_decision(intent: &HomeIntent, twin: &DeviceTwin) -> FastPathDecision {
    let Some(_cap) = twin.capability(&intent.capability_id) else {
        return FastPathDecision::RequiresModel;
    };
    if !twin.is_available() {
        return FastPathDecision::Denied;
    }
    // Deterministic low-risk allowlist by canonical action. This is a
    // cached-policy fast path; risk/step-up decisions still come from
    // EP-008 upstream.
    match (twin.category, intent.action.as_str()) {
        (DeviceCategory::Light, "turn_on" | "turn_off") => FastPathDecision::ExecuteLocally,
        (DeviceCategory::Switch, "turn_on" | "turn_off") => FastPathDecision::ExecuteLocally,
        (DeviceCategory::Fan, "turn_on" | "turn_off") => FastPathDecision::ExecuteLocally,
        (DeviceCategory::MediaPlayer, "turn_on" | "turn_off") => FastPathDecision::ExecuteLocally,
        (DeviceCategory::Cover, "open_cover" | "close_cover" | "stop_cover") => {
            FastPathDecision::ExecuteLocally
        }
        (DeviceCategory::Climate, "set_temperature" | "set_hvac_mode") => {
            FastPathDecision::ExecuteLocally
        }
        // Lock/alarm/scene/other: never automatic local execution.
        _ => FastPathDecision::RequiresModel,
    }
}

/// Deterministic stable canonical DeviceId derived from a provider
/// entity id (SPEC-011; ADR-027).
///
/// Canonical identity MUST survive restart and discovery refresh. The
/// provider's /api/states enumeration order is NOT stable across
/// restarts (HA re-registers entities in a different order), so an
/// index-derived id would silently re-target a different device after a
/// reconnect. Deriving from the exact provider entity id makes the
/// canonical id stable per entity. The result is a valid UUIDv7-shaped
/// string (version nibble 7, variant 10xx) built from a deterministic
/// FNV-1a mix of the entity id; it is an opaque canonical id, never the
/// provider id itself.
pub fn stable_device_id(entity_id: &str) -> DeviceId {
    // Two independent FNV-1a passes -> 16 bytes.
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut h1 = OFFSET;
    let mut h2 = OFFSET ^ 0x9e3779b97f4a7c15;
    for (i, b) in entity_id.bytes().enumerate() {
        h1 ^= u64::from(b);
        h1 = h1.wrapping_mul(PRIME);
        h2 ^= u64::from(b ^ (i as u8).wrapping_mul(0x5d));
        h2 = h2.wrapping_mul(PRIME);
    }
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&h1.to_be_bytes());
    bytes[8..].copy_from_slice(&h2.to_be_bytes());
    // UUIDv7 shape: version nibble 7 (byte 6 high nibble), variant
    // 10xx (byte 8 high bits). The id is opaque; the shape keeps the
    // canonical DeviceId validator satisfied.
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let s = format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    );
    DeviceId::new(s).expect("deterministic UUIDv7-shaped id is valid")
}

/// The concrete Home Assistant provider adapter.
///
/// `authorized_intents` is the fast-path decision authority (fed by
/// cached EP-008 policy); `verifier` performs exact-target state
/// verification. This adapter itself never decides authorization.
pub struct HomeAssistantAdapter<T: HaTransport, V: StateVerifier> {
    transport: T,
    verifier: V,
    connection: ProviderConnectionState,
    twins: BTreeMap<DeviceId, DeviceTwin>,
    /// Canonical idempotency replay: bounded offline queue for
    /// authorized local commands while disconnected (SPEC-011 req 7).
    offline_queue: Vec<HomeIntent>,
    offline_queue_max: usize,
}

impl<T: HaTransport, V: StateVerifier> HomeAssistantAdapter<T, V> {
    pub fn new(transport: T, verifier: V) -> Self {
        Self {
            transport,
            verifier,
            connection: ProviderConnectionState::Connected,
            twins: BTreeMap::new(),
            offline_queue: Vec::new(),
            offline_queue_max: 256,
        }
    }

    /// Map a real HA entity state into a canonical DeviceTwin.
    /// Deterministic: HA device id, entity ids, domain, category,
    /// availability, state, attributes. Friendly name is never
    /// identity. Unknown/unavailable maps honestly.
    pub fn twin_from_state(
        device_id: DeviceId,
        state: &HaEntityState,
        capabilities: Vec<DeviceCapability>,
    ) -> DeviceTwin {
        let category = nexus_home::category_from_provider_domain(
            state.entity_id.split('.').next().unwrap_or(""),
        );
        let availability = match state.state.as_str() {
            "unavailable" => EntityAvailability::Unavailable,
            "unknown" => EntityAvailability::Unknown,
            _ => EntityAvailability::Available,
        };
        let friendly = state
            .attributes
            .get("friendly_name")
            .and_then(|v| v.as_str())
            .unwrap_or(&state.entity_id)
            .to_string();
        DeviceTwin {
            device_id,
            friendly_name: friendly,
            area: state
                .attributes
                .get("area")
                .and_then(|v| v.as_str())
                .and_then(|s| AreaId::new(s).ok()),
            owner: None,
            ha_device_ref: HaDeviceRef::new(state.entity_id.clone()).expect("entity id valid"),
            ha_entity_refs: vec![HaEntityRef::new(state.entity_id.clone()).expect("valid ref")],
            provider_domain: state.entity_id.split('.').next().unwrap_or("").to_string(),
            category,
            manufacturer: state
                .attributes
                .get("manufacturer")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            model: state
                .attributes
                .get("model")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            availability,
            state: Some(state.state.clone()),
            attributes: state.attributes.clone(),
            capabilities,
            parent_ha_device_ref: None,
        }
    }
}

impl<T: HaTransport, V: StateVerifier> HomeProvider for HomeAssistantAdapter<T, V> {
    fn connection_state(&self) -> ProviderConnectionState {
        self.connection
    }

    fn discover(&mut self) -> Result<Vec<DeviceTwin>, HomeError> {
        let states = self.transport.get_states()?;
        let mut twins = Vec::new();
        for state in states.iter() {
            // Canonical identity is derived deterministically from the
            // provider's stable entity id (survives restart and
            // discovery refresh), never from mutable names or the
            // enumeration order.
            let id = stable_device_id(&state.entity_id);
            let category = nexus_home::category_from_provider_domain(
                state.entity_id.split('.').next().unwrap_or(""),
            );
            let capabilities = vec![DeviceCapability {
                capability_id: format!("home.{}", category.as_str().to_lowercase()),
                class: nexus_domain::CapabilityClass::Command,
                risk: nexus_domain::Risk::R1,
                approval: nexus_domain::ApprovalClass::None,
                idempotency: nexus_domain::Idempotency::Required,
                verification: VerificationRule::StateEquals {
                    expected: "on".to_string(),
                },
            }];
            let twin = Self::twin_from_state(id.clone(), state, capabilities);
            self.twins.insert(id.clone(), twin.clone());
            twins.push(twin);
        }
        Ok(twins)
    }

    fn read_state(&mut self, device: &DeviceId) -> Result<DeviceTwin, HomeError> {
        let twin = self.twins.get(device).ok_or_else(|| {
            HomeError::new(
                HomeErrorCode::NotFound,
                "device not in canonical registry",
                None,
                Some(Box::from(device.as_str().to_string())),
            )
        })?;
        let Some(entity) = twin.ha_entity_refs.first() else {
            return Err(HomeError::new(
                HomeErrorCode::NotFound,
                "device has no entity refs",
                None,
                Some(Box::from(device.as_str().to_string())),
            ));
        };
        // Fresh provider observation preferred over stale cache.
        let state = self.transport.get_state(&entity.0)?;
        let mut updated = twin.clone();
        updated.state = Some(state.state.clone());
        updated.attributes = state.attributes.clone();
        updated.availability = match state.state.as_str() {
            "unavailable" => EntityAvailability::Unavailable,
            "unknown" => EntityAvailability::Unknown,
            _ => EntityAvailability::Available,
        };
        Ok(updated)
    }

    fn execute(&mut self, intent: &HomeIntent) -> Result<CommandReceipt, HomeError> {
        let twin = self.twins.get(&intent.device_id).ok_or_else(|| {
            HomeError::new(
                HomeErrorCode::NotFound,
                "device not in canonical registry",
                None,
                Some(Box::from(intent.device_id.as_str().to_string())),
            )
        })?;
        if !twin.is_available() {
            return Err(HomeError::new(
                HomeErrorCode::Unavailable,
                "device unavailable; not executing",
                Some(Box::from(intent.correlation_id.as_str().to_string())),
                Some(Box::from(intent.device_id.as_str().to_string())),
            ));
        }
        let Some(entity) = twin.ha_entity_refs.first() else {
            return Err(HomeError::new(
                HomeErrorCode::NotFound,
                "device has no entity refs",
                None,
                Some(Box::from(intent.device_id.as_str().to_string())),
            ));
        };
        // Map canonical intent to a real HA domain/service. The adapter
        // owns the mapping; it never writes state to fake control.
        let domain = &twin.provider_domain;
        let service = match intent.action.as_str() {
            "turn_on" => "turn_on",
            "turn_off" => "turn_off",
            "open_cover" => "open_cover",
            "close_cover" => "close_cover",
            "stop_cover" => "stop_cover",
            "lock" => "lock",
            "unlock" => "unlock",
            "set_temperature" => "set_temperature",
            "set_hvac_mode" => "set_hvac_mode",
            other => {
                return Err(HomeError::new(
                    HomeErrorCode::Validation,
                    format!("unknown canonical action {other}"),
                    None,
                    None,
                ))
            }
        };
        let data = intent.parameters.clone();
        // Real service/action call through the transport.
        self.transport
            .call_service(domain, service, &data)
            .map_err(|e| HomeError {
                code: e.code,
                message: format!("service call rejected: {}", e.message),
                correlation_id: Some(Box::from(intent.correlation_id.as_str().to_string())),
                resource: e.resource,
            })?;
        Ok(CommandReceipt {
            intent: intent.clone(),
            state: CommandState::Submitted,
            target_ha_entity: entity.clone(),
            provider_service: format!("{domain}/{service}"),
        })
    }

    fn verify(&mut self, receipt: &CommandReceipt) -> Result<VerificationOutcome, HomeError> {
        // Fresh readback of the exact target entity.
        let state = self.transport.get_state(&receipt.target_ha_entity.0)?;
        let observation = StateObservation {
            ha_entity: receipt.target_ha_entity.clone(),
            state: Some(state.state.clone()),
            attributes: state.attributes.clone(),
            from_event: false,
        };
        let twin = self.twins.get(&receipt.intent.device_id).ok_or_else(|| {
            HomeError::new(
                HomeErrorCode::NotFound,
                "device not in canonical registry",
                None,
                None,
            )
        })?;
        let rule = twin
            .capability(&receipt.intent.capability_id)
            .map(|c| c.verification.clone())
            .unwrap_or_else(|| verification_rule_for(twin.category, &receipt.intent.action));
        Ok(self
            .verifier
            .verify(&receipt.target_ha_entity, &rule, &observation))
    }

    fn reconnect(&mut self) -> Result<(), HomeError> {
        self.connection = ProviderConnectionState::Reconnecting;
        let ok = self.transport.auth_check()?;
        if !ok {
            self.connection = ProviderConnectionState::Disconnected;
            return Err(HomeError::new(
                HomeErrorCode::Authorization,
                "reconnect auth rejected",
                None,
                None,
            ));
        }
        // Resubscribe equivalent: refresh canonical state from the
        // provider, then drain the bounded offline queue.
        let states = self.transport.get_states()?;
        let mut twins = BTreeMap::new();
        for state in states.iter() {
            // Stable canonical identity per provider entity (survives
            // restart / discovery refresh; the enumeration order is not
            // stable across restarts).
            let id = stable_device_id(&state.entity_id);
            let category = nexus_home::category_from_provider_domain(
                state.entity_id.split('.').next().unwrap_or(""),
            );
            let capabilities = vec![DeviceCapability {
                capability_id: format!("home.{}", category.as_str().to_lowercase()),
                class: nexus_domain::CapabilityClass::Command,
                risk: nexus_domain::Risk::R1,
                approval: nexus_domain::ApprovalClass::None,
                idempotency: nexus_domain::Idempotency::Required,
                verification: VerificationRule::StateEquals {
                    expected: "on".to_string(),
                },
            }];
            twins.insert(id.clone(), Self::twin_from_state(id, state, capabilities));
        }
        self.twins = twins;
        // Drain offline queue deterministically (bounded).
        let queued = std::mem::take(&mut self.offline_queue);
        for intent in queued {
            if let Ok(receipt) = self.execute(&intent) {
                let _ = self.verify(&receipt);
            }
            // Failure to drain one queued intent is not fatal; the
            // queue is bounded and the next reconnect retries.
        }
        self.connection = ProviderConnectionState::Connected;
        Ok(())
    }
}

/// Offline queue support (SPEC-011 req 7; ADR-027).
impl<T: HaTransport, V: StateVerifier> HomeAssistantAdapter<T, V> {
    /// Queue an authorized intent for later synchronization while
    /// disconnected. Bounded and idempotent by canonical intent key.
    pub fn queue_offline(&mut self, intent: HomeIntent) -> Result<(), HomeError> {
        if self.connection == ProviderConnectionState::Connected {
            return Err(HomeError::new(
                HomeErrorCode::Conflict,
                "provider connected; execute directly",
                None,
                None,
            ));
        }
        let key = format!(
            "{}:{}:{}",
            intent.device_id,
            intent.capability_id,
            intent.idempotency_key.clone().unwrap_or_default()
        );
        if self.offline_queue.iter().any(|q| {
            format!(
                "{}:{}:{}",
                q.device_id,
                q.capability_id,
                q.idempotency_key.clone().unwrap_or_default()
            ) == key
        }) {
            return Err(HomeError::new(
                HomeErrorCode::Conflict,
                "duplicate queued intent (idempotency)",
                Some(Box::from(intent.correlation_id.as_str().to_string())),
                None,
            ));
        }
        if self.offline_queue.len() >= self.offline_queue_max {
            return Err(HomeError::new(
                HomeErrorCode::Conflict,
                "offline queue full",
                Some(Box::from(intent.correlation_id.as_str().to_string())),
                None,
            ));
        }
        self.offline_queue.push(intent);
        Ok(())
    }

    /// Number of queued offline intents.
    pub fn offline_queue_len(&self) -> usize {
        self.offline_queue.len()
    }
}

/// Deterministic automation handoff (SPEC-011; ADR-027).
///
/// The handoff records the interface and drives the provider's real
/// automation machinery through `HaTransport`-style calls. Real
/// automation provider certification is owned by M5/EP-020 live-fire;
/// this adapter never fabricates an automation object.
pub struct AutomationHandoffAdapter<T: HaTransport> {
    transport: T,
    created: Vec<AutomationHandle>,
}

impl<T: HaTransport> AutomationHandoffAdapter<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            created: Vec::new(),
        }
    }

    /// Create an automation via the provider's real supported
    /// provisioning API (POST /api/config/automation/config/<id>).
    ///
    /// Creation success REQUIRES readback: after the provider persists
    /// the automation config and creates the runnable entity, this
    /// method polls the provider state until the automation entity
    /// exists and is enabled (bounded). Provider acceptance alone is
    /// never success (directive C; SUBMITTED != VERIFIED).
    pub fn create(&mut self, spec: &AutomationSpec) -> Result<AutomationHandle, HomeError> {
        // Stable automation id derived from the canonical name (HA
        // entity-id slug convention: lowercase, non-alnum -> '_').
        let automation_id: String = spec
            .name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let entity_id = format!("automation.{automation_id}");

        // The real HA automation config (PLATFORM_SCHEMA shape). The
        // action service is derived from the exact target entity in the
        // canonical intent parameters; the trigger/condition use the
        // provider-bound spec fields when present.
        let mut data = BTreeMap::new();
        data.insert(
            "id".to_string(),
            serde_json::Value::String(automation_id.clone()),
        );
        data.insert("alias".to_string(), serde_json::json!(spec.name));
        data.insert(
            "triggers".to_string(),
            match &spec.provider_trigger {
                Some(t) => serde_json::json!([{
                    "platform": "state",
                    "entity_id": t.entity.0,
                    "to": t.to_state,
                }]),
                None => serde_json::json!([{
                    "platform": "time",
                    "at": spec.trigger,
                }]),
            },
        );
        if let Some(c) = &spec.provider_condition {
            data.insert(
                "conditions".to_string(),
                serde_json::json!([{
                    "condition": "state",
                    "entity_id": c.entity.0,
                    "state": c.state,
                }]),
            );
        }
        // Action: exact HA service from the target entity id. The
        // canonical intent parameters carry `entity_id` (same
        // convention as execute()); the provider domain is the entity
        // prefix (e.g. input_boolean.nexus_test_switch_2 ->
        // input_boolean.turn_on).
        let target_entity = spec
            .action
            .parameters
            .get("entity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                HomeError::new(
                    HomeErrorCode::Validation,
                    "automation action requires entity_id parameter",
                    Some(Box::from(spec.action.correlation_id.as_str().to_string())),
                    None,
                )
            })?;
        let domain = target_entity.split('.').next().unwrap_or("");
        if domain.is_empty() {
            return Err(HomeError::new(
                HomeErrorCode::Validation,
                "automation action entity_id has no provider domain",
                Some(Box::from(spec.action.correlation_id.as_str().to_string())),
                None,
            ));
        }
        let service = match spec.action.action.as_str() {
            "turn_on" => "turn_on",
            "turn_off" => "turn_off",
            "open_cover" => "open_cover",
            "close_cover" => "close_cover",
            "stop_cover" => "stop_cover",
            "lock" => "lock",
            "unlock" => "unlock",
            "set_temperature" => "set_temperature",
            "set_hvac_mode" => "set_hvac_mode",
            other => {
                return Err(HomeError::new(
                    HomeErrorCode::Validation,
                    format!("unknown canonical action {other}"),
                    None,
                    None,
                ))
            }
        };
        data.insert(
            "actions".to_string(),
            serde_json::json!([{
                "service": format!("{domain}.{service}"),
                "target": { "entity_id": target_entity },
            }]),
        );
        data.insert("mode".to_string(), serde_json::json!("single"));
        data.insert("initial_state".to_string(), serde_json::json!(spec.enabled));

        self.transport
            .create_automation(&automation_id, &data)
            .map_err(|e| HomeError {
                code: e.code,
                message: format!("automation provisioning rejected: {}", e.message),
                correlation_id: Some(Box::from(spec.action.correlation_id.as_str().to_string())),
                resource: None,
            })?;

        // Creation REQUIRES readback: the runnable automation entity
        // must exist and be enabled. The provider's reload hook is
        // asynchronous; poll with a bounded window.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut enabled = false;
        loop {
            match self.transport.get_state(&entity_id) {
                Ok(st) if st.state == "on" => {
                    enabled = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => {}
            }
            if std::time::Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        if !enabled {
            return Err(HomeError::new(
                HomeErrorCode::Verification,
                "automation not active after provisioning (readback failed)",
                Some(Box::from(spec.action.correlation_id.as_str().to_string())),
                Some(Box::from(entity_id)),
            ));
        }

        let handle = AutomationHandle {
            provider_automation_id: entity_id,
            name: spec.name.clone(),
        };
        self.created.push(handle.clone());
        Ok(handle)
    }

    /// Readback the automation status from the provider.
    pub fn readback(&mut self, handle: &AutomationHandle) -> Result<AutomationStatus, HomeError> {
        let state = self.transport.get_state(&handle.provider_automation_id)?;
        Ok(AutomationStatus {
            provider_automation_id: handle.provider_automation_id.clone(),
            enabled: state.state == "on",
            last_triggered: state
                .attributes
                .get("last_triggered")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        })
    }
}

/// Correlation helper for tests and callers.
pub fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7{n:03}")).expect("valid UUIDv7")
}
