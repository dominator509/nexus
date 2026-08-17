//! EP-024 irrigation adapter core (SPEC-011; M4).
//!
//! Real production adapter behind the nexus-devices
//! `IrrigationProvider` port: zone discovery, deterministic capability
//! mapping through `DeviceCapabilityMapper`, availability truth table,
//! capability-gated command dispatch, exact-target verification, and
//! bounded observability (redacted audit ring, counters, correlation).
//!
//! Permanent invariants (SPEC-011 / owner directive):
//!
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.
//! - Unrelated zone changes never satisfy verification.
//! - Unsupported capabilities fail closed (Policy) BEFORE any provider
//!   service call.
//! - Unknown zones are NotFound, never Verified and never a benign
//!   zone state.
//! - Provider-unavailable zones map to UNAVAILABLE, never OFF.
//! - Provider domain names never become domain contracts.
//! - Device capability discovered != principal authorized (EP-008 owns
//!   authorization).
//! - Every operation records a correlation id; observability is
//!   bounded and poison-safe (secrets redacted at insert).
//!
//! No test-mode branches exist in production code.

use std::collections::BTreeMap;
use std::sync::Mutex;

use nexus_devices::mapper::DeviceCapabilityMapper;
use nexus_devices::verifier::{
    verify_irrigation, DeviceCommandVerifier, DeviceStateObservation, VerificationOutcome,
};
use nexus_devices::vocabulary::{DeviceAvailability, IrrigationCapability, IrrigationZoneId};
use nexus_devices::{DevicesError, DevicesErrorCode, IrrigationProvider};

use crate::error::{IrrigationError, IrrigationErrorCode};
use crate::observability::IrrigationObservability;
use crate::transport::{
    IrrigationCommand, IrrigationCommandReceipt, IrrigationCommandState, IrrigationTransport,
    IrrigationZone,
};

/// Canonical irrigation zone selector (explicit allowlist).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrrigationZoneSelector {
    entity_ids: Vec<String>,
}

