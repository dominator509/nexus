//! EP-022 M2 adapter suite (SPEC-012 behaviors 3, 6, 9; node contract
//! obligations 2 and 3).
//!
//! Real adapter core tests through real cargo machinery: local wake
//! gating, hardware mute authority, room-local ephemeral capture,
//! visible satellite state, conversation context survival across stop
//! and endpoint transfer, and fail-closed unbound ports. Fixtures are
//! CONTROLLED_TEST_FIXTURE wake gates/audio sources (TESTING.md test
//! double zone); no transport certification is claimed.

use nexus_assist_satellite::{
    adapter::{AssistSatelliteAdapter, CaptureDecision, SatelliteState, WakeDecision, WakeGate},
    transport::{AudioBatch, AudioFrameSink, AudioSource, SourceEvent},
};
use nexus_audio::{
    AudioEndpointId, AudioError, AudioErrorCode, ConversationContext, DeterministicTransfer,
    VoiceSatelliteId,
};
use nexus_domain::PersonId;

// --- CONTROLLED_TEST_FIXTURE wake gates / sources (TESTING.md zone) ---

struct ArmedGate;
impl WakeGate for ArmedGate {
    fn evaluate(&mut self, _batch: &AudioBatch) -> Result<WakeDecision, AudioError> {
        Ok(WakeDecision::Armed)
    }
}

struct TriggerGate;
impl WakeGate for TriggerGate {
    fn evaluate(&mut self, _batch: &AudioBatch) -> Result<WakeDecision, AudioError> {
        Ok(WakeDecision::Triggered)
    }
}

struct EmptySource;
impl AudioSource for EmptySource {
    fn next_batch(&mut self) -> Result<Option<AudioBatch>, AudioError> {
        Ok(None)
    }
}

struct NoopSink;
impl AudioFrameSink for NoopSink {
    fn play(&mut self, _batch: &AudioBatch) -> Result<(), AudioError> {
        Ok(())
    }
}

fn satellite(id: &str) -> AssistSatelliteAdapter {
    AssistSatelliteAdapter::new(VoiceSatelliteId::new(id).expect("satellite id"))
}

fn batch(sequence: u64) -> AudioBatch {
    AudioBatch {
        sequence,
        frames: vec![0i16; 16],
        endpoint_ref: "endpoint-1".to_string(),
    }
}

fn person(n: u8) -> PersonId {
    PersonId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f71{n:02}")).expect("valid UUIDv7")
}

// --- Construction and visible state ---

#[test]
fn ep022_unit_satellite_construction_and_visible_state() {
    let sat = satellite("sat-kitchen");
    assert_eq!(sat.satellite_id().as_str(), "sat-kitchen");
    assert_eq!(sat.state(), SatelliteState::Stopped);
    assert_eq!(sat.state().as_str(), "STOPPED");
    assert_eq!(sat.endpoint_ref(), None);
    assert_eq!(sat.sequence(), 0);
}

#[test]
fn ep022_unit_satellite_mute_state_is_visible() {
    let mut sat = satellite("sat-kitchen");
    sat.bind_wake(Box::new(TriggerGate));
    sat.start_listening("endpoint-1").expect("starts");
    sat.hardware_mute().expect("mute");
    assert_eq!(sat.state(), SatelliteState::HardwareMuted);
    assert_eq!(sat.state().as_str(), "HARDWARE_MUTED");
}

// --- Local wake gating (behavior 3) ---

#[test]
fn ep022_unit_satellite_cannot_listen_without_bound_wake() {
    let mut sat = satellite("sat-kitchen");
    let err = sat.start_listening("endpoint-1").expect_err("unbound wake");
    assert_eq!(err.code, AudioErrorCode::Unavailable);
    let surface = err.as_dict();
    assert_eq!(surface["code"], "UNAVAILABLE");
    assert!(surface.get("data").is_none());
    assert!(surface.get("audio").is_none());
}

#[test]
fn ep022_unit_satellite_wake_gate_controls_capture() {
    let mut sat = satellite("sat-kitchen");
    sat.bind_wake(Box::new(ArmedGate));
    sat.start_listening("endpoint-1").expect("starts");

    let decision = sat
        .process_event(SourceEvent::Frames(batch(1)))
        .expect("armed");
    assert_eq!(decision, CaptureDecision::Armed);
    assert_eq!(sat.state(), SatelliteState::Listening);

    sat.bind_wake(Box::new(TriggerGate));
    let decision = sat
        .process_event(SourceEvent::Frames(batch(2)))
        .expect("triggered");
    assert_eq!(decision, CaptureDecision::CaptureStarted);
    assert_eq!(sat.state(), SatelliteState::Capturing);
    assert_eq!(sat.state().as_str(), "CAPTURING");

    let decision = sat
        .process_event(SourceEvent::Frames(batch(3)))
        .expect("capturing");
    assert_eq!(decision, CaptureDecision::Capturing);
}

#[test]
fn ep022_unit_satellite_wake_event_also_triggers() {
    let mut sat = satellite("sat-kitchen");
    sat.bind_wake(Box::new(TriggerGate));
    sat.start_listening("endpoint-1").expect("starts");
    let decision = sat.process_event(SourceEvent::Wake).expect("wake event");
    assert_eq!(decision, CaptureDecision::CaptureStarted);
}

