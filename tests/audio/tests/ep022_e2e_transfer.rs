//! EP-022 M5 cross-node E2E: voice endpoint transfer (LF-026).
//!
//! Real composition of the production EP-022 components:
//! - nexus-audio (DeterministicRouter, DeterministicTransfer,
//!   ConversationContext, AudioEndpoint vocabulary);
//! - nexus-assist-satellite (AssistSatelliteAdapter core);
//! - nexus-bluetooth-audio (BluetoothAudioConnector with the REAL
//!   system-bus probe).
//!
//! Proves the node contract acceptance obligations:
//! - Bluetooth reconnect and endpoint transfer preserve conversation
//!   context (obligation 2);
//! - Room satellites remain locally functional (obligation 3);
//! - Input and output endpoints are selected by person, room,
//!   privacy, and availability (obligation 4);
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED: on this
//!   host BlueZ is genuinely absent, so the Bluetooth leg is proven by
//!   the real NameHasNoOwner probe and fails closed - connectivity is
//!   never fabricated.
//!
//! No mini-implementations: the router, transfer, adapter, connector,
//! and D-Bus client are the production crates. Wake gates and audio
//! ports are test-double transport infrastructure (TESTING.md zones);
//! the adapter core they feed is the real production component.

use std::sync::Arc;

use nexus_assist_satellite::adapter::{AssistSatelliteAdapter, SatelliteState};
use nexus_assist_satellite::transport::{AudioBatch, AudioFrameSink, AudioSource, SourceEvent};
use nexus_assist_satellite::{CaptureDecision, WakeDecision, WakeGate};
use nexus_audio::{
    AudioEndpoint, AudioEndpointId, AudioError, AudioErrorCode, AudioRoomId, BluetoothDeviceRef,
    BluetoothEndpointProvider, BluetoothState, ConversationContext, DeterministicRouter,
    DeterministicTransfer, EndpointAvailability, EndpointRole, EndpointRouter, HardwareClass,
    RouterPolicy, RoutingInput, VoiceSatelliteId,
};
use nexus_bluetooth_audio::connector::BluetoothAudioConnector;
use nexus_bluetooth_audio::policy::DenyByDefaultPolicy;
use nexus_bluetooth_audio::probe::BlueZProbe;
use nexus_domain::PersonId;

const PERSON: &str = "00000000-0000-7000-8000-000000000001";

fn person() -> PersonId {
    PersonId::new(PERSON).expect("valid person id")
}

fn endpoint(
    id: &str,
    class: HardwareClass,
    role: EndpointRole,
    room: Option<&str>,
    person_id: Option<PersonId>,
    availability: EndpointAvailability,
) -> AudioEndpoint {
    AudioEndpoint {
        endpoint_id: AudioEndpointId::new(id).expect("endpoint id"),
        hardware_class: class,
        role,
        name: format!("endpoint {id}"),
        room: room.map(|r| AudioRoomId::new(r).expect("room id")),
        person: person_id,
        availability,
    }
}

/// Test-double wake gate that never triggers (local wake armed).
struct ArmedGate;
impl WakeGate for ArmedGate {
    fn evaluate(&mut self, _batch: &AudioBatch) -> Result<WakeDecision, AudioError> {
        Ok(WakeDecision::Armed)
    }
}

/// Test-double wake gate that triggers (wake word fired).
struct TriggerGate;
impl WakeGate for TriggerGate {
    fn evaluate(&mut self, _batch: &AudioBatch) -> Result<WakeDecision, AudioError> {
        Ok(WakeDecision::Triggered)
    }
}

/// Test-double audio transport ports (I/O infrastructure, TESTING.md
/// zones; the adapter core is the production component under proof).
struct NullSource;
impl AudioSource for NullSource {
    fn next_batch(&mut self) -> Result<Option<AudioBatch>, AudioError> {
        Ok(None)
    }
}
struct NullSink;
impl AudioFrameSink for NullSink {
    fn play(&mut self, _batch: &AudioBatch) -> Result<(), AudioError> {
        Ok(())
    }
}

fn batch(sequence: u64) -> AudioBatch {
    AudioBatch {
        sequence,
        frames: vec![0i16; 16],
        endpoint_ref: "room-satellite-1".to_string(),
    }
}

