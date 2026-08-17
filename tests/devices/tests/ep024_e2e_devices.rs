//! EP-024 M2 cross-component E2E: acceptance obligations (SPEC-011).
//!
//! Real composition of the production EP-024 components:
//! - nexus-devices (contracts: MediaProvider, ApplianceProvider,
//!   IrrigationProvider, VacuumProvider, RobotProvider,
//!   DeviceCapabilityMapper, DeviceCommandVerifier, RobotSafetyDeclaration);
//! - nexus-media (MediaAdapter core on a controlled fixture transport);
//! - nexus-robotics (fail-closed RobotProviderHost).
//!
//! Proves the node contract acceptance obligations:
//! - Home Assistant is preferred for commodity devices (obligation 1):
//!   the media adapter is transport-neutral, so a Home Assistant
//!   transport binds behind the same port (EP-020 precedent);
//! - Direct providers exist only for capability or reliability gaps
//!   (obligation 2): robotics/other connectors fail closed until a
//!   capability gap justifies a direct transport;
//! - Commands are target-scoped and verified (obligation 3): media
//!   command receipts are SUBMITTED at most, verification is
//!   exact-target and unrelated changes never verify;
//! - Future robots receive no broader authority than declared
//!   capabilities (obligation 4): activation of an undeclared
//!   capability is Policy-refused by the robotics host.

use std::cell::RefCell;
use std::collections::HashMap;

use nexus_devices::robot::RobotSafetyDeclaration;
use nexus_devices::verifier::VerificationOutcome;
use nexus_devices::vocabulary::{MediaCapability, MediaDeviceId, RobotCapability, RobotId};
use nexus_devices::{MediaProvider, RobotProvider};
use nexus_media::adapter::MediaAdapter;
use nexus_media::transport::{MediaCommand, MediaState, MediaTransport};
use nexus_media::{MediaError, MediaErrorCode};
use nexus_robotics::RobotProviderHost;

fn media_id(id: &str) -> MediaDeviceId {
    MediaDeviceId::new(id).expect("media device id")
}

/// Controlled-fixture media transport (TESTING.md test-double zone).
#[derive(Default)]
struct FixtureTransport {
    devices: Vec<String>,
    states: RefCell<HashMap<String, MediaState>>,
    playback: RefCell<HashMap<String, String>>,
}

impl FixtureTransport {
    fn with(device: &str, playback: &str) -> Self {
        let mut transport = Self::default();
        transport.devices.push(device.to_string());
        transport.states.borrow_mut().insert(
            device.to_string(),
            MediaState {
                device: device.to_string(),
                playback: Some(playback.to_string()),
                volume: Some(30),
                source: Some("spotify".to_string()),
                power: Some("ON".to_string()),
            },
        );
        transport
            .playback
            .borrow_mut()
            .insert(device.to_string(), playback.to_string());
        transport
    }
}

impl MediaTransport for FixtureTransport {
    fn list_devices(&self) -> Result<Vec<String>, MediaError> {
        Ok(self.devices.clone())
    }

    fn state(&self, device: &str) -> Result<MediaState, MediaError> {
        self.states
            .borrow()
            .get(device)
            .cloned()
            .ok_or_else(|| MediaError::new(MediaErrorCode::NotFound, "unknown device", None, None))
    }

    fn send_command(&self, device: &str, command: MediaCommand) -> Result<(), MediaError> {
        let mut playback = self.playback.borrow_mut();
        if let Some(state) = playback.get_mut(device) {
            match command {
                MediaCommand::Play => *state = "PLAYING".to_string(),
                MediaCommand::Pause => *state = "PAUSED".to_string(),
                MediaCommand::Stop => *state = "STOPPED".to_string(),
                _ => {}
            }
        }
        if let Some(state) = self.states.borrow_mut().get_mut(device) {
            state.playback = playback.get(device).cloned();
        }
        Ok(())
    }
}

fn all_capabilities() -> Vec<MediaCapability> {
    vec![
        MediaCapability::Playback,
        MediaCapability::Volume,
        MediaCapability::Source,
        MediaCapability::Power,
    ]
}

#[test]
fn ep024_e2e_media_command_target_scoped_and_verified() {
    // Obligation 3: commands are target-scoped and verified.
    let transport = FixtureTransport::with("sonos-living", "PAUSED");
    let adapter = MediaAdapter::new(transport);
    let receipt = adapter
        .execute(
            &media_id("sonos-living"),
            MediaCommand::Play,
            &all_capabilities(),
        )
        .expect("command accepted");
    assert_eq!(
        receipt.state,
        nexus_media::transport::MediaCommandState::Submitted
    );
    // Verification reads back the exact target state.
    let outcome = adapter
        .verify(&media_id("sonos-living"), MediaCommand::Play, "PLAYING")
        .expect("verified");
    assert_eq!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep024_e2e_media_unrelated_target_never_verifies() {
    // Obligation 3: verification binds to the exact target. A target
    // the transport cannot observe fails closed (NotFound) and is never
    // Verified.
    let transport = FixtureTransport::with("sonos-living", "PLAYING");
    let adapter = MediaAdapter::new(transport);
    let error = adapter
        .verify(&media_id("sonos-other"), MediaCommand::Play, "PLAYING")
        .expect_err("unrelated target never verifies");
    assert_eq!(error.code, MediaErrorCode::NotFound);
}

#[test]
fn ep024_e2e_media_provider_neutral_transport() {
    // Obligation 1: the media surface is provider-neutral; a Home
    // Assistant transport (EP-020) or a direct Sonos transport binds
    // behind the same port without touching the adapter.
    let transport = FixtureTransport::with("tv-living", "IDLE");
    let adapter = MediaAdapter::new(transport);
    let devices = adapter.list_devices().expect("devices");
    assert_eq!(devices, vec![media_id("tv-living")]);
    let capabilities = adapter
        .capabilities(&media_id("tv-living"))
        .expect("capabilities");
    assert!(capabilities.contains(&MediaCapability::Playback));
}

#[test]
fn ep024_e2e_robot_no_broader_authority_than_declared() {
    // Obligation 4: future robots receive no broader authority than
    // declared capabilities.
    let host = RobotProviderHost;
    let declaration = RobotSafetyDeclaration::new(
        "workshop",
        0.5,
        5.0,
        vec!["bumper".to_string()],
        true,
        true,
        nexus_devices::ApprovalClass::Human,
        vec![RobotCapability::Navigation],
    )
    .expect("valid declaration");
    // Navigation declared -> allowed by the gating rule.
    assert!(host
        .validate_activation(&declaration, RobotCapability::Navigation)
        .is_ok());
    // Manipulation undeclared -> Policy refusal; the robot never
    // receives broader authority.
    let error = host
        .validate_activation(&declaration, RobotCapability::Manipulation)
        .expect_err("undeclared refused");
    assert_eq!(error.code, nexus_devices::DevicesErrorCode::Policy);
}

#[test]
fn ep024_e2e_robot_inventory_never_fabricated() {
    // No hardware certified: the robotics host truthfully reports an
    // empty inventory (Reality rule; never fabricate devices).
    let host = RobotProviderHost;
    assert!(host.list_robots().expect("empty inventory").is_empty());
    let robot = RobotId::new("robo-1").expect("id");
    assert_eq!(
        host.availability(&robot).expect("availability"),
        nexus_devices::vocabulary::DeviceAvailability::Unavailable
    );
}
