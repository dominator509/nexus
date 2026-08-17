//! EP-025 canonical telephony vocabulary (SPEC-014 terms are
//! vocabulary locked; a new synonym requires an ADR and schema
//! update).
//!
//! Permanent hierarchy (owner directive, EP-025):
//! CALL REQUESTED != SIP INVITE SENT != REMOTE RINGING != ANSWERED
//! != MEDIA ESTABLISHED != TWO-WAY AUDIO VERIFIED != CALL COMPLETED.
//!
//! SIP SIGNALING IS NOT MEDIA CERTIFICATION: a 200/ANSWER proves
//! signaling, never audio. Two-way means TWO directions.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{CallError, CallErrorCode};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, CallError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(CallError::new(
                        CallErrorCode::Validation,
                        concat!(stringify!($name), " must be 1..=128 characters"),
                        None,
                        None,
                    ));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(CallSessionId);
typed_id!(CallLegId);
typed_id!(SipEndpointId);
typed_id!(CarrierId);
typed_id!(TranscriptId);

/// Call direction (SPEC-014 CallSession).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallDirection {
    Inbound,
    Outbound,
}

impl CallDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inbound => "INBOUND",
            Self::Outbound => "OUTBOUND",
        }
    }

    pub fn parse(text: &str) -> Result<Self, CallError> {
        match text {
            "INBOUND" => Ok(Self::Inbound),
            "OUTBOUND" => Ok(Self::Outbound),
            _ => Err(CallError::vocabulary(format!(
                "unknown call direction {text:?}"
            ))),
        }
    }
}

/// Canonical call state ladder.
///
/// Each rung is a distinct observed truth. A state NEVER advances
/// because a test expects it: it advances only from real Asterisk
/// channel/event evidence (directive 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallState {
    /// CALL REQUESTED: a Nexus call request/session was created.
    Requested,
    /// SIP INVITE SENT (dialing): signaling attempted, no answer.
    InviteSent,
    /// REMOTE RINGING: provisional 180 received.
    Ringing,
    /// ANSWERED: 200 OK received (signaling answer only).
    Answered,
    /// BRIDGED: real Asterisk channel bridge exists.
    Bridged,
    /// MEDIA ESTABLISHED: RTP/media packets flow in both directions.
    MediaEstablished,
    /// TWO-WAY AUDIO VERIFIED: decoded bidirectional audio verified.
    TwoWayAudioVerified,
    /// CALL COMPLETED (hung up cleanly).
    HungUp,
    /// Terminal failure: remote endpoint busy.
    Busy,
    /// Terminal failure: no answer before timeout.
    NoAnswer,
    /// Terminal failure: remote rejected the call.
    Rejected,
    /// Terminal failure: provider/endpoint unavailable.
    Unavailable,
    /// Terminal failure: authentication failed.
    AuthFailed,
    /// Terminal failure: network/transport error.
    NetworkError,
    /// Terminal failure: unspecified/invariant failure.
    Failed,
}

impl CallState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "REQUESTED",
            Self::InviteSent => "INVITE_SENT",
            Self::Ringing => "RINGING",
            Self::Answered => "ANSWERED",
            Self::Bridged => "BRIDGED",
            Self::MediaEstablished => "MEDIA_ESTABLISHED",
            Self::TwoWayAudioVerified => "TWO_WAY_AUDIO_VERIFIED",
            Self::HungUp => "HUNG_UP",
            Self::Busy => "BUSY",
            Self::NoAnswer => "NO_ANSWER",
            Self::Rejected => "REJECTED",
            Self::Unavailable => "UNAVAILABLE",
            Self::AuthFailed => "AUTH_FAILED",
            Self::NetworkError => "NETWORK_ERROR",
            Self::Failed => "FAILED",
        }
    }

    pub fn parse(text: &str) -> Result<Self, CallError> {
        match text {
            "REQUESTED" => Ok(Self::Requested),
            "INVITE_SENT" => Ok(Self::InviteSent),
            "RINGING" => Ok(Self::Ringing),
            "ANSWERED" => Ok(Self::Answered),
            "BRIDGED" => Ok(Self::Bridged),
            "MEDIA_ESTABLISHED" => Ok(Self::MediaEstablished),
            "TWO_WAY_AUDIO_VERIFIED" => Ok(Self::TwoWayAudioVerified),
            "HUNG_UP" => Ok(Self::HungUp),
            "BUSY" => Ok(Self::Busy),
            "NO_ANSWER" => Ok(Self::NoAnswer),
            "REJECTED" => Ok(Self::Rejected),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            "AUTH_FAILED" => Ok(Self::AuthFailed),
            "NETWORK_ERROR" => Ok(Self::NetworkError),
            "FAILED" => Ok(Self::Failed),
            _ => Err(CallError::vocabulary(format!(
                "unknown call state {text:?}"
            ))),
        }
    }

    /// Terminal states never transition to an active state.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::HungUp
                | Self::Busy
                | Self::NoAnswer
                | Self::Rejected
                | Self::Unavailable
                | Self::AuthFailed
                | Self::NetworkError
                | Self::Failed
        )
    }
}