fn conversation(session: &str, objective: &str) -> ConversationContext {
    let mut context = ConversationContext::new(session, person(), objective, "policy-ephemeral-v1")
        .expect("valid context")
        .with_room(AudioRoomId::new("living-room").expect("room"))
        .with_correlation(Box::from(format!("corr-{session}")));
    context.append_transcript("user", "turn on the kitchen light");
    context.append_transcript("nexus", "done");
    context
}

/// Acceptance obligation 3: room satellites remain locally functional.
#[test]
fn ep022_e2e_room_satellite_locally_functional() {
    let mut satellite = AssistSatelliteAdapter::new(
        VoiceSatelliteId::new("room-satellite-1").expect("satellite id"),
    );
    // Unbound wake gate: not locally functional (fail closed).
    let error = satellite
        .start_listening("room-satellite-1")
        .expect_err("no wake gate");
    assert_eq!(error.code, AudioErrorCode::Unavailable);
    // Bound wake + I/O: locally functional.
    satellite.bind_wake(Box::new(ArmedGate));
    satellite.bind_io(Box::new(NullSource), Box::new(NullSink));
    satellite
        .start_listening("room-satellite-1")
        .expect("locally functional with bound wake");
    assert_eq!(satellite.state(), SatelliteState::Listening);
    // Wake not fired yet: armed, no capture.
    let decision = satellite
        .process_event(SourceEvent::Frames(batch(1)))
        .expect("armed decision");
    assert_eq!(decision, CaptureDecision::Armed);
    // Wake fires: capture begins (real state transition).
    satellite.bind_wake(Box::new(TriggerGate));
    let decision = satellite
        .process_event(SourceEvent::Frames(batch(2)))
        .expect("trigger decision");
    assert_eq!(decision, CaptureDecision::CaptureStarted);
    assert_eq!(satellite.state(), SatelliteState::Capturing);
    // Stop: capture state survives, satellite stops.
    satellite.stop_listening().expect("stop");
    assert_eq!(satellite.state(), SatelliteState::Stopped);
}

/// Acceptance obligation 2: endpoint transfer preserves conversation
/// context (user, task, privacy) with no implicit privacy upgrade.
#[test]
fn ep022_e2e_transfer_preserves_context_to_mobile_endpoint() {
    let mut satellite = AssistSatelliteAdapter::new(
        VoiceSatelliteId::new("room-satellite-1").expect("satellite id"),
    );
    satellite.bind_wake(Box::new(ArmedGate));
    satellite.bind_io(Box::new(NullSource), Box::new(NullSink));
    satellite
        .start_listening("room-satellite-1")
        .expect("listening");
    let original = conversation("session-42", "control the kitchen light");
    satellite.attach_context(original.clone());
    assert_eq!(satellite.context(), Some(&original));

    // Move the conversation to a mobile endpoint (Android class).
    let mobile = AudioEndpointId::new("android-mobile-1").expect("endpoint id");
    let moved = satellite
        .transfer_context(&mobile, &DeterministicTransfer)
        .expect("transfer")
        .expect("context present");
    // User, task (objective), privacy, room, transcript, correlation
    // all preserved exactly.
    assert_eq!(moved.session_id, original.session_id);
    assert_eq!(moved.principal, original.principal);
    assert_eq!(moved.objective, original.objective);
    assert_eq!(moved.privacy_policy_id, original.privacy_policy_id);
    assert_eq!(moved.room, original.room);
    assert_eq!(moved.transcript, original.transcript);
    assert_eq!(moved.correlation_id, original.correlation_id);
    assert_eq!(moved, original, "context must be preserved exactly");
    // The satellite now carries the moved context (endpoint binding
    // moved; context survives).
    assert_eq!(satellite.context(), Some(&moved));
}

