//! EP-024 M5 vacuum adapter core (SPEC-011; M5).
//!
//! Real production adapter behind the nexus-devices `VacuumProvider`
//! port: vacuum discovery, deterministic capability mapping from REAL
//! provider feature bits, availability truth table, capability-gated
//! command dispatch, exact-target verification, and bounded
//! observability (redacted audit ring, counters, correlation).
//!
//! Permanent invariants (SPEC-011 / owner directive):
//!
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.
//! - Unrelated vacuum changes never satisfy verification.
//! - Unsupported capabilities fail closed (Policy) BEFORE any provider
//!   service call.
//! - Unknown vacuums are NotFound, never Verified and never benign.
//! - Provider-unavailable vacuums map to UNAVAILABLE, never DOCKED/
//!   IDLE/SAFE/COMPLETED.
//! - Provider unknown/unavailable/error states are never mapped to a
//!   safe state.
//! - MapReadback succeeds only on REAL non-empty provider map data;
//!   never fabricated, never a canned artifact, never an empty
//!   success. Safe metadata only (digest/dimensions/reference).
//! - Dock and ReturnHome are DISTINCT Nexus capabilities that map to
//!   the SAME Home Assistant provider action (`vacuum.return_to_base`)
//!   - the mapping is explicit, not two fabricated behaviors.
//! - No blind retry of ambiguous physical commands: UNKNOWN OUTCOME ->
//!   VERIFY FIRST.
//! - Provider domain names never become domain contracts.
//! - Device capability discovered != principal authorized (EP-008 owns
//!   authorization; a valid HA credential is infrastructure access
//!   only, never robot/cleaning authority).
//! - Every operation records a correlation id; observability is
//!   bounded and poison-safe (secrets redacted at insert).
//!
//! No test-mode branches exist in production code.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde_json::Value;

use nexus_devices::mapper::DeviceCapabilityMapper;
use nexus_devices::verifier::{
    verify_vacuum, DeviceCommandVerifier, DeviceStateObservation, VerificationOutcome,
};
use nexus_devices::vocabulary::{DeviceAvailability, VacuumCapability, VacuumDeviceId};
use nexus_devices::{DevicesError, DevicesErrorCode, VacuumProvider};

use crate::error::{VacuumError, VacuumErrorCode};
use crate::observability::VacuumObservability;
use crate::transport::{
    VacuumActivityState, VacuumCommand, VacuumCommandReceipt, VacuumCommandState, VacuumDevice,
    VacuumTransport,
};

/// Real Home Assistant vacuum `supported_features` bits.
///
/// BOUND TO THE OBSERVED pinned build (template vacuum with
/// start/pause/return_to_base configured publishes 12308 =
/// START(4096) | STATE(8192) | PAUSE(4) | RETURN_HOME(16); verified
/// live by ep024_failure_vacuum_probe_capabilities_from_real_features,
/// which records the observed value). Never invented.
const HA_FEATURE_START: u64 = 4096;
const HA_FEATURE_PAUSE: u64 = 4;
const HA_FEATURE_RETURN_HOME: u64 = 16;
// NOTE: no HA_FEATURE_MAP constant - MapReadback is detected from a
// REAL non-empty map attribute surface (has_real_map_surface), never
// from a bare feature bit (REAL data only).

/// Canonical vacuum device selector (explicit allowlist).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VacuumDeviceSelector {
    entity_ids: Vec<String>,
}

impl VacuumDeviceSelector {
    pub fn entities<I, S>(entity_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut entity_ids: Vec<String> = entity_ids.into_iter().map(Into::into).collect();
        entity_ids.sort();
        entity_ids.dedup();
        Self { entity_ids }
    }

    pub fn contains(&self, entity_id: &str) -> bool {
        self.entity_ids.iter().any(|id| id == entity_id)
    }

    pub fn configured(&self) -> &[String] {
        &self.entity_ids
    }
}

/// Deterministic opaque canonical vacuum id derived from the stable
/// provider entity id (EP-020 stable-identity principle; FNV-1a mix ->
/// hex string).
pub fn stable_vacuum_id(entity_id: &str) -> String {
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
    format!("{h1:016x}{h2:016x}")
}