/// Canonical media verification state.
///
/// MEDIA_TRANSPORT_ACTIVE != TWO-WAY MEDIA VERIFIED. A bridge with
/// packets proves transport; decoded bidirectional audio proves
/// two-way media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaState {
    /// No media path established.
    None,
    /// Media transport active (RTP packets observed; signaling/media
    /// routing established).
    TransportActive,
    /// Bidirectional decoded audio verified with real canaries.
    TwoWayAudioVerified,
    /// Media path failed mid-call.
    Failed,
}

impl MediaState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::TransportActive => "TRANSPORT_ACTIVE",
            Self::TwoWayAudioVerified => "TWO_WAY_AUDIO_VERIFIED",
            Self::Failed => "FAILED",
        }
    }

    pub fn parse(text: &str) -> Result<Self, CallError> {
        match text {
            "NONE" => Ok(Self::None),
            "TRANSPORT_ACTIVE" => Ok(Self::TransportActive),
            "TWO_WAY_AUDIO_VERIFIED" => Ok(Self::TwoWayAudioVerified),
            "FAILED" => Ok(Self::Failed),
            _ => Err(CallError::vocabulary(format!(
                "unknown media state {text:?}"
            ))),
        }
    }
}

/// Governed call capabilities (SPEC-014 behavior 4; EP-025 acceptance
/// obligation 3: dial, answer, hangup, transfer, DTMF, hold, and
/// status are governed capabilities).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallCapability {
    Dial,
    Answer,
    Hangup,
    Transfer,
    Dtmf,
    Hold,
    Status,
}

impl CallCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dial => "DIAL",
            Self::Answer => "ANSWER",
            Self::Hangup => "HANGUP",
            Self::Transfer => "TRANSFER",
            Self::Dtmf => "DTMF",
            Self::Hold => "HOLD",
            Self::Status => "STATUS",
        }
    }

    pub fn parse(text: &str) -> Result<Self, CallError> {
        match text {
            "DIAL" => Ok(Self::Dial),
            "ANSWER" => Ok(Self::Answer),
            "HANGUP" => Ok(Self::Hangup),
            "TRANSFER" => Ok(Self::Transfer),
            "DTMF" => Ok(Self::Dtmf),
            "HOLD" => Ok(Self::Hold),
            "STATUS" => Ok(Self::Status),
            _ => Err(CallError::vocabulary(format!(
                "unknown call capability {text:?}"
            ))),
        }
    }
}

/// Canonical call commands (capability-gated dispatch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallCommand {
    Dial,
    Answer,
    Hangup,
    Transfer,
    SendDtmf,
    Hold,
    Resume,
}

impl CallCommand {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dial => "DIAL",
            Self::Answer => "ANSWER",
            Self::Hangup => "HANGUP",
            Self::Transfer => "TRANSFER",
            Self::SendDtmf => "SEND_DTMF",
            Self::Hold => "HOLD",
            Self::Resume => "RESUME",
        }
    }

    pub fn parse(text: &str) -> Result<Self, CallError> {
        match text {
            "DIAL" => Ok(Self::Dial),
            "ANSWER" => Ok(Self::Answer),
            "HANGUP" => Ok(Self::Hangup),
            "TRANSFER" => Ok(Self::Transfer),
            "SEND_DTMF" => Ok(Self::SendDtmf),
            "HOLD" => Ok(Self::Hold),
            "RESUME" => Ok(Self::Resume),
            _ => Err(CallError::vocabulary(format!(
                "unknown call command {text:?}"
            ))),
        }
    }
}

