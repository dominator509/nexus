//! EP-024 appliance adapter core (SPEC-011; M3).
//!
//! Real production adapter behind the nexus-devices `ApplianceProvider`
//! port: appliance discovery, deterministic capability mapping through
//! `DeviceCapabilityMapper`, availability truth table, capability-gated
//! command dispatch, and exact-target verification. Commands flow
//! through a real provider service/action; SUBMITTED is never VERIFIED.
//!
//! Permanent invariants (SPEC-011 / owner directive):
//!
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED. A receipt
//!   is SUBMITTED at most; verification is a fresh exact-target
//!   readback.
//! - Unrelated device changes never satisfy verification.
//! - Unsupported capabilities fail closed (Policy) BEFORE any provider
//!   service call; a switch fixture never accepts fan-speed commands.
//! - Unknown targets are NotFound at the transport/adapter boundary,
//!   never Verified and never a benign device state.
//! - Provider-unavailable entities map to UNAVAILABLE, never OFF.
//! - Provider domain names are normalized at the transport boundary and
//!   never become domain contracts; capability keys are canonical.
//! - Device capability discovered != principal authorized (EP-008 owns
//!   authorization; this adapter never decides authorization).
//! - Robot authority is not widened by any other device class.
//!
//! No test-mode branches exist in production code.

use std::collections::BTreeMap;
use std::sync::Mutex;

use nexus_devices::mapper::DeviceCapabilityMapper;
use nexus_devices::verifier::{
    verify_appliance, DeviceCommandVerifier, DeviceStateObservation, VerificationOutcome,
};
use nexus_devices::vocabulary::{ApplianceCapability, ApplianceDeviceId, DeviceAvailability};
use nexus_devices::{ApplianceProvider, DevicesError, DevicesErrorCode};

use crate::error::{ApplianceError, ApplianceErrorCode};
use crate::mapping::{
    capabilities_for, mode_payload, mode_value, power_value, stable_appliance_id, ApplianceSelector,
};
use crate::transport::{
    ApplianceCommand, ApplianceCommandReceipt, ApplianceCommandState, ApplianceEntity,
    ApplianceState, ApplianceTransport,
};

/// Appliance adapter implementing the canonical `ApplianceProvider`
/// port.
///
/// `T` is the real transport (composing through the EP-020-certified
/// Home Assistant boundary) or a controlled fixture in test zones.
/// Interior mutability (Mutex) lets the `&self` port methods drive the
/// stateful transport and the in-flight command map; the adapter is
/// thread-safe when `T: Send + Sync`.
pub struct ApplianceAdapter<T: ApplianceTransport> {
    transport: Mutex<T>,
    selector: ApplianceSelector,
    mapper: DeviceCapabilityMapper,
    verifier: DeviceCommandVerifier,
    /// In-flight command keys (device + command) for idempotency.
    in_flight: Mutex<BTreeMap<String, ApplianceCommand>>,
}

impl<T: ApplianceTransport> ApplianceAdapter<T> {
    pub fn new(transport: T, selector: ApplianceSelector) -> Self {
        Self {
            transport: Mutex::new(transport),
            selector,
            mapper: DeviceCapabilityMapper,
            verifier: DeviceCommandVerifier,
            in_flight: Mutex::new(BTreeMap::new()),
        }
    }