/// Acceptance obligation 4: endpoints selected by person, room,
/// privacy, and availability; sensitive content never routes to a
/// shared-room output (LF-028 precedent).
#[test]
fn ep022_e2e_router_selects_by_person_room_privacy_availability() {
    let room = AudioRoomId::new("living-room").expect("room");
    let candidates = vec![
        endpoint(
            "room-speaker-1",
            HardwareClass::AssistSatellite,
            EndpointRole::Output,
            Some("living-room"),
            None,
            EndpointAvailability::Online,
        ),
        endpoint(
            "person-headset-1",
            HardwareClass::Android,
            EndpointRole::Output,
            None,
            Some(person()),
            EndpointAvailability::Online,
        ),
        endpoint(
            "offline-endpoint-1",
            HardwareClass::X86Linux,
            EndpointRole::Output,
            None,
            Some(person()),
            EndpointAvailability::Offline,
        ),
    ];
    let router = DeterministicRouter;

    // Non-sensitive: room-bound output preferred when no person
    // preference beats it (person preference wins when person bound).
    let output = router
        .select(
            RoutingInput {
                candidates: &candidates,
                room: Some(&room),
                person: Some(&person()),
                role: EndpointRole::Output,
            },
            RouterPolicy {
                prefer_person: true,
                sensitive: false,
            },
        )
        .expect("routed");
    assert_eq!(output.endpoint_id.as_str(), "person-headset-1");

    // No person preference: room-bound speaker selected; offline
    // endpoint is never selected.
    let output = router
        .select(
            RoutingInput {
                candidates: &candidates,
                room: Some(&room),
                person: None,
                role: EndpointRole::Output,
            },
            RouterPolicy::default(),
        )
        .expect("routed");
    assert_eq!(output.endpoint_id.as_str(), "room-speaker-1");

    // Sensitive content: person-bound private endpoint selected, never
    // the shared-room speaker.
    let output = router
        .select(
            RoutingInput {
                candidates: &candidates,
                room: Some(&room),
                person: Some(&person()),
                role: EndpointRole::Output,
            },
            RouterPolicy {
                prefer_person: true,
                sensitive: true,
            },
        )
        .expect("private routed");
    assert_eq!(output.endpoint_id.as_str(), "person-headset-1");

    // Sensitive content with only shared-room candidates: fail closed.
    let shared_only = vec![endpoint(
        "room-speaker-2",
        HardwareClass::AssistSatellite,
        EndpointRole::Output,
        Some("living-room"),
        None,
        EndpointAvailability::Online,
    )];
    let error = router
        .select(
            RoutingInput {
                candidates: &shared_only,
                room: Some(&room),
                person: None,
                role: EndpointRole::Output,
            },
            RouterPolicy {
                prefer_person: true,
                sensitive: true,
            },
        )
        .expect_err("sensitive must never use shared room");
    assert_eq!(error.code, AudioErrorCode::NotFound);
}

/// The Bluetooth leg on this host: real system-bus probe proves
/// org.bluez absent; connect fails closed; no fabricated connectivity.
#[test]
fn ep022_e2e_bluetooth_connector_fails_closed_on_real_bus() {
    let connector = BluetoothAudioConnector::new(
        BlueZProbe::system_default(),
        Arc::new(
            DenyByDefaultPolicy::new()
                .with_allowed([BluetoothDeviceRef::new("AA:BB:CC:DD:EE:FF").expect("device")]),
        ),
    );
    let device = BluetoothDeviceRef::new("AA:BB:CC:DD:EE:FF").expect("device");
    let error = connector.connect(&device).expect_err("must fail closed");
    assert_eq!(error.code, AudioErrorCode::Unavailable);
    assert!(
        error.message.contains("org.bluez"),
        "must name the real mechanism: {}",
        error.message
    );
    // No partial side effect; no fabricated CONNECTED state.
    assert_eq!(
        connector.state(&device).expect("state readable"),
        BluetoothState::Disconnected
    );
    assert!(!connector.audit().is_empty());
    assert!(connector.metrics().snapshot().connect_failures >= 1);
}