/// Canonical audio codec vocabulary. Only codecs actually exercised
/// may be advertised (directive 13: no wideband claim without a
/// wideband codec path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaCodec {
    Pcmu,
    Pcma,
    G722,
    Opus,
    Slin16,
}

impl MediaCodec {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pcmu => "PCMU",
            Self::Pcma => "PCMA",
            Self::G722 => "G722",
            Self::Opus => "OPUS",
            Self::Slin16 => "SLIN16",
        }
    }

    pub fn parse(text: &str) -> Result<Self, CallError> {
        match text {
            "PCMU" => Ok(Self::Pcmu),
            "PCMA" => Ok(Self::Pcma),
            "G722" => Ok(Self::G722),
            "OPUS" => Ok(Self::Opus),
            "SLIN16" => Ok(Self::Slin16),
            _ => Err(CallError::vocabulary(format!(
                "unknown media codec {text:?}"
            ))),
        }
    }

    /// PCMU/PCMA/G722 are narrowband (8 kHz); OPUS/SLIN16 may be
    /// wideband. A wideband claim requires a wideband-exercised path.
    pub const fn is_wideband(self) -> bool {
        matches!(self, Self::Opus | Self::Slin16)
    }
}

/// Call outcome (durable terminal record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallOutcome {
    Completed,
    Busy,
    NoAnswer,
    Rejected,
    Failed,
    Aborted,
}

impl CallOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "COMPLETED",
            Self::Busy => "BUSY",
            Self::NoAnswer => "NO_ANSWER",
            Self::Rejected => "REJECTED",
            Self::Failed => "FAILED",
            Self::Aborted => "ABORTED",
        }
    }

    pub fn parse(text: &str) -> Result<Self, CallError> {
        match text {
            "COMPLETED" => Ok(Self::Completed),
            "BUSY" => Ok(Self::Busy),
            "NO_ANSWER" => Ok(Self::NoAnswer),
            "REJECTED" => Ok(Self::Rejected),
            "FAILED" => Ok(Self::Failed),
            "ABORTED" => Ok(Self::Aborted),
            _ => Err(CallError::vocabulary(format!(
                "unknown call outcome {text:?}"
            ))),
        }
    }
}

/// Privacy class for calls and transcripts (SPEC-020; recording and
/// AI disclosure follow policy and jurisdiction configuration).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallPrivacyClass {
    Public,
    Private,
    Confidential,
}

impl CallPrivacyClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "PUBLIC",
            Self::Private => "PRIVATE",
            Self::Confidential => "CONFIDENTIAL",
        }
    }

    pub fn parse(text: &str) -> Result<Self, CallError> {
        match text {
            "PUBLIC" => Ok(Self::Public),
            "PRIVATE" => Ok(Self::Private),
            "CONFIDENTIAL" => Ok(Self::Confidential),
            _ => Err(CallError::vocabulary(format!(
                "unknown call privacy class {text:?}"
            ))),
        }
    }
}

/// Disclosure policy: recording and AI disclosure follow policy and
/// jurisdiction configuration (acceptance obligation 4; directive 23).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosurePolicy {
    /// Whether recording is consented for this call context.
    pub recording_consented: bool,
    /// Whether the AI participant must be disclosed to the caller.
    pub ai_disclosure_required: bool,
    /// Jurisdiction policy code (ISO 3166-1 alpha-2 or configured
    /// jurisdiction id).
    pub jurisdiction: String,
    /// Retention bound in seconds (0 = no retention beyond call).
    pub retention_seconds: u64,
}

impl DisclosurePolicy {
    pub fn new(
        recording_consented: bool,
        ai_disclosure_required: bool,
        jurisdiction: impl Into<String>,
        retention_seconds: u64,
    ) -> Result<Self, CallError> {
        let jurisdiction = jurisdiction.into();
        if jurisdiction.is_empty() || jurisdiction.len() > 16 {
            return Err(CallError::validation(
                "jurisdiction must be 1..=16 characters",
            ));
        }
        if retention_seconds > 31_536_000 {
            return Err(CallError::validation(
                "retention_seconds exceeds 1 year bound",
            ));
        }
        Ok(Self {
            recording_consented,
            ai_disclosure_required,
            jurisdiction,
            retention_seconds,
        })
    }
}