/// True when the entity exposes a REAL non-empty map surface.
///
/// MapReadback is NEVER advertised from a bare feature bit: it
/// requires actual provider map data (e.g. a non-empty `map`
/// attribute). The controlled vacuum fixture has no map surface, so
/// MapReadback is not advertised and the command fails closed.
pub fn has_real_map_surface(device: &VacuumDevice) -> bool {
    match device.attributes.get("map") {
        Some(Value::String(text)) => !text.is_empty(),
        Some(_) => true,
        None => false,
    }
}

/// Map vacuum capabilities from REAL provider feature bits and map
/// surface. Only genuinely advertised/available capabilities are
/// mapped; unsupported capability requests fail closed (Policy)
/// before any provider mutation.
pub fn capabilities_for(
    device: &VacuumDevice,
    mapper: &DeviceCapabilityMapper,
) -> Result<Vec<VacuumCapability>, VacuumError> {
    for key in [
        "vacuum.dock",
        "vacuum.clean",
        "vacuum.pause",
        "vacuum.home",
        "vacuum.map",
    ] {
        mapper.map(key).map_err(|error| {
            VacuumError::new(
                VacuumErrorCode::Internal,
                format!("canonical vacuum key {key:?} rejected: {}", error.message),
                None,
                None,
            )
        })?;
    }
    let mut capabilities = Vec::new();
    let features = device.supported_features().unwrap_or(0);
    if features & HA_FEATURE_START != 0 {
        capabilities.push(VacuumCapability::StartClean);
    }
    if features & HA_FEATURE_PAUSE != 0 {
        capabilities.push(VacuumCapability::Pause);
    }
    if features & HA_FEATURE_RETURN_HOME != 0 {
        // Dock and ReturnHome are distinct Nexus capabilities mapping
        // to the SAME provider action (vacuum.return_to_base); both
        // are advertised together from the return-home feature bit.
        capabilities.push(VacuumCapability::ReturnHome);
        capabilities.push(VacuumCapability::Dock);
    }
    if has_real_map_surface(device) {
        capabilities.push(VacuumCapability::MapReadback);
    }
    Ok(capabilities)
}

/// Extract the canonical vacuum activity state observation.
/// Provider-unavailable and unknown states are NOT mapped to any safe
/// state (never DOCKED/IDLE/SAFE/COMPLETED).
pub fn vacuum_state_value(device: &VacuumDevice) -> Option<VacuumActivityState> {
    if device.is_provider_unavailable() || device.is_state_unknown() {
        return None;
    }
    match device.state.as_str() {
        "cleaning" => Some(VacuumActivityState::Cleaning),
        "docked" => Some(VacuumActivityState::Docked),
        "idle" => Some(VacuumActivityState::Idle),
        "paused" => Some(VacuumActivityState::Paused),
        "returning" => Some(VacuumActivityState::Returning),
        "error" => Some(VacuumActivityState::Error),
        // Any other provider string is unknown -> None, never safe.
        _ => None,
    }
}

/// Vacuum adapter implementing the canonical `VacuumProvider` port
/// with bounded observability.
pub struct VacuumAdapter<T: VacuumTransport> {
    transport: Mutex<T>,
    selector: VacuumDeviceSelector,
    mapper: DeviceCapabilityMapper,
    verifier: DeviceCommandVerifier,
    /// In-flight command keys (device + command) for idempotency.
    in_flight: Mutex<BTreeMap<String, VacuumCommand>>,
    /// Bounded redacted observability. Secrets are redacted at insert.
    observability: Mutex<VacuumObservability>,
}

impl<T: VacuumTransport> VacuumAdapter<T> {
    pub fn new(transport: T, selector: VacuumDeviceSelector) -> Self {
        Self {
            transport: Mutex::new(transport),
            selector,
            mapper: DeviceCapabilityMapper,
            verifier: DeviceCommandVerifier,
            in_flight: Mutex::new(BTreeMap::new()),
            observability: Mutex::new(VacuumObservability::default()),
        }
    }