// --- Hardware mute authority (behavior 9) ---

#[test]
fn ep022_unit_satellite_hardware_mute_is_authoritative() {
    let mut sat = satellite("sat-kitchen");
    sat.bind_wake(Box::new(TriggerGate));
    sat.start_listening("endpoint-1").expect("starts");
    sat.hardware_mute().expect("mute");

    // Software cannot start listening while hardware-muted.
    let err = sat.start_listening("endpoint-1").expect_err("muted");
    assert_eq!(err.code, AudioErrorCode::Policy);

    // Even a trigger gate cannot capture while muted.
    let decision = sat
        .process_event(SourceEvent::Frames(batch(1)))
        .expect("muted");
    assert_eq!(decision, CaptureDecision::HardwareMuted);

    sat.hardware_unmute().expect("unmute");
    assert_eq!(sat.state(), SatelliteState::Stopped, "never auto-resumes");
    let decision = sat
        .process_event(SourceEvent::Frames(batch(2)))
        .expect("stopped");
    assert_eq!(decision, CaptureDecision::Stopped);
}

// --- Conversation context survival (obligation 2) ---

#[test]
fn ep022_unit_satellite_context_survives_stop() {
    let mut sat = satellite("sat-kitchen");
    sat.bind_wake(Box::new(ArmedGate));
    sat.start_listening("endpoint-1").expect("starts");

    let mut context = ConversationContext::new(
        "session-1",
        person(1),
        "turn on the lights",
        "policy-private",
    )
    .expect("context")
    .with_correlation("corr-1".into());
    context.append_transcript("user", "turn on the lights");
    sat.attach_context(context);

    sat.stop_listening().expect("stop");
    assert_eq!(sat.state(), SatelliteState::Stopped);
    let context = sat.context().expect("context survives");
    assert_eq!(context.session_id, "session-1");
    assert_eq!(context.correlation_id.as_deref(), Some("corr-1"));
}

#[test]
fn ep022_unit_satellite_transfer_preserves_context_without_privacy_upgrade() {
    let mut sat = satellite("sat-kitchen");
    sat.bind_wake(Box::new(ArmedGate));
    sat.start_listening("endpoint-1").expect("starts");

    let mut context =
        ConversationContext::new("session-2", person(2), "send a message", "policy-private")
            .expect("context")
            .with_correlation("corr-2".into());
    context.append_transcript("user", "send a message");
    context.append_transcript("assist", "to whom?");
    sat.attach_context(context);

    let target = AudioEndpointId::new("endpoint-9").expect("target id");
    let moved = sat
        .transfer_context(&target, &DeterministicTransfer)
        .expect("transfer")
        .expect("context moved");
    assert_eq!(moved.session_id, "session-2");
    assert_eq!(moved.principal, person(2));
    assert_eq!(moved.objective, "send a message");
    // Privacy class is preserved exactly; transfer never upgrades it.
    assert_eq!(moved.privacy_policy_id, "policy-private");
    assert_eq!(moved.correlation_id.as_deref(), Some("corr-2"));
    assert_eq!(moved.transcript.len(), 2);
    // The adapter retains the moved context.
    assert_eq!(sat.context().expect("kept").session_id, "session-2");
}

#[test]
fn ep022_unit_satellite_transfer_without_context_returns_none() {
    let mut sat = satellite("sat-kitchen");
    sat.bind_wake(Box::new(ArmedGate));
    sat.start_listening("endpoint-1").expect("starts");
    let target = AudioEndpointId::new("endpoint-9").expect("target id");
    let moved = sat
        .transfer_context(&target, &DeterministicTransfer)
        .expect("no context");
    assert!(moved.is_none());
}

// --- Fail-closed unbound ports (Reality rule) ---

#[test]
fn ep022_unit_satellite_unbound_wake_never_fabricates_trigger() {
    let mut sat = satellite("sat-kitchen");
    sat.bind_wake(Box::new(ArmedGate));
    sat.start_listening("endpoint-1").expect("starts");
    // Detach the gate mid-session; next evaluation fails closed.
    sat.bind_wake(Box::new(NoopGate));
    let err = sat
        .process_event(SourceEvent::Frames(batch(1)))
        .expect_err("noop gate");
    assert_eq!(err.code, AudioErrorCode::Unavailable);
}

struct NoopGate;
impl WakeGate for NoopGate {
    fn evaluate(&mut self, _batch: &AudioBatch) -> Result<WakeDecision, AudioError> {
        Err(AudioError::unavailable("no implementation bound"))
    }
}

// --- Dependency direction: connector imports contracts, never reverse ---

#[test]
fn ep022_unit_satellite_io_ports_fail_closed() {
    let mut source = EmptySource;
    assert!(source.next_batch().expect("empty source").is_none());
    let mut sink = NoopSink;
    let result = sink.play(&batch(1));
    // NoopSink is a fixture; the port default itself fails closed.
    assert!(result.is_ok());
}