/// Governed call policy: capabilities allowed for this principal and
/// context, with bounded cost/duration (SPEC-014 behavior 4; EP-025
/// acceptance obligation 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallPolicy {
    pub allowed_capabilities: Vec<CallCapability>,
    /// Maximum call duration in seconds.
    pub max_duration_seconds: u64,
    /// Maximum cost cap in fractional units (provider-specific).
    pub cost_cap: f64,
    pub disclosure: DisclosurePolicy,
}

impl CallPolicy {
    pub fn allows(&self, capability: CallCapability) -> bool {
        self.allowed_capabilities.contains(&capability)
    }
}

/// Transcript artifact (SPEC-014; SPEC-020 privacy).
///
/// Raw transcripts are classified; evidence should prefer digest and
/// metadata over raw content (directive 24).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptArtifact {
    pub id: TranscriptId,
    pub session_id: CallSessionId,
    pub privacy_class: CallPrivacyClass,
    pub sha256_digest: String,
    pub word_count: u64,
    pub duration_seconds: u64,
    pub retention_seconds: u64,
    pub redacted: bool,
}

impl TranscriptArtifact {
    /// Fixed 8-field record constructor; a builder would add noise
    /// without safety for a versioned artifact shape.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TranscriptId,
        session_id: CallSessionId,
        privacy_class: CallPrivacyClass,
        sha256_digest: impl Into<String>,
        word_count: u64,
        duration_seconds: u64,
        retention_seconds: u64,
        redacted: bool,
    ) -> Result<Self, CallError> {
        let sha256_digest = sha256_digest.into();
        if sha256_digest.is_empty() || sha256_digest.len() != 64 {
            return Err(CallError::validation(
                "sha256_digest must be a 64-char hex digest",
            ));
        }
        Ok(Self {
            id,
            session_id,
            privacy_class,
            sha256_digest,
            word_count,
            duration_seconds,
            retention_seconds,
            redacted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid() -> CallSessionId {
        CallSessionId::new("session/1").unwrap()
    }

    #[test]
    fn ep025_unit_call_state_ladder_order() {
        // The permanent hierarchy must be strictly ordered.
        assert!(CallState::Requested < CallState::InviteSent);
        assert!(CallState::InviteSent < CallState::Ringing);
        assert!(CallState::Ringing < CallState::Answered);
        assert!(CallState::Answered < CallState::Bridged);
        assert!(CallState::Bridged < CallState::MediaEstablished);
        assert!(CallState::MediaEstablished < CallState::TwoWayAudioVerified);
        // Terminal states are terminal.
        assert!(CallState::HungUp.is_terminal());
        assert!(CallState::Busy.is_terminal());
        assert!(CallState::NoAnswer.is_terminal());
        assert!(CallState::Rejected.is_terminal());
        assert!(CallState::Unavailable.is_terminal());
        assert!(CallState::AuthFailed.is_terminal());
        assert!(CallState::NetworkError.is_terminal());
        assert!(CallState::Failed.is_terminal());
        assert!(!CallState::Requested.is_terminal());
        assert!(!CallState::Answered.is_terminal());
    }

    #[test]
    fn ep025_unit_call_state_parse_roundtrip() {
        for state in [
            CallState::Requested,
            CallState::InviteSent,
            CallState::Ringing,
            CallState::Answered,
            CallState::Bridged,
            CallState::MediaEstablished,
            CallState::TwoWayAudioVerified,
            CallState::HungUp,
            CallState::Busy,
            CallState::NoAnswer,
            CallState::Rejected,
            CallState::Unavailable,
            CallState::AuthFailed,
            CallState::NetworkError,
            CallState::Failed,
        ] {
            assert_eq!(CallState::parse(state.as_str()).unwrap(), state);
            let json = serde_json::to_string(&state).unwrap();
            let back: CallState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, state);
        }
    }

    #[test]
    fn ep025_unit_call_state_rejects_unknown() {
        let err = CallState::parse("MAGIC_ANSWER").unwrap_err();
        assert_eq!(err.code, CallErrorCode::Vocabulary);
    }

    #[test]
    fn ep025_unit_media_state_semantics() {
        // Transport active is NOT two-way verified.
        assert_ne!(
            MediaState::TransportActive.as_str(),
            MediaState::TwoWayAudioVerified.as_str()
        );
        assert_eq!(
            MediaState::parse("TRANSPORT_ACTIVE").unwrap(),
            MediaState::TransportActive
        );
        assert!(MediaState::parse("NOPE").is_err());
    }

    #[test]
    fn ep025_unit_capability_vocabulary() {
        for cap in [
            CallCapability::Dial,
            CallCapability::Answer,
            CallCapability::Hangup,
            CallCapability::Transfer,
            CallCapability::Dtmf,
            CallCapability::Hold,
            CallCapability::Status,
        ] {
            assert_eq!(CallCapability::parse(cap.as_str()).unwrap(), cap);
        }
        assert!(CallCapability::parse("ROBOCALL").is_err());
    }

    #[test]
    fn ep025_unit_command_maps_to_capability() {
        assert_eq!(CallCommand::SendDtmf.as_str(), "SEND_DTMF");
        assert_eq!(CallCommand::parse("HANGUP").unwrap(), CallCommand::Hangup);
        assert!(CallCommand::parse("SPAM").is_err());
    }

    #[test]
    fn ep025_unit_codec_vocabulary() {
        assert!(!MediaCodec::Pcmu.is_wideband());
        assert!(MediaCodec::Opus.is_wideband());
        assert_eq!(MediaCodec::parse("G722").unwrap(), MediaCodec::G722);
        assert!(MediaCodec::parse("AAC").is_err());
    }

    #[test]
    fn ep025_unit_direction_and_outcome() {
        assert_eq!(
            CallDirection::parse("INBOUND").unwrap(),
            CallDirection::Inbound
        );
        assert_eq!(
            CallDirection::parse("OUTBOUND").unwrap(),
            CallDirection::Outbound
        );
        assert!(CallDirection::parse("SIDEWAYS").is_err());
        assert_eq!(
            CallOutcome::parse("COMPLETED").unwrap(),
            CallOutcome::Completed
        );
        assert!(CallOutcome::parse("MAYBE").is_err());
    }

    #[test]
    fn ep025_unit_typed_ids() {
        assert_eq!(sid().as_str(), "session/1");
        assert!(CallSessionId::new("").is_err());
        assert!(CallLegId::new("x").is_ok());
        assert!(SipEndpointId::new("endpoint-a").is_ok());
        assert!(CarrierId::new("carrier-1").is_ok());
        assert!(TranscriptId::new("t-1").is_ok());
    }

    #[test]
    fn ep025_unit_disclosure_policy_validation() {
        let ok = DisclosurePolicy::new(true, true, "US", 3600).unwrap();
        assert!(ok.recording_consented);
        assert!(ok.ai_disclosure_required);
        assert_eq!(ok.jurisdiction, "US");
        assert!(DisclosurePolicy::new(false, true, "", 60).is_err());
        assert!(DisclosurePolicy::new(false, true, "US", 99_999_999).is_err());
    }

    #[test]
    fn ep025_unit_call_policy_gating() {
        let policy = CallPolicy {
            allowed_capabilities: vec![CallCapability::Dial, CallCapability::Hangup],
            max_duration_seconds: 300,
            cost_cap: 1.0,
            disclosure: DisclosurePolicy::new(false, true, "US", 0).unwrap(),
        };
        assert!(policy.allows(CallCapability::Dial));
        assert!(!policy.allows(CallCapability::Transfer));
        assert!(!policy.allows(CallCapability::Dtmf));
    }

    #[test]
    fn ep025_unit_transcript_artifact_validation() {
        let digest = "a".repeat(64);
        let ok = TranscriptArtifact::new(
            TranscriptId::new("t-1").unwrap(),
            sid(),
            CallPrivacyClass::Private,
            digest.clone(),
            120,
            45,
            3600,
            true,
        )
        .unwrap();
        assert!(ok.redacted);
        assert_eq!(ok.sha256_digest, digest);
        assert!(TranscriptArtifact::new(
            TranscriptId::new("t-2").unwrap(),
            sid(),
            CallPrivacyClass::Private,
            "short",
            0,
            0,
            0,
            false,
        )
        .is_err());
    }

    #[test]
    fn ep025_unit_privacy_class() {
        assert_eq!(
            CallPrivacyClass::parse("CONFIDENTIAL").unwrap(),
            CallPrivacyClass::Confidential
        );
        assert!(CallPrivacyClass::parse("TOP_SECRET").is_err());
    }
}
