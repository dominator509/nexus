//! EP-022 Assist satellite adapter core (SPEC-012 behaviors 3, 6, 9).
//!
//! Real production adapter behavior behind the `AssistSatelliteProvider`
//! port family: local wake gating, hardware mute authority, room-local
//! ephemeral capture, visible satellite state, and conversation context
//! survival across reconnect and endpoint transfer.
//!
//! Permanent invariants (Reality rule, SPEC-012):
//!
//! - Raw room audio is ephemeral by default and never continuously
//!   streamed to cloud (behavior 4).
//! - Hardware mute is authoritative: a fixed microphone with hardware
//!   mute cannot be captured from software (behavior 9).
//! - A satellite is only locally functional when its wake gate is
//!   actually bound; an unbound gate fails closed (UNAVAILABLE) and
//!   never fabricates a trigger (Reality rule).
//! - Conversation context (principal, objective, privacy policy,
//!   transcript, correlation) survives reconnect and endpoint transfer
//!   without implicit privacy upgrades.
//!
//! The adapter is I/O-agnostic: audio arrives through `AudioSource` and
//! leaves through `AudioFrameSink`. Real microphone/Bluetooth/Wyoming
//! transports are owned by M3/M4/M5; no transport certification is
//! claimed here.

use nexus_audio::{
    AudioEndpointId, AudioError, AudioErrorCode, ConversationContext, ConversationTransfer,
    VoiceSatelliteId,
};

use crate::transport::{AudioBatch, AudioFrameSink, AudioSource, SourceEvent};

/// Canonical satellite state (visible to operators; SPEC-012 behavior
/// 9 requires visible mute state).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SatelliteState {
    Stopped,
    Listening,
    Capturing,
    HardwareMuted,
}

impl SatelliteState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "STOPPED",
            Self::Listening => "LISTENING",
            Self::Capturing => "CAPTURING",
            Self::HardwareMuted => "HARDWARE_MUTED",
        }
    }
}

/// Wake decision produced by the local wake gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeDecision {
    Armed,
    Triggered,
}

/// Local wake gate port (fail-closed default).
pub trait WakeGate {
    /// Evaluate one frame batch; returns whether the wake word fired.
    /// An unbound gate fails closed and never fabricates a trigger.
    fn evaluate(&mut self, batch: &AudioBatch) -> Result<WakeDecision, AudioError> {
        let _ = batch;
        Err(AudioError::new(
            AudioErrorCode::Unavailable,
            "wake gate has no implementation bound",
            None,
            None,
        ))
    }
}

/// Outcome of processing one source event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureDecision {
    /// Satellite is stopped; the event was ignored.
    Stopped,
    /// Satellite is hardware-muted; the event was ignored.
    HardwareMuted,
    /// Wake gate is bound but did not trigger.
    Armed,
    /// Wake triggered; capture began.
    CaptureStarted,
    /// Capture in progress.
    Capturing,
}

/// A completed room-local capture (transcript context preserved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatelliteCapture {
    pub satellite_id: VoiceSatelliteId,
    pub endpoint_ref: String,
    pub sequence: u64,
    pub context: Option<ConversationContext>,
    /// Captured frames (raw audio is ephemeral; never streamed to cloud).
    pub frames: Vec<i16>,
}

/// Assist satellite adapter core.
pub struct AssistSatelliteAdapter {
    satellite_id: VoiceSatelliteId,
    state: SatelliteState,
    /// Stable canonical identity of the currently bound endpoint.
    endpoint_ref: Option<String>,
    /// Conversation context survives reconnect and transfer.
    context: Option<ConversationContext>,
    /// Local wake gate. An unbound gate fails closed (Reality rule).
    wake: Option<Box<dyn WakeGate>>,
    source: Option<Box<dyn AudioSource>>,
    sink: Option<Box<dyn AudioFrameSink>>,
    sequence: u64,
}

impl std::fmt::Debug for AssistSatelliteAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssistSatelliteAdapter")
            .field("satellite_id", &self.satellite_id)
            .field("state", &self.state)
            .field("endpoint_ref", &self.endpoint_ref)
            .field("wake_bound", &self.wake.is_some())
            .field("source_bound", &self.source.is_some())
            .field("sink_bound", &self.sink.is_some())
            .field("sequence", &self.sequence)
            .finish()
    }
}

impl AssistSatelliteAdapter {
    pub fn new(satellite_id: VoiceSatelliteId) -> Self {
        Self {
            satellite_id,
            state: SatelliteState::Stopped,
            endpoint_ref: None,
            context: None,
            wake: None,
            source: None,
            sink: None,
            sequence: 0,
        }
    }