    /// Configure the observability secret set (the provider token is
    /// redacted from every stored audit detail).
    pub fn with_observability_secrets(mut self, secrets: Vec<String>) -> Self {
        self.observability = Mutex::new(VacuumObservability::new(256, secrets));
        self
    }

    /// Map a canonical vacuum command to its canonical capability key
    /// (EP-010 taxonomy via `DeviceCapabilityMapper`).
    pub fn capability_key(command: VacuumCommand) -> &'static str {
        match command {
            VacuumCommand::StartClean => "vacuum.clean",
            VacuumCommand::Pause => "vacuum.pause",
            VacuumCommand::ReturnHome => "vacuum.home",
            VacuumCommand::Dock => "vacuum.dock",
            VacuumCommand::MapReadback => "vacuum.map",
        }
    }

    fn device_supports(&self, capabilities: &[VacuumCapability], command: VacuumCommand) -> bool {
        match command {
            VacuumCommand::StartClean => capabilities.contains(&VacuumCapability::StartClean),
            VacuumCommand::Pause => capabilities.contains(&VacuumCapability::Pause),
            VacuumCommand::ReturnHome => capabilities.contains(&VacuumCapability::ReturnHome),
            VacuumCommand::Dock => capabilities.contains(&VacuumCapability::Dock),
            VacuumCommand::MapReadback => capabilities.contains(&VacuumCapability::MapReadback),
        }
    }

    fn provider_vacuum_id(&self, device: &VacuumDeviceId) -> Result<String, VacuumError> {
        self.selector
            .configured()
            .iter()
            .find(|id| stable_vacuum_id(id) == device.as_str())
            .cloned()
            .ok_or_else(|| {
                VacuumError::not_found(format!(
                    "vacuum {} is not a configured vacuum entity",
                    device.as_str()
                ))
            })
    }

    /// Discover vacuums: real provider entities selected by the
    /// configured selector, each with a stable canonical identity.
    /// Order is deterministic (sorted by provider entity id), so
    /// discovery never depends on provider enumeration order.
    pub fn discover(&self) -> Result<Vec<VacuumDevice>, VacuumError> {
        let all = self
            .transport
            .lock()
            .expect("transport lock")
            .list_vacuums()?;
        let mut devices: Vec<VacuumDevice> = all
            .into_iter()
            .filter(|device| self.selector.contains(&device.entity_id))
            .collect();
        devices.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
        Ok(devices)
    }

    /// Execute a command and return a SUBMITTED-at-most receipt.
    ///
    /// Capability gate: an unsupported command fails closed (Policy)
    /// BEFORE any provider service call. Idempotency: a duplicate
    /// in-flight command for the same device returns Conflict rather
    /// than double-sending. Physical commands are NEVER blindly
    /// retried: on an ambiguous transport outcome the error is
    /// returned (SUBMITTED/UNKNOWN semantics preserved); the caller
    /// must verify first.
    pub fn execute(
        &self,
        device: &VacuumDeviceId,
        command: VacuumCommand,
        capabilities: &[VacuumCapability],
    ) -> Result<VacuumCommandReceipt, VacuumError> {
        // Correlation is minted under a short lock; the observability
        // lock is NEVER held across the provider dispatch (a slow or
        // blocked provider must not serialize or deadlock commands).
        let correlation = {
            let mut obs = self.observability.lock().expect("observability lock");
            obs.correlation()
        };
        let key = format!("{}:{}", device.as_str(), command.as_str());
        {
            let mut in_flight = self.in_flight.lock().expect("in-flight lock");
            if in_flight.contains_key(&key) {
                self.observability
                    .lock()
                    .expect("observability lock")
                    .record(
                        correlation.clone(),
                        command.as_str(),
                        "CONFLICT",
                        "duplicate in-flight vacuum command",
                    );
                return Err(VacuumError::new(
                    VacuumErrorCode::Conflict,
                    "duplicate in-flight vacuum command",
                    Some(Box::from(correlation.as_str())),
                    Some(Box::from(device.as_str())),
                ));
            }
            in_flight.insert(key.clone(), command);
        }

        // Capability gate BEFORE provider mutation.
        if !self.device_supports(capabilities, command) {
            self.in_flight.lock().expect("in-flight lock").remove(&key);
            self.observability
                .lock()
                .expect("observability lock")
                .record(
                    correlation.clone(),
                    command.as_str(),
                    "POLICY",
                    &format!(
                        "vacuum {} does not support {}",
                        device.as_str(),
                        command.as_str()
                    ),
                );
            return Err(VacuumError::new(
                VacuumErrorCode::Policy,
                format!(
                    "vacuum {} does not support {}",
                    device.as_str(),
                    command.as_str()
                ),
                Some(Box::from(correlation.as_str())),
                Some(Box::from(device.as_str())),
            ));
        }

        let result = (|| -> Result<(), VacuumError> {
            let entity_id = self.provider_vacuum_id(device)?;
            let device_entity = self
                .transport
                .lock()
                .expect("transport lock")
                .read_vacuum(&entity_id)?;
            let (domain, service, data) = match command {
                VacuumCommand::StartClean => ("vacuum", "start", BTreeMap::new()),
                VacuumCommand::Pause => ("vacuum", "pause", BTreeMap::new()),
                // Dock and ReturnHome are DISTINCT Nexus capabilities
                // that map to the SAME real provider action
                // (vacuum.return_to_base) - explicit mapping, not two
                // fabricated behaviors.
                VacuumCommand::ReturnHome | VacuumCommand::Dock => {
                    ("vacuum", "return_to_base", BTreeMap::new())
                }
                // MapReadback is a read-only capability; when
                // advertised it is handled by `map_readback`, never by
                // a mutating provider action.
                VacuumCommand::MapReadback => {
                    return Err(VacuumError::policy(format!(
                        "vacuum {} MAP_READBACK is a read-only query",
                        device.as_str()
                    )));
                }
            };
            self.transport.lock().expect("transport lock").invoke(
                domain,
                service,
                &device_entity.entity_id,
                &data,
            )
        })();

        match result {
            Ok(()) => {
                self.in_flight.lock().expect("in-flight lock").remove(&key);
                self.observability
                    .lock()
                    .expect("observability lock")
                    .record(correlation, command.as_str(), "ok", device.as_str());
                Ok(VacuumCommandReceipt {
                    device: device.as_str().to_string(),
                    command,
                    state: VacuumCommandState::Submitted,
                })
            }
            Err(error) => {
                self.in_flight.lock().expect("in-flight lock").remove(&key);
                // Preserve correlation + resource on EVERY error path
                // (directive L): the canonical id survives command ->
                // provider -> readback -> audit -> caller. No blind
                // retry of an ambiguous physical command.
                let mut error = error;
                if error.correlation.is_none() {
                    error.correlation = Some(Box::from(correlation.as_str()));
                }
                if error.resource.is_none() {
                    error.resource = Some(Box::from(device.as_str()));
                }
                self.observability
                    .lock()
                    .expect("observability lock")
                    .record(
                        correlation,
                        command.as_str(),
                        error.code.as_str(),
                        &format!("{} on {}", error.message, device.as_str()),
                    );
                Err(error)
            }
        }
    }

    /// Verify a command outcome with a fresh exact-target readback.
    pub fn verify(
        &self,
        device: &VacuumDeviceId,
        command: VacuumCommand,
        expected: &str,
    ) -> Result<VerificationOutcome, VacuumError> {
        // Correlation under a short lock; the observability lock is
        // never held across the provider read.
        let correlation = {
            let mut obs = self.observability.lock().expect("observability lock");
            obs.correlation()
        };
        let entity_id = self.provider_vacuum_id(device)?;
        let device_entity = self
            .transport
            .lock()
            .expect("transport lock")
            .read_vacuum(&entity_id)?;
        let observation = DeviceStateObservation {
            device: device.as_str().to_string(),
            state: vacuum_state_value(&device_entity).map(|s| s.as_str().to_string()),
        };
        let outcome = verify_vacuum(&self.verifier, device, expected, &observation);
        if outcome == VerificationOutcome::Verified {
            self.observability
                .lock()
                .expect("observability lock")
                .record(correlation, "verify", "ok", device.as_str());
            Ok(outcome)
        } else {
            self.observability
                .lock()
                .expect("observability lock")
                .record(
                    correlation.clone(),
                    "verify",
                    "VERIFICATION",
                    &format!("{} on {}", outcome.as_str(), device.as_str()),
                );
            // Preserve correlation + resource on the verification error
            // path.
            Err(VacuumError::new(
                VacuumErrorCode::Verification,
                format!(
                    "verification for {} on {}: {}",
                    command.as_str(),
                    device.as_str(),
                    outcome.as_str()
                ),
                Some(Box::from(correlation.as_str())),
                Some(Box::from(device.as_str())),
            ))
        }
    }

    /// Zone availability truth table: known + reachable -> AVAILABLE;
    /// provider-unavailable -> UNAVAILABLE; unknown -> AVAILABLE (never
    /// claimed safe); absent -> NotFound; provider offline -> UNAVAILABLE.
    pub fn availability(&self, device: &VacuumDeviceId) -> Result<DeviceAvailability, VacuumError> {
        let entity_id = match self.provider_vacuum_id(device) {
            Ok(id) => id,
            Err(error) if error.code == VacuumErrorCode::NotFound => return Err(error),
            Err(error) => return Err(error),
        };
        match self
            .transport
            .lock()
            .expect("transport lock")
            .read_vacuum(&entity_id)
        {
            Ok(device_entity) => {
                if device_entity.is_provider_unavailable() {
                    Ok(DeviceAvailability::Unavailable)
                } else if device_entity.is_state_unknown()
                    || vacuum_state_value(&device_entity).is_some()
                {
                    Ok(DeviceAvailability::Available)
                } else {
                    Ok(DeviceAvailability::Discovered)
                }
            }
            Err(error)
                if error.code == VacuumErrorCode::Unavailable
                    || error.code == VacuumErrorCode::NotFound =>
            {
                Ok(DeviceAvailability::Unavailable)
            }
            Err(error) => Err(error),
        }
    }

    /// Map readback: REAL non-empty provider map data only.
    ///
    /// Returns safe metadata (digest, dimensions, provider reference);
    /// never raw household map imagery in telemetry/evidence (M5
    /// privacy boundary). When the provider exposes no map surface,
    /// the capability is not advertised and this returns a canonical
    /// unsupported result (fail closed), NOT success.
    pub fn map_readback(
        &self,
        device: &VacuumDeviceId,
        capabilities: &[VacuumCapability],
    ) -> Result<VacuumMapMetadata, VacuumError> {
        let correlation = {
            let mut obs = self.observability.lock().expect("observability lock");
            obs.correlation()
        };
        if !capabilities.contains(&VacuumCapability::MapReadback) {
            self.observability
                .lock()
                .expect("observability lock")
                .record(
                    correlation.clone(),
                    "MAP_READBACK",
                    "POLICY",
                    &format!("vacuum {} has no map surface", device.as_str()),
                );
            return Err(VacuumError::new(
                VacuumErrorCode::Policy,
                format!("vacuum {} has no provider map surface", device.as_str()),
                Some(Box::from(correlation.as_str())),
                Some(Box::from(device.as_str())),
            ));
        }
        let entity_id = self.provider_vacuum_id(device)?;
        let device_entity = self
            .transport
            .lock()
            .expect("transport lock")
            .read_vacuum(&entity_id)?;
        match device_entity.attributes.get("map") {
            Some(serde_json::Value::String(text)) if !text.is_empty() => {
                // Safe metadata only: digest + provider reference, not
                // raw imagery.
                let digest = format!("{:x}", md5_like(text));
                let metadata = VacuumMapMetadata {
                    device: device.as_str().to_string(),
                    digest,
                    reference: "entity.attribute.map".to_string(),
                    bytes: text.len(),
                };
                self.observability
                    .lock()
                    .expect("observability lock")
                    .record(
                        correlation,
                        "MAP_READBACK",
                        "ok",
                        &format!(
                            "vacuum {} map digest={} bytes={}",
                            device.as_str(),
                            metadata.digest,
                            metadata.bytes
                        ),
                    );
                Ok(metadata)
            }
            _ => {
                self.observability
                    .lock()
                    .expect("observability lock")
                    .record(
                        correlation.clone(),
                        "MAP_READBACK",
                        "NOT_FOUND",
                        &format!("vacuum {} has no current map data", device.as_str()),
                    );
                Err(VacuumError::new(
                    VacuumErrorCode::NotFound,
                    format!(
                        "vacuum {} has no current provider map data",
                        device.as_str()
                    ),
                    Some(Box::from(correlation.as_str())),
                    Some(Box::from(device.as_str())),
                ))
            }
        }
    }

    /// Direct real readback of a configured vacuum entity (used by the
    /// ops diagnostic for the current canonical state; always a live
    /// provider read, never cached).
    pub fn read_device(&self, entity_id: &str) -> Result<VacuumDevice, VacuumError> {
        if !self.selector.contains(entity_id) {
            return Err(VacuumError::not_found(format!(
                "vacuum {entity_id:?} is not a configured vacuum entity"
            )));
        }
        self.transport
            .lock()
            .expect("transport lock")
            .read_vacuum(entity_id)
    }

    /// Bounded redacted audit ring (already redacted at insert).
    pub fn audit(&self) -> Vec<crate::observability::VacuumAuditEntry> {
        self.observability
            .lock()
            .expect("observability lock")
            .audit()
    }

    /// Counter snapshot.
    pub fn counters(&self) -> BTreeMap<String, u64> {
        self.observability
            .lock()
            .expect("observability lock")
            .counters()
    }

    /// Bounded recovery: clear any stuck in-flight entries (e.g. after
    /// a provider outage). Returns the number cleared.
    pub fn recover(&self) -> usize {
        let mut in_flight = self.in_flight.lock().expect("in-flight lock");
        let cleared = in_flight.len();
        in_flight.clear();
        cleared
    }
}