impl IrrigationZoneSelector {
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

/// Deterministic opaque canonical zone id derived from the stable
/// provider entity id (EP-020 stable-identity principle; FNV-1a mix ->
/// hex string).
pub fn stable_zone_id(entity_id: &str) -> String {
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

/// Map zone capabilities from REAL entity features. The fixture zones
/// are binary input_booleans: ZoneControl. Schedule and moisture
/// surfaces are NOT advertised (category-default rule; the fixture has
/// no schedule/moisture provider surface, and the canonical irrigation
/// readback capability is MoistureReadback - which requires a real
/// moisture sensor to advertise).
pub fn capabilities_for(
    zone: &IrrigationZone,
    mapper: &DeviceCapabilityMapper,
) -> Result<Vec<IrrigationCapability>, IrrigationError> {
    for key in [
        "irrigation.zone",
        "irrigation.schedule",
        "irrigation.moisture",
    ] {
        mapper.map(key).map_err(|error| {
            IrrigationError::new(
                IrrigationErrorCode::Internal,
                format!(
                    "canonical irrigation key {key:?} rejected: {}",
                    error.message
                ),
                None,
                None,
            )
        })?;
    }
    let mut capabilities = Vec::new();
    if has_zone_control(zone) {
        capabilities.push(IrrigationCapability::ZoneControl);
    }
    Ok(capabilities)
}

/// True when the zone exposes a real controllable on/off surface.
pub fn has_zone_control(zone: &IrrigationZone) -> bool {
    matches!(zone.domain.as_str(), "input_boolean" | "switch" | "fan")
}

/// Extract the canonical zone state observation ("ON"/"OFF").
/// Provider-unavailable and unknown states are NOT mapped to OFF.
pub fn zone_state_value(zone: &IrrigationZone) -> Option<String> {
    if zone.is_provider_unavailable() || zone.is_state_unknown() {
        None
    } else if zone.is_on() {
        Some("ON".to_string())
    } else {
        Some("OFF".to_string())
    }
}

/// Irrigation adapter implementing the canonical `IrrigationProvider`
/// port with bounded observability.
pub struct IrrigationAdapter<T: IrrigationTransport> {
    transport: Mutex<T>,
    selector: IrrigationZoneSelector,
    mapper: DeviceCapabilityMapper,
    verifier: DeviceCommandVerifier,
    /// In-flight command keys (zone + command) for idempotency.
    in_flight: Mutex<BTreeMap<String, IrrigationCommand>>,
    /// Bounded redacted observability. Secrets are redacted at insert.
    observability: Mutex<IrrigationObservability>,
}

impl<T: IrrigationTransport> IrrigationAdapter<T> {
    pub fn new(transport: T, selector: IrrigationZoneSelector) -> Self {
        Self {
            transport: Mutex::new(transport),
            selector,
            mapper: DeviceCapabilityMapper,
            verifier: DeviceCommandVerifier,
            in_flight: Mutex::new(BTreeMap::new()),
            observability: Mutex::new(IrrigationObservability::default()),
        }
    }

    /// Configure the observability secret set (the provider token is
    /// redacted from every stored audit detail).
    pub fn with_observability_secrets(mut self, secrets: Vec<String>) -> Self {
        self.observability = Mutex::new(IrrigationObservability::new(256, secrets));
        self
    }

    /// Map a canonical irrigation command to its canonical capability
    /// key (EP-010 taxonomy via `DeviceCapabilityMapper`).
    pub fn capability_key(command: IrrigationCommand) -> &'static str {
        match command {
            IrrigationCommand::ZoneOn | IrrigationCommand::ZoneOff => "irrigation.zone",
            IrrigationCommand::SetSchedule => "irrigation.schedule",
        }
    }

    fn device_supports(
        &self,
        capabilities: &[IrrigationCapability],
        command: IrrigationCommand,
    ) -> bool {
        match command {
            IrrigationCommand::ZoneOn | IrrigationCommand::ZoneOff => {
                capabilities.contains(&IrrigationCapability::ZoneControl)
            }
            IrrigationCommand::SetSchedule => {
                capabilities.contains(&IrrigationCapability::ScheduleControl)
            }
        }
    }

    fn provider_zone_id(&self, zone: &IrrigationZoneId) -> Result<String, IrrigationError> {
        self.selector
            .configured()
            .iter()
            .find(|id| stable_zone_id(id) == zone.as_str())
            .cloned()
            .ok_or_else(|| {
                IrrigationError::not_found(format!(
                    "irrigation zone {} is not a configured zone entity",
                    zone.as_str()
                ))
            })
    }

    /// Discover zones: real provider entities selected by the
    /// configured selector, each with a stable canonical identity.
    /// Order is deterministic (sorted by provider entity id), so
    /// discovery never depends on provider enumeration order.
    pub fn discover(&self) -> Result<Vec<IrrigationZone>, IrrigationError> {
        let all = self
            .transport
            .lock()
            .expect("transport lock")
            .list_zones()?;
        let mut zones: Vec<IrrigationZone> = all
            .into_iter()
            .filter(|zone| self.selector.contains(&zone.entity_id))
            .collect();
        zones.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
        Ok(zones)
    }

    /// Execute a command and return a SUBMITTED-at-most receipt.
    ///
    /// Capability gate: an unsupported command fails closed (Policy)
    /// BEFORE any provider service call. Idempotency: a duplicate
    /// in-flight command for the same zone returns Conflict rather than
    /// double-sending. Failures release the in-flight entry (bounded
    /// recovery: a retry after provider recovery is possible).
    pub fn execute(
        &self,
        zone: &IrrigationZoneId,
        command: IrrigationCommand,
        capabilities: &[IrrigationCapability],
    ) -> Result<IrrigationCommandReceipt, IrrigationError> {
        // Correlation is minted under a short lock; the observability
        // lock is NEVER held across the provider dispatch (a slow or
        // blocked provider must not serialize or deadlock commands).
        let correlation = {
            let mut obs = self.observability.lock().expect("observability lock");
            obs.correlation()
        };
        let key = format!("{}:{}", zone.as_str(), command.as_str());
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
                        "duplicate in-flight irrigation command",
                    );
                return Err(IrrigationError::new(
                    IrrigationErrorCode::Conflict,
                    "duplicate in-flight irrigation command",
                    Some(Box::from(correlation.as_str())),
                    Some(Box::from(zone.as_str())),
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
                        "zone {} does not support {}",
                        zone.as_str(),
                        command.as_str()
                    ),
                );
            return Err(IrrigationError::new(
                IrrigationErrorCode::Policy,
                format!(
                    "zone {} does not support {}",
                    zone.as_str(),
                    command.as_str()
                ),
                Some(Box::from(correlation.as_str())),
                Some(Box::from(zone.as_str())),
            ));
        }