    pub fn satellite_id(&self) -> &VoiceSatelliteId {
        &self.satellite_id
    }

    pub fn state(&self) -> SatelliteState {
        self.state
    }

    pub fn endpoint_ref(&self) -> Option<&str> {
        self.endpoint_ref.as_deref()
    }

    pub fn context(&self) -> Option<&ConversationContext> {
        self.context.as_ref()
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Bind the local wake gate. Re-binding replaces the gate.
    pub fn bind_wake(&mut self, wake: Box<dyn WakeGate>) {
        self.wake = Some(wake);
    }

    /// Bind the audio source and sink.
    pub fn bind_io(&mut self, source: Box<dyn AudioSource>, sink: Box<dyn AudioFrameSink>) {
        self.source = Some(source);
        self.sink = Some(sink);
    }

    /// Attach conversation context (e.g. after wake + VAD + STT).
    pub fn attach_context(&mut self, context: ConversationContext) {
        self.context = Some(context);
    }

    /// Start listening on a stable endpoint reference. Fails closed if
    /// the satellite is hardware-muted (behavior 9) or the wake gate is
    /// not bound (a satellite without local wake cannot be locally
    /// functional).
    pub fn start_listening(&mut self, endpoint_ref: &str) -> Result<(), AudioError> {
        if self.state == SatelliteState::HardwareMuted {
            return Err(AudioError::new(
                AudioErrorCode::Policy,
                "hardware mute is authoritative; satellite cannot listen",
                None,
                Some(endpoint_ref.to_string().into()),
            ));
        }
        if self.wake.is_none() {
            return Err(AudioError::new(
                AudioErrorCode::Unavailable,
                "wake gate is not bound; satellite is not locally functional",
                None,
                Some(endpoint_ref.to_string().into()),
            ));
        }
        self.endpoint_ref = Some(endpoint_ref.to_string());
        self.state = SatelliteState::Listening;
        Ok(())
    }

    /// Stop listening; capture state and context survive.
    pub fn stop_listening(&mut self) -> Result<(), AudioError> {
        self.state = SatelliteState::Stopped;
        Ok(())
    }

    /// Apply hardware mute. Authoritative: the satellite cannot be
    /// captured from software while muted (behavior 9).
    pub fn hardware_mute(&mut self) -> Result<(), AudioError> {
        self.state = SatelliteState::HardwareMuted;
        Ok(())
    }

    /// Release hardware mute. The satellite returns to Stopped; it
    /// never silently resumes listening (fail closed).
    pub fn hardware_unmute(&mut self) -> Result<(), AudioError> {
        if self.state == SatelliteState::HardwareMuted {
            self.state = SatelliteState::Stopped;
        }
        Ok(())
    }

    /// Process one source event. Deterministic state machine:
    /// stopped -> ignored; muted -> ignored; listening -> wake gate;
    /// wake trigger -> capture started; capturing -> capture continues.
    /// Unbound wake gate fails closed (UNAVAILABLE), never triggers.
    pub fn process_event(&mut self, event: SourceEvent) -> Result<CaptureDecision, AudioError> {
        self.sequence += 1;
        match self.state {
            SatelliteState::Stopped => Ok(CaptureDecision::Stopped),
            SatelliteState::HardwareMuted => Ok(CaptureDecision::HardwareMuted),
            SatelliteState::Listening => {
                let batch = match &event {
                    SourceEvent::Frames(batch) => batch.clone(),
                    SourceEvent::Wake => AudioBatch {
                        sequence: self.sequence,
                        frames: Vec::new(),
                        endpoint_ref: self.endpoint_ref.clone().unwrap_or_default(),
                    },
                };
                let Some(wake) = &mut self.wake else {
                    return Err(AudioError::new(
                        AudioErrorCode::Unavailable,
                        "wake gate is not bound; cannot evaluate audio",
                        None,
                        None,
                    ));
                };
                match wake.evaluate(&batch)? {
                    WakeDecision::Armed => Ok(CaptureDecision::Armed),
                    WakeDecision::Triggered => {
                        self.state = SatelliteState::Capturing;
                        Ok(CaptureDecision::CaptureStarted)
                    }
                }
            }
            SatelliteState::Capturing => Ok(CaptureDecision::Capturing),
        }
    }

    /// Transfer conversation context to another endpoint without
    /// implicit privacy upgrades (node contract obligation 2). The
    /// source context is preserved exactly; only the endpoint binding
    /// moves. Privacy class is never mutated here - the canonical
    /// router decision (nexus-audio) governs privacy.
    pub fn transfer_context(
        &mut self,
        target: &AudioEndpointId,
        transfer: &dyn ConversationTransfer,
    ) -> Result<Option<ConversationContext>, AudioError> {
        let Some(context) = &self.context else {
            return Ok(None);
        };
        let moved = transfer.transfer(context, target)?;
        let moved = Some(moved);
        self.context = moved.clone();
        Ok(moved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn batch(sequence: u64) -> AudioBatch {
        AudioBatch {
            sequence,
            frames: vec![0i16; 16],
            endpoint_ref: "sat-1".to_string(),
        }
    }

    #[test]
    fn adapter_starts_only_with_bound_wake() {
        let mut sat = AssistSatelliteAdapter::new(VoiceSatelliteId::new("sat-1").expect("id"));
        let err = sat.start_listening("endpoint-1").expect_err("unbound wake");
        assert_eq!(err.code, AudioErrorCode::Unavailable);

        sat.bind_wake(Box::new(ArmedGate));
        sat.start_listening("endpoint-1")
            .expect("starts with bound wake");
        assert_eq!(sat.state(), SatelliteState::Listening);
    }

    #[test]
    fn hardware_mute_is_authoritative() {
        let mut sat = AssistSatelliteAdapter::new(VoiceSatelliteId::new("sat-1").expect("id"));
        sat.bind_wake(Box::new(TriggerGate));
        sat.start_listening("endpoint-1").expect("starts");
        sat.hardware_mute().expect("mute");
        assert_eq!(sat.state(), SatelliteState::HardwareMuted);

        // Software cannot listen while hardware-muted.
        let err = sat.start_listening("endpoint-1").expect_err("muted");
        assert_eq!(err.code, AudioErrorCode::Policy);

        // Even a trigger gate cannot capture while muted.
        let decision = sat
            .process_event(SourceEvent::Frames(batch(1)))
            .expect("muted decision");
        assert_eq!(decision, CaptureDecision::HardwareMuted);

        sat.hardware_unmute().expect("unmute");
        assert_eq!(sat.state(), SatelliteState::Stopped, "never auto-resumes");
        let decision = sat
            .process_event(SourceEvent::Frames(batch(2)))
            .expect("stopped decision");
        assert_eq!(decision, CaptureDecision::Stopped);
    }

    #[test]
    fn wake_gate_controls_capture() {
        let mut sat = AssistSatelliteAdapter::new(VoiceSatelliteId::new("sat-1").expect("id"));
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
        let decision = sat
            .process_event(SourceEvent::Frames(batch(3)))
            .expect("capturing");
        assert_eq!(decision, CaptureDecision::Capturing);
    }

    #[test]
    fn unbound_wake_never_fabricates_trigger() {
        let mut sat = AssistSatelliteAdapter::new(VoiceSatelliteId::new("sat-1").expect("id"));
        sat.bind_wake(Box::new(ArmedGate));
        sat.start_listening("endpoint-1").expect("starts");
        // Remove the gate mid-session; the next evaluation fails closed.
        sat.wake = None;
        let err = sat
            .process_event(SourceEvent::Frames(batch(1)))
            .expect_err("unbound gate");
        assert_eq!(err.code, AudioErrorCode::Unavailable);
    }

    #[test]
    fn context_survives_stop_and_transfer() {
        use nexus_audio::DeterministicTransfer;
        use nexus_domain::PersonId;
        let p = PersonId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7010".to_string()).expect("uuid");
        let mut context =
            ConversationContext::new("session-1", p, "turn on the lights", "policy-private")
                .expect("context")
                .with_correlation("corr-1".into());
        context.append_transcript("user", "turn on the lights");

        let mut sat = AssistSatelliteAdapter::new(VoiceSatelliteId::new("sat-1").expect("id"));
        sat.bind_wake(Box::new(ArmedGate));
        sat.start_listening("endpoint-1").expect("starts");
        sat.attach_context(context);
        sat.stop_listening().expect("stop");
        assert_eq!(sat.state(), SatelliteState::Stopped);
        assert!(sat.context().is_some(), "context survives stop");

        let target = AudioEndpointId::new("endpoint-2").expect("id");
        let moved = sat
            .transfer_context(&target, &DeterministicTransfer)
            .expect("transfer")
            .expect("context moved");
        assert_eq!(moved.session_id, "session-1");
        assert_eq!(moved.privacy_policy_id, "policy-private");
        assert_eq!(moved.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(moved.transcript.len(), 1);
    }
}