/// Safe map readback metadata (never raw household imagery).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VacuumMapMetadata {
    pub device: String,
    pub digest: String,
    pub reference: String,
    pub bytes: usize,
}

/// Small deterministic digest for map data evidence (not
/// cryptographic security; evidence prefer digests over raw imagery).
fn md5_like(text: &str) -> u128 {
    let mut h: u128 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h = h.wrapping_mul(0x100000001b3).wrapping_add(u128::from(b));
    }
    h
}

impl<T: VacuumTransport> VacuumProvider for VacuumAdapter<T> {
    fn list_devices(&self) -> Result<Vec<VacuumDeviceId>, DevicesError> {
        let devices = self.discover().map_err(DevicesError::from)?;
        devices
            .iter()
            .map(|device| VacuumDeviceId::new(stable_vacuum_id(&device.entity_id)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                DevicesError::new(
                    DevicesErrorCode::Internal,
                    format!("vacuum discovery returned an invalid id: {}", error.message),
                    None,
                    None,
                )
            })
    }

    fn capabilities(&self, device: &VacuumDeviceId) -> Result<Vec<VacuumCapability>, DevicesError> {
        let entity_id = self
            .provider_vacuum_id(device)
            .map_err(DevicesError::from)?;
        let device_entity = self
            .transport
            .lock()
            .expect("transport lock")
            .read_vacuum(&entity_id)
            .map_err(DevicesError::from)?;
        capabilities_for(&device_entity, &self.mapper).map_err(DevicesError::from)
    }

    fn availability(&self, device: &VacuumDeviceId) -> Result<DeviceAvailability, DevicesError> {
        self.availability(device).map_err(DevicesError::from)
    }
}

/// Convenience: build the canonical device id for a configured entity id.
pub fn vacuum_device_id(entity_id: &str) -> Result<VacuumDeviceId, VacuumError> {
    VacuumDeviceId::new(stable_vacuum_id(entity_id))
        .map_err(|error| VacuumError::new(VacuumErrorCode::Internal, error.message, None, None))
}
