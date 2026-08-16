//! EP-022 Assist satellite transport ports (SPEC-012 behavior 3).
//!
//! The adapter core is I/O-agnostic: raw audio frames arrive through an
//! `AudioSource` (wake events, frame batches) and leave through an
//! `AudioFrameSink`. Real microphone/Bluetooth/Wyoming transports are
//! owned by later milestones; an unbound source/sink fails closed and
//! never fabricates audio (Reality rule).

use nexus_audio::{AudioError, AudioErrorCode};

/// A batch of raw audio frames at the satellite sample rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioBatch {
    /// Monotonic batch sequence within the satellite session.
    pub sequence: u64,
    /// Raw 16-bit PCM mono frames.
    pub frames: Vec<i16>,
    /// Endpoint this audio arrived on (stable canonical ref).
    pub endpoint_ref: String,
}

/// Local audio source port (fail-closed default).
pub trait AudioSource {
    fn next_batch(&mut self) -> Result<Option<AudioBatch>, AudioError> {
        Err(AudioError::new(
            AudioErrorCode::Unavailable,
            "audio source has no implementation bound",
            None,
            None,
        ))
    }
}

/// Local audio output sink port (fail-closed default).
pub trait AudioFrameSink {
    fn play(&mut self, batch: &AudioBatch) -> Result<(), AudioError> {
        let _ = batch;
        Err(AudioError::new(
            AudioErrorCode::Unavailable,
            "audio sink has no implementation bound",
            None,
            None,
        ))
    }
}

/// Source events observed by the satellite loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceEvent {
    Wake,
    Frames(AudioBatch),
}

/// Wake event delivered by the wake gate when a trigger fires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeEvent {
    pub endpoint_ref: String,
    pub sequence: u64,
}