    /// Map a canonical appliance command to its canonical capability
    /// key (EP-010 taxonomy via `DeviceCapabilityMapper`).
    pub fn capability_key(command: ApplianceCommand) -> &'static str {
        match command {
            ApplianceCommand::PowerOn | ApplianceCommand::PowerOff => "appliance.power",
            ApplianceCommand::SetMode => "appliance.mode",
        }
    }

    /// True when the device exposes the capability backing `command`.
    fn device_supports(
        &self,
        capabilities: &[ApplianceCapability],
        command: ApplianceCommand,
    ) -> bool {
        match command {
            ApplianceCommand::PowerOn | ApplianceCommand::PowerOff => {
                capabilities.contains(&ApplianceCapability::PowerControl)
            }
            ApplianceCommand::SetMode => capabilities.contains(&ApplianceCapability::ModeControl),
        }
    }

    /// Read the real entity behind a canonical device id.
    fn read_entity(&self, device: &ApplianceDeviceId) -> Result<ApplianceEntity, ApplianceError> {
        // The canonical id is opaque; the transport needs the provider
        // entity id. Resolution is through the configured selector
        // (explicit mapping), never by guessing.
        let entity_id = self
            .selector
            .configured()
            .iter()
            .find(|id| stable_appliance_id(id) == device.as_str())
            .cloned()
            .ok_or_else(|| {
                ApplianceError::not_found(format!(
                    "appliance device {} is not a configured appliance entity",
                    device.as_str()
                ))
            })?;
        self.transport
            .lock()
            .expect("transport lock")
            .read_appliance(&entity_id)
    }

    /// Discover appliances: real provider entities selected by the
    /// configured selector, each with a stable canonical identity.
    pub fn discover(&self) -> Result<Vec<ApplianceEntity>, ApplianceError> {
        let all = self
            .transport
            .lock()
            .expect("transport lock")
            .list_appliances()?;
        Ok(all
            .into_iter()
            .filter(|entity| self.selector.contains(&entity.entity_id))
            .collect())
    }

    /// Execute a command and return a SUBMITTED-at-most receipt.
    ///
    /// Capability gate: an unsupported command fails closed (Policy)
    /// BEFORE any provider service call. Idempotency: a duplicate
    /// in-flight command for the same device returns Conflict rather
    /// than double-sending.
    pub fn execute(
        &self,
        device: &ApplianceDeviceId,
        command: ApplianceCommand,
        capabilities: &[ApplianceCapability],
    ) -> Result<ApplianceCommandReceipt, ApplianceError> {
        let key = format!("{}:{}", device.as_str(), command.as_str());
        {
            let mut in_flight = self.in_flight.lock().expect("in-flight lock");
            if in_flight.contains_key(&key) {
                return Err(ApplianceError::new(
                    ApplianceErrorCode::Conflict,
                    "duplicate in-flight appliance command",
                    None,
                    Some(Box::from(device.as_str())),
                ));
            }
            in_flight.insert(key.clone(), command);
        }

        // Capability gate BEFORE provider mutation.
        if !self.device_supports(capabilities, command) {
            self.in_flight.lock().expect("in-flight lock").remove(&key);
            return Err(ApplianceError::policy(format!(
                "device {} does not support {}",
                device.as_str(),
                command.as_str()
            )));
        }

        let entity = match self.read_entity(device) {
            Ok(entity) => entity,
            Err(error) => {
                self.in_flight.lock().expect("in-flight lock").remove(&key);
                return Err(error);
            }
        };

        let result = self.invoke_command(&entity, command, None);
        match result {
            Ok(()) => {
                self.in_flight.lock().expect("in-flight lock").remove(&key);
                Ok(ApplianceCommandReceipt {
                    device: device.as_str().to_string(),
                    command,
                    state: ApplianceCommandState::Submitted,
                })
            }
            Err(error) => {
                self.in_flight.lock().expect("in-flight lock").remove(&key);
                Err(error)
            }
        }
    }

    /// Execute a mode command with a runtime-generated value (e.g. a
    /// random supported fan percentage). The value is passed through
    /// exactly and read back for verification.
    pub fn execute_mode(
        &self,
        device: &ApplianceDeviceId,
        value: &str,
        capabilities: &[ApplianceCapability],
    ) -> Result<ApplianceCommandReceipt, ApplianceError> {
        if !capabilities.contains(&ApplianceCapability::ModeControl) {
            return Err(ApplianceError::policy(format!(
                "device {} does not support SET_MODE",
                device.as_str()
            )));
        }
        let entity = self.read_entity(device)?;
        let result = self.invoke_command(&entity, ApplianceCommand::SetMode, Some(value));
        match result {
            Ok(()) => Ok(ApplianceCommandReceipt {
                device: device.as_str().to_string(),
                command: ApplianceCommand::SetMode,
                state: ApplianceCommandState::Submitted,
            }),
            Err(error) => Err(error),
        }
    }

    /// Build and invoke the real provider service/action for a command.
    fn invoke_command(
        &self,
        entity: &ApplianceEntity,
        command: ApplianceCommand,
        mode_value: Option<&str>,
    ) -> Result<(), ApplianceError> {
        let (service, data) = match command {
            ApplianceCommand::PowerOn => ("turn_on", BTreeMap::new()),
            ApplianceCommand::PowerOff => ("turn_off", BTreeMap::new()),
            ApplianceCommand::SetMode => {
                ("set_percentage", mode_payload(mode_value.unwrap_or("100")))
            }
        };
        self.transport.lock().expect("transport lock").invoke(
            &entity.domain,
            service,
            &entity.entity_id,
            &data,
        )
    }

    /// Verify a command outcome with a fresh exact-target readback.
    ///
    /// An unrelated device change is never Verified; a mismatch or an
    /// unobservable target fails closed.
    pub fn verify(
        &self,
        device: &ApplianceDeviceId,
        command: ApplianceCommand,
        expected: &str,
    ) -> Result<VerificationOutcome, ApplianceError> {
        let state = self
            .transport
            .lock()
            .expect("transport lock")
            .read_appliance(&self.provider_entity_id(device)?)?;
        let observation = DeviceStateObservation {
            device: device.as_str().to_string(),
            state: state_value(&state, command),
        };
        let outcome = verify_appliance(&self.verifier, device, expected, &observation);
        if outcome == VerificationOutcome::Verified {
            Ok(outcome)
        } else {
            Err(ApplianceError::verification(format!(
                "verification for {} on {}: {}",
                command.as_str(),
                device.as_str(),
                outcome.as_str()
            )))
        }
    }

    /// Device availability truth table (directive I):
    /// - entity present and usable -> AVAILABLE
    /// - entity present but provider-unavailable -> UNAVAILABLE
    /// - entity absent -> NotFound
    /// - provider offline -> UNAVAILABLE
    pub fn availability(
        &self,
        device: &ApplianceDeviceId,
    ) -> Result<DeviceAvailability, ApplianceError> {
        let entity_id = match self.provider_entity_id(device) {
            Ok(id) => id,
            Err(error) if error.code == ApplianceErrorCode::NotFound => {
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        match self
            .transport
            .lock()
            .expect("transport lock")
            .read_appliance(&entity_id)
        {
            Ok(entity) => {
                if entity.is_provider_unavailable() {
                    Ok(DeviceAvailability::Unavailable)
                } else if entity.is_state_unknown() || power_value(&entity).is_some() {
                    // "unknown" (observed: template entities before
                    // first actuation) is present + usable, so the
                    // device is AVAILABLE even though its state is
                    // never claimed as OFF.
                    Ok(DeviceAvailability::Available)
                } else {
                    Ok(DeviceAvailability::Discovered)
                }
            }
            Err(error)
                if error.code == ApplianceErrorCode::Unavailable
                    || error.code == ApplianceErrorCode::NotFound =>
            {
                // Provider offline or unobservable target is never
                // advertised as available (configured != reachable !=
                // usable; Reality rule).
                Ok(DeviceAvailability::Unavailable)
            }
            Err(error) => Err(error),
        }
    }

    fn provider_entity_id(&self, device: &ApplianceDeviceId) -> Result<String, ApplianceError> {
        self.selector
            .configured()
            .iter()
            .find(|id| stable_appliance_id(id) == device.as_str())
            .cloned()
            .ok_or_else(|| {
                ApplianceError::not_found(format!(
                    "appliance device {} is not a configured appliance entity",
                    device.as_str()
                ))
            })
    }
}

/// Map an appliance entity to the canonical observation value for a
/// command (exact-target readback value).
fn state_value(entity: &ApplianceEntity, command: ApplianceCommand) -> Option<String> {
    match command {
        ApplianceCommand::PowerOn | ApplianceCommand::PowerOff => power_value(entity),
        ApplianceCommand::SetMode => mode_value(entity),
    }
}

impl<T: ApplianceTransport> ApplianceProvider for ApplianceAdapter<T> {
    fn list_devices(&self) -> Result<Vec<ApplianceDeviceId>, DevicesError> {
        let entities = self.discover().map_err(DevicesError::from)?;
        entities
            .iter()
            .map(|entity| ApplianceDeviceId::new(stable_appliance_id(&entity.entity_id)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                DevicesError::new(
                    DevicesErrorCode::Internal,
                    format!(
                        "appliance discovery returned an invalid id: {}",
                        error.message
                    ),
                    None,
                    None,
                )
            })
    }

    fn capabilities(
        &self,
        device: &ApplianceDeviceId,
    ) -> Result<Vec<ApplianceCapability>, DevicesError> {
        let entity = self.read_entity(device).map_err(DevicesError::from)?;
        capabilities_for(&entity, &self.mapper).map_err(DevicesError::from)
    }

    fn availability(&self, device: &ApplianceDeviceId) -> Result<DeviceAvailability, DevicesError> {
        self.availability(device).map_err(DevicesError::from)
    }
}
/// Convenience: build the canonical id for a configured entity id.
pub fn appliance_device_id(entity_id: &str) -> Result<ApplianceDeviceId, ApplianceError> {
    ApplianceDeviceId::new(stable_appliance_id(entity_id)).map_err(|error| {
        ApplianceError::new(ApplianceErrorCode::Internal, error.message, None, None)
    })
}

/// Read a fresh appliance state through the transport (integration
/// helper used by the live suite for independent readback).
pub fn read_state<T: ApplianceTransport>(
    adapter: &ApplianceAdapter<T>,
    device: &ApplianceDeviceId,
) -> Result<ApplianceState, ApplianceError> {
    let entity = adapter.read_entity(device)?;
    Ok(ApplianceState {
        device: device.as_str().to_string(),
        power: power_value(&entity),
        mode: mode_value(&entity),
    })
}