        let result = (|| -> Result<(), IrrigationError> {
            let entity_id = self.provider_zone_id(zone)?;
            let zone_entity = self
                .transport
                .lock()
                .expect("transport lock")
                .read_zone(&entity_id)?;
            let (service, data) = match command {
                IrrigationCommand::ZoneOn => ("turn_on", BTreeMap::new()),
                IrrigationCommand::ZoneOff => ("turn_off", BTreeMap::new()),
                IrrigationCommand::SetSchedule => {
                    return Err(IrrigationError::policy(format!(
                        "zone {} does not support SET_SCHEDULE",
                        zone.as_str()
                    )));
                }
            };
            self.transport.lock().expect("transport lock").invoke(
                &zone_entity.domain,
                service,
                &zone_entity.entity_id,
                &data,
            )
        })();

        match result {
            Ok(()) => {
                self.in_flight.lock().expect("in-flight lock").remove(&key);
                self.observability
                    .lock()
                    .expect("observability lock")
                    .record(correlation, command.as_str(), "ok", zone.as_str());
                Ok(IrrigationCommandReceipt {
                    zone: zone.as_str().to_string(),
                    command,
                    state: IrrigationCommandState::Submitted,
                })
            }
            Err(error) => {
                self.in_flight.lock().expect("in-flight lock").remove(&key);
                // Preserve correlation + resource on EVERY error path
                // (directive L): the canonical id survives command ->
                // provider -> readback -> audit -> caller.
                let mut error = error;
                if error.correlation.is_none() {
                    error.correlation = Some(Box::from(correlation.as_str()));
                }
                if error.resource.is_none() {
                    error.resource = Some(Box::from(zone.as_str()));
                }
                self.observability
                    .lock()
                    .expect("observability lock")
                    .record(
                        correlation,
                        command.as_str(),
                        error.code.as_str(),
                        &format!("{} on {}", error.message, zone.as_str()),
                    );
                Err(error)
            }
        }
    }

    /// Verify a command outcome with a fresh exact-target readback.
    pub fn verify(
        &self,
        zone: &IrrigationZoneId,
        command: IrrigationCommand,
        expected: &str,
    ) -> Result<VerificationOutcome, IrrigationError> {
        // Correlation under a short lock; the observability lock is
        // never held across the provider read.
        let correlation = {
            let mut obs = self.observability.lock().expect("observability lock");
            obs.correlation()
        };
        let entity_id = self.provider_zone_id(zone)?;
        let zone_entity = self
            .transport
            .lock()
            .expect("transport lock")
            .read_zone(&entity_id)?;
        let observation = DeviceStateObservation {
            device: zone.as_str().to_string(),
            state: zone_state_value(&zone_entity),
        };
        let outcome = verify_irrigation(&self.verifier, zone, expected, &observation);
        if outcome == VerificationOutcome::Verified {
            self.observability
                .lock()
                .expect("observability lock")
                .record(correlation, "verify", "ok", zone.as_str());
            Ok(outcome)
        } else {
            self.observability
                .lock()
                .expect("observability lock")
                .record(
                    correlation.clone(),
                    "verify",
                    "VERIFICATION",
                    &format!("{} on {}", outcome.as_str(), zone.as_str()),
                );
            // Preserve correlation + resource on the verification error
            // path (directive L).
            Err(IrrigationError::new(
                IrrigationErrorCode::Verification,
                format!(
                    "verification for {} on {}: {}",
                    command.as_str(),
                    zone.as_str(),
                    outcome.as_str()
                ),
                Some(Box::from(correlation.as_str())),
                Some(Box::from(zone.as_str())),
            ))
        }
    }

    /// Zone availability truth table: present + usable -> AVAILABLE;
    /// provider-unavailable -> UNAVAILABLE; unknown -> AVAILABLE (never
    /// claimed OFF); absent -> NotFound; provider offline -> UNAVAILABLE.
    pub fn availability(
        &self,
        zone: &IrrigationZoneId,
    ) -> Result<DeviceAvailability, IrrigationError> {
        let entity_id = match self.provider_zone_id(zone) {
            Ok(id) => id,
            Err(error) if error.code == IrrigationErrorCode::NotFound => return Err(error),
            Err(error) => return Err(error),
        };
        match self
            .transport
            .lock()
            .expect("transport lock")
            .read_zone(&entity_id)
        {
            Ok(zone_entity) => {
                if zone_entity.is_provider_unavailable() {
                    Ok(DeviceAvailability::Unavailable)
                } else if zone_entity.is_state_unknown() || zone_state_value(&zone_entity).is_some()
                {
                    Ok(DeviceAvailability::Available)
                } else {
                    Ok(DeviceAvailability::Discovered)
                }
            }
            Err(error)
                if error.code == IrrigationErrorCode::Unavailable
                    || error.code == IrrigationErrorCode::NotFound =>
            {
                Ok(DeviceAvailability::Unavailable)
            }
            Err(error) => Err(error),
        }
    }

    /// Bounded redacted audit ring (already redacted at insert).
    pub fn audit(&self) -> Vec<crate::observability::IrrigationAuditEntry> {
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

impl<T: IrrigationTransport> IrrigationProvider for IrrigationAdapter<T> {
    fn list_zones(&self) -> Result<Vec<IrrigationZoneId>, DevicesError> {
        let zones = self.discover().map_err(DevicesError::from)?;
        zones
            .iter()
            .map(|zone| IrrigationZoneId::new(stable_zone_id(&zone.entity_id)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                DevicesError::new(
                    DevicesErrorCode::Internal,
                    format!(
                        "irrigation discovery returned an invalid id: {}",
                        error.message
                    ),
                    None,
                    None,
                )
            })
    }

    fn capabilities(
        &self,
        zone: &IrrigationZoneId,
    ) -> Result<Vec<IrrigationCapability>, DevicesError> {
        let entity_id = self.provider_zone_id(zone).map_err(DevicesError::from)?;
        let zone_entity = self
            .transport
            .lock()
            .expect("transport lock")
            .read_zone(&entity_id)
            .map_err(DevicesError::from)?;
        capabilities_for(&zone_entity, &self.mapper).map_err(DevicesError::from)
    }

    fn availability(&self, zone: &IrrigationZoneId) -> Result<DeviceAvailability, DevicesError> {
        self.availability(zone).map_err(DevicesError::from)
    }
}

/// Convenience: build the canonical zone id for a configured entity id.
pub fn irrigation_zone_id(entity_id: &str) -> Result<IrrigationZoneId, IrrigationError> {
    IrrigationZoneId::new(stable_zone_id(entity_id)).map_err(|error| {
        IrrigationError::new(IrrigationErrorCode::Internal, error.message, None, None)
    })
}