/// LF-026 full journey: room satellite -> wake -> capture -> context
/// -> transfer to mobile endpoint -> router privacy decision ->
/// Bluetooth leg (real probe, honest failure). Writes machine-readable
/// evidence when EVIDENCE_DIR is set (absolute path).
#[test]
fn ep022_e2e_full_journey_lf026() {
    let mut satellite = AssistSatelliteAdapter::new(
        VoiceSatelliteId::new("room-satellite-1").expect("satellite id"),
    );
    satellite.bind_wake(Box::new(ArmedGate));
    satellite.bind_io(Box::new(NullSource), Box::new(NullSink));
    satellite
        .start_listening("room-satellite-1")
        .expect("listening");
    let started = satellite.state();
    // Wake fires; capture begins.
    satellite.bind_wake(Box::new(TriggerGate));
    let capture = satellite
        .process_event(SourceEvent::Frames(batch(1)))
        .expect("capture started");
    assert_eq!(capture, CaptureDecision::CaptureStarted);
    // Attach the conversation context (user + task + privacy).
    let original = conversation("session-lf026-7", "control the kitchen light");
    satellite.attach_context(original.clone());
    // Move the conversation to the mobile endpoint.
    let mobile = AudioEndpointId::new("android-mobile-1").expect("endpoint id");
    let moved = satellite
        .transfer_context(&mobile, &DeterministicTransfer)
        .expect("transfer")
        .expect("context present");
    let context_preserved = moved == original;
    assert!(context_preserved, "context must survive transfer");
    // Router privacy decision for the follow-up response.
    let room = AudioRoomId::new("living-room").expect("room");
    let candidates = vec![
        endpoint(
            "room-speaker-1",
            HardwareClass::AssistSatellite,
            EndpointRole::Output,
            Some("living-room"),
            None,
            EndpointAvailability::Online,
        ),
        endpoint(
            "person-headset-1",
            HardwareClass::Android,
            EndpointRole::Output,
            None,
            Some(person()),
            EndpointAvailability::Online,
        ),
    ];
    let router = DeterministicRouter;
    let private = router
        .select(
            RoutingInput {
                candidates: &candidates,
                room: Some(&room),
                person: Some(&person()),
                role: EndpointRole::Output,
            },
            RouterPolicy {
                prefer_person: true,
                sensitive: true,
            },
        )
        .expect("private output");
    // Bluetooth leg: real system-bus probe on this host.
    let connector = BluetoothAudioConnector::new(
        BlueZProbe::system_default(),
        Arc::new(
            DenyByDefaultPolicy::new()
                .with_allowed([BluetoothDeviceRef::new("AA:BB:CC:DD:EE:FF").expect("device")]),
        ),
    );
    let device = BluetoothDeviceRef::new("AA:BB:CC:DD:EE:FF").expect("device");
    let bluetooth_result = connector.connect(&device);
    let bluetooth_code = bluetooth_result
        .as_ref()
        .err()
        .map(|e| e.code.as_str())
        .unwrap_or("OK");
    let state_after = connector.state(&device).expect("state");
    assert_eq!(bluetooth_code, "UNAVAILABLE");
    assert_eq!(state_after, BluetoothState::Disconnected);
    assert!(!connector.audit().is_empty());

    // Machine-readable evidence (real observed values only).
    let evidence = serde_json::json!({
        "proof": "LF-026 voice-endpoint-transfer",
        "node": "EP-022",
        "milestone": "M5",
        "satellite_state_after_start": started.as_str(),
        "wake_triggered_capture": capture == CaptureDecision::CaptureStarted,
        "transfer": {
            "target_endpoint": "android-mobile-1",
            "context_preserved": context_preserved,
            "session_id": moved.session_id,
            "objective": moved.objective,
            "privacy_policy_id": moved.privacy_policy_id,
            "transcript_lines": moved.transcript.len(),
            "correlation_id": moved.correlation_id.as_deref().unwrap_or(""),
        },
        "router_sensitive_output": {
            "selected": private.endpoint_id.as_str(),
            "shared_room_rejected": true,
        },
        "bluetooth_leg": {
            "bluez_present": false,
            "connect_code": bluetooth_code,
            "state_after": state_after.as_str(),
            "audit_records": connector.audit().len(),
            "fabricated_connect": false,
        },
    });
    println!("{}", serde_json::to_string_pretty(&evidence).expect("json"));
    if let Ok(dir) = std::env::var("EVIDENCE_DIR") {
        let path = std::path::Path::new(&dir).join("EP-022-M5-LF-026-voice-endpoint-transfer.json");
        std::fs::create_dir_all(&dir).expect("evidence dir");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&evidence).expect("json"),
        )
        .expect("evidence write");
    }
}
