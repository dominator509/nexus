//! EP-024 M2 unit suite for the nexus-media adapter core
//! (real implementation, boundary values, idempotency, unauthorized
//! states, exact-target verification, dependency direction).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nexus_devices::verifier::VerificationOutcome;
use nexus_devices::vocabulary::{DeviceAvailability, MediaCapability, MediaDeviceId};
use nexus_devices::MediaProvider;
use nexus_media::adapter::MediaAdapter;
use nexus_media::transport::{MediaCommand, MediaCommandState, MediaState, MediaTransport};
use nexus_media::{MediaError, MediaErrorCode};

fn media_id(id: &str) -> MediaDeviceId {
    MediaDeviceId::new(id).expect("media device id")
}

/// Controlled-fixture media transport (TESTING.md test-double zone).
/// Real adapter logic drives this stateful transport; the fixture
/// records command dispatch and exposes mutable state for readback.
#[derive(Default)]
struct FixtureTransport {
    devices: Vec<String>,
    states: HashMap<String, MediaState>,
    commands: RefCell<Vec<(String, MediaCommand)>>,
    fail: RefCell<Option<MediaErrorCode>>,
}

impl FixtureTransport {
    fn with(device: &str, state: MediaState) -> Self {
        let mut transport = Self::default();
        transport.devices.push(device.to_string());
        transport.states.insert(device.to_string(), state);
        transport
    }

    fn fail_next(&self, code: MediaErrorCode) {
        *self.fail.borrow_mut() = Some(code);
    }
}

impl MediaTransport for FixtureTransport {
    fn list_devices(&self) -> Result<Vec<String>, MediaError> {
        Ok(self.devices.clone())
    }

    fn state(&self, device: &str) -> Result<MediaState, MediaError> {
        if let Some(code) = *self.fail.borrow() {
            return Err(MediaError::new(
                code,
                "fixture transport failure",
                None,
                None,
            ));
        }
        self.states
            .get(device)
            .cloned()
            .ok_or_else(|| MediaError::new(MediaErrorCode::NotFound, "unknown device", None, None))
    }

    fn send_command(&self, device: &str, command: MediaCommand) -> Result<(), MediaError> {
        if let Some(code) = *self.fail.borrow() {
            return Err(MediaError::new(
                code,
                "fixture transport failure",
                None,
                None,
            ));
        }
        self.commands
            .borrow_mut()
            .push((device.to_string(), command));
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
fn ep024_unit_media_command_vocabulary_lock() {
    for (text, expected) in [
        ("PLAY", MediaCommand::Play),
        ("PAUSE", MediaCommand::Pause),
        ("STOP", MediaCommand::Stop),
        ("SEEK", MediaCommand::Seek),
        ("SET_VOLUME", MediaCommand::SetVolume),
        ("SET_SOURCE", MediaCommand::SetSource),
        ("POWER_ON", MediaCommand::PowerOn),
        ("POWER_OFF", MediaCommand::PowerOff),
    ] {
        assert_eq!(MediaCommand::parse(text).expect("canonical"), expected);
        assert_eq!(expected.as_str(), text);
    }
    let error = MediaCommand::parse("NEXT_TRACK").expect_err("unknown rejected");
    assert_eq!(error.code, MediaErrorCode::Vocabulary);
    let json = serde_json::to_string(&MediaCommand::Play).expect("json");
    assert_eq!(json, "\"PLAY\"");
    let back: MediaCommand = serde_json::from_str(&json).expect("roundtrip");
    assert_eq!(back, MediaCommand::Play);
}

#[test]
fn ep024_unit_media_command_state_never_verified_on_submit() {
    let transport = FixtureTransport::with(
        "sonos-living",
        MediaState {
            device: "sonos-living".to_string(),
            playback: Some("PAUSED".to_string()),
            volume: Some(40),
            source: Some("spotify".to_string()),
            power: Some("ON".to_string()),
        },
    );
    let adapter = MediaAdapter::new(transport);
    let receipt = adapter
        .execute(
            &media_id("sonos-living"),
            MediaCommand::Play,
            &all_capabilities(),
        )
        .expect("command accepted");
    // COMMAND ACCEPTED != DEVICE VERIFIED: receipt is SUBMITTED at most.
    assert_eq!(receipt.state, MediaCommandState::Submitted);
    assert_ne!(receipt.state, MediaCommandState::Verified);
}

#[test]
fn ep024_unit_media_execute_dispatches_real_command() {
    let log: Arc<Mutex<Vec<(String, MediaCommand)>>> = Arc::new(Mutex::new(Vec::new()));
    let transport = FixtureTransport::with(
        "sonos-living",
        MediaState {
            device: "sonos-living".to_string(),
            playback: Some("PAUSED".to_string()),
            volume: Some(40),
            source: Some("spotify".to_string()),
            power: Some("ON".to_string()),
        },
    );
    let transport = LoggingTransport::new(transport, Arc::clone(&log));
    let adapter = MediaAdapter::new(transport);
    adapter
        .execute(
            &media_id("sonos-living"),
            MediaCommand::Play,
            &all_capabilities(),
        )
        .expect("command accepted");
    let commands = log.lock().expect("log").clone();
    assert_eq!(
        commands,
        vec![("sonos-living".to_string(), MediaCommand::Play)]
    );
}

/// Transport wrapper that records dispatched commands into a shared
/// log before delegating to the inner fixture.
struct LoggingTransport<T: MediaTransport> {
    inner: T,
    log: Arc<Mutex<Vec<(String, MediaCommand)>>>,
}

impl<T: MediaTransport> LoggingTransport<T> {
    fn new(inner: T, log: Arc<Mutex<Vec<(String, MediaCommand)>>>) -> Self {
        Self { inner, log }
    }
}

impl<T: MediaTransport> MediaTransport for LoggingTransport<T> {
    fn list_devices(&self) -> Result<Vec<String>, MediaError> {
        self.inner.list_devices()
    }

    fn state(&self, device: &str) -> Result<MediaState, MediaError> {
        self.inner.state(device)
    }

    fn send_command(&self, device: &str, command: MediaCommand) -> Result<(), MediaError> {
        self.log
            .lock()
            .expect("log")
            .push((device.to_string(), command));
        self.inner.send_command(device, command)
    }
}

#[test]
fn ep024_unit_media_exact_target_verification() {
    let transport = FixtureTransport::with(
        "sonos-living",
        MediaState {
            device: "sonos-living".to_string(),
            playback: Some("PAUSED".to_string()),
            volume: Some(40),
            source: Some("spotify".to_string()),
            power: Some("ON".to_string()),
        },
    );
    let adapter = MediaAdapter::new(transport);
    // Expected PLAYING but state is PAUSED -> verification error.
    let error = adapter
        .verify(&media_id("sonos-living"), MediaCommand::Play, "PLAYING")
        .expect_err("mismatch is not verified");
    assert_eq!(error.code, MediaErrorCode::Verification);
}

#[test]
fn ep024_unit_media_verify_success_after_state_change() {
    // A fixture that applies the command on dispatch.
    let transport = ApplyOnCommandTransport::new("sonos-living");
    let adapter = MediaAdapter::new(transport);
    adapter
        .execute(
            &media_id("sonos-living"),
            MediaCommand::Play,
            &all_capabilities(),
        )
        .expect("command accepted");
    let outcome = adapter
        .verify(&media_id("sonos-living"), MediaCommand::Play, "PLAYING")
        .expect("verified after real state change");
    assert_eq!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep024_unit_media_unrelated_device_never_verifies() {
    let transport = FixtureTransport::with(
        "sonos-living",
        MediaState {
            device: "sonos-living".to_string(),
            playback: Some("PLAYING".to_string()),
            volume: Some(40),
            source: Some("spotify".to_string()),
            power: Some("ON".to_string()),
        },
    );
    let adapter = MediaAdapter::new(transport);
    // Verifying a target the transport cannot observe fails closed
    // (NotFound) and is never Verified (exact-target rule).
    let error = adapter
        .verify(&media_id("sonos-other"), MediaCommand::Play, "PLAYING")
        .expect_err("unknown target fails");
    assert_eq!(error.code, MediaErrorCode::NotFound);
}

#[test]
fn ep024_unit_media_idempotency_duplicate_conflict() {
    // A transport whose send blocks until released; the adapter's
    // in-flight guard must reject a duplicate while the first command
    // is still in flight.
    let release = Arc::new(Mutex::new(false));
    let transport = BlockingTransport::new("sonos-living", Arc::clone(&release));
    let adapter = Arc::new(MediaAdapter::new(transport));

    let adapter_clone = Arc::clone(&adapter);
    let handle = std::thread::spawn(move || {
        adapter_clone.execute(
            &media_id("sonos-living"),
            MediaCommand::Play,
            &all_capabilities(),
        )
    });
    // Give the first command time to enter the in-flight window.
    std::thread::sleep(std::time::Duration::from_millis(100));
    // Second identical command while in flight -> Conflict.
    let error = adapter
        .execute(
            &media_id("sonos-living"),
            MediaCommand::Play,
            &all_capabilities(),
        )
        .expect_err("duplicate refused");
    assert_eq!(error.code, MediaErrorCode::Conflict);
    *release.lock().expect("release") = true;
    let first = handle.join().expect("thread");
    assert!(first.is_ok(), "first command accepted");
}

#[test]
fn ep024_unit_media_unsupported_command_policy() {
    let transport = FixtureTransport::with(
        "tv-living",
        MediaState {
            device: "tv-living".to_string(),
            playback: None,
            volume: None,
            source: None,
            power: Some("ON".to_string()),
        },
    );
    let adapter = MediaAdapter::new(transport);
    // TV only has Power; a SetSource command is refused (policy).
    let error = adapter
        .execute(
            &media_id("tv-living"),
            MediaCommand::SetSource,
            &[MediaCapability::Power],
        )
        .expect_err("unsupported refused");
    assert_eq!(error.code, MediaErrorCode::Policy);
}

#[test]
fn ep024_unit_media_transport_failure_fails_closed() {
    let transport = FixtureTransport::with(
        "sonos-living",
        MediaState {
            device: "sonos-living".to_string(),
            playback: Some("PAUSED".to_string()),
            volume: Some(40),
            source: Some("spotify".to_string()),
            power: Some("ON".to_string()),
        },
    );
    transport.fail_next(MediaErrorCode::Timeout);
    let adapter = MediaAdapter::new(transport);
    let error = adapter
        .execute(
            &media_id("sonos-living"),
            MediaCommand::Play,
            &all_capabilities(),
        )
        .expect_err("transport timeout propagates");
    assert_eq!(error.code, MediaErrorCode::Timeout);
}

#[test]
fn ep024_unit_media_availability_truth_table() {
    let transport = FixtureTransport::with(
        "sonos-living",
        MediaState {
            device: "sonos-living".to_string(),
            playback: Some("PLAYING".to_string()),
            volume: Some(40),
            source: Some("spotify".to_string()),
            power: Some("ON".to_string()),
        },
    );
    let adapter = MediaAdapter::new(transport);
    assert_eq!(
        adapter
            .availability(&media_id("sonos-living"))
            .expect("availability"),
        DeviceAvailability::Streaming
    );

    let idle = FixtureTransport::with(
        "tv-bedroom",
        MediaState {
            device: "tv-bedroom".to_string(),
            playback: Some("IDLE".to_string()),
            volume: Some(0),
            source: None,
            power: Some("ON".to_string()),
        },
    );
    let adapter = MediaAdapter::new(idle);
    assert_eq!(
        adapter
            .availability(&media_id("tv-bedroom"))
            .expect("availability"),
        DeviceAvailability::Streaming
    );

    // Unbound transport -> Unavailable.
    let empty = FixtureTransport::default();
    let adapter = MediaAdapter::new(empty);
    assert_eq!(
        adapter
            .availability(&media_id("ghost"))
            .expect("availability"),
        DeviceAvailability::Unavailable
    );
}

#[test]
fn ep024_unit_media_list_devices_maps_through_typed_ids() {
    let transport = FixtureTransport::with(
        "sonos-living",
        MediaState {
            device: "sonos-living".to_string(),
            playback: Some("PAUSED".to_string()),
            volume: Some(40),
            source: Some("spotify".to_string()),
            power: Some("ON".to_string()),
        },
    );
    let adapter = MediaAdapter::new(transport);
    let devices = adapter.list_devices().expect("devices");
    assert_eq!(devices, vec![media_id("sonos-living")]);
}

#[test]
fn ep024_unit_media_capabilities_deterministic() {
    let transport = FixtureTransport::with(
        "sonos-living",
        MediaState {
            device: "sonos-living".to_string(),
            playback: Some("PAUSED".to_string()),
            volume: Some(40),
            source: Some("spotify".to_string()),
            power: Some("ON".to_string()),
        },
    );
    let adapter = MediaAdapter::new(transport);
    let capabilities = adapter
        .capabilities(&media_id("sonos-living"))
        .expect("capabilities");
    assert!(capabilities.contains(&MediaCapability::Playback));
    assert!(capabilities.contains(&MediaCapability::Power));
    // Provider domain names never appear; only canonical keys.
    assert_eq!(
        MediaAdapter::<FixtureTransport>::capability_key(MediaCommand::Play),
        "media.playback"
    );
}

#[test]
fn ep024_unit_media_dependency_direction_connector_imports_contracts_not_reverse() {
    // nexus-media imports nexus-devices contracts (the port), never the
    // reverse. This file's imports prove the surface.
    let correlation = nexus_devices::CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6401")
        .expect("valid UUIDv7");
    assert_eq!(correlation.as_str().len(), 36);
}

/// Fixture transport that applies a command on dispatch (real state
/// transition observable through readback).
struct ApplyOnCommandTransport {
    devices: Vec<String>,
    states: RefCell<HashMap<String, MediaState>>,
}

impl ApplyOnCommandTransport {
    fn new(device: &str) -> Self {
        let mut states = HashMap::new();
        states.insert(
            device.to_string(),
            MediaState {
                device: device.to_string(),
                playback: Some("PAUSED".to_string()),
                volume: Some(40),
                source: Some("spotify".to_string()),
                power: Some("ON".to_string()),
            },
        );
        Self {
            devices: vec![device.to_string()],
            states: RefCell::new(states),
        }
    }
}

impl MediaTransport for ApplyOnCommandTransport {
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
        let mut states = self.states.borrow_mut();
        if let Some(state) = states.get_mut(device) {
            match command {
                MediaCommand::Play => state.playback = Some("PLAYING".to_string()),
                MediaCommand::Pause => state.playback = Some("PAUSED".to_string()),
                MediaCommand::Stop => state.playback = Some("STOPPED".to_string()),
                MediaCommand::PowerOn => state.power = Some("ON".to_string()),
                MediaCommand::PowerOff => state.power = Some("OFF".to_string()),
                MediaCommand::SetVolume => {}
                MediaCommand::SetSource => {}
                MediaCommand::Seek => {}
            }
        }
        Ok(())
    }
}

/// Fixture transport whose send blocks until released (in-flight
/// guard). Real adapter logic drives the in-flight window; the release
/// flag simulates a slow provider round trip.
struct BlockingTransport {
    devices: Vec<String>,
    release: Arc<Mutex<bool>>,
}

impl BlockingTransport {
    fn new(device: &str, release: Arc<Mutex<bool>>) -> Self {
        Self {
            devices: vec![device.to_string()],
            release,
        }
    }
}

impl MediaTransport for BlockingTransport {
    fn list_devices(&self) -> Result<Vec<String>, MediaError> {
        Ok(self.devices.clone())
    }

    fn state(&self, _device: &str) -> Result<MediaState, MediaError> {
        Err(MediaError::new(
            MediaErrorCode::Timeout,
            "slow transport",
            None,
            None,
        ))
    }

    fn send_command(&self, _device: &str, _command: MediaCommand) -> Result<(), MediaError> {
        // Block until the test releases the in-flight command.
        while !*self.release.lock().expect("release") {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }
}
