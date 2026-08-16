"""EP-021 voice vocabulary (SPEC-012 canonical terms; ADR required for new names).

Vocabulary-locked constants for the voice core. Canonical terms from
SPEC-012: AudioEndpoint, VoiceSession, VAD, WakeWord, STTProvider,
TTSProvider, SpeakerEvidence, Diarization, AEC, Wyoming, Assist
Satellite. Unknown values are rejected at parse time; wire values are
canonical SCREAMING_SNAKE strings so the Python binding matches the
Rust and TypeScript surfaces exactly (EP-011 SDK precedent).
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Canonical voice vocabulary (SPEC-012). Unknown values must be rejected.
# ---------------------------------------------------------------------------

VOICE_VOCABULARY = (
    # Endpoint kinds (SPEC-012 required behavior 6 top-ten satellite matrix).
    "VOICE_PREVIEW",
    "ESP32_S3_BOX_3",
    "ATOM_ECHO",
    "ESP32_S3_I2S",
    "PI_5",
    "PI_4",
    "PI_ZERO_2_W",
    "X86_LINUX",
    "ANDROID",
    "IOS",
    "WYOMING",
    "ASSIST_SATELLITE",
    # Pipeline capabilities (SPEC-012 required behaviors 1-3).
    "VAD",
    "WAKE_WORD",
    "STT",
    "TTS",
    "SPEAKER_EVIDENCE",
    "DIARIZATION",
    "AEC",
    "DENOISE",
    "STREAMING_STT",
    "STREAMING_TTS",
    "INTERRUPTION",
    "ENDPOINT_TRANSFER",
    # Privacy states (SPEC-012 required behaviors 4, 9).
    "EPHEMERAL",
    "HARDWARE_MUTE",
    "SOFTWARE_MUTE",
    "SHARED_ROOM",
    "PRIVATE",
)

# Vocabulary-locked enums as canonical wire strings.
AUDIO_ENDPOINT_KINDS = (
    "VOICE_PREVIEW",
    "ESP32_S3_BOX_3",
    "ATOM_ECHO",
    "ESP32_S3_I2S",
    "PI_5",
    "PI_4",
    "PI_ZERO_2_W",
    "X86_LINUX",
    "ANDROID",
    "IOS",
    "WYOMING",
    "ASSIST_SATELLITE",
)

VOICE_CAPABILITIES = (
    "VAD",
    "WAKE_WORD",
    "STT",
    "TTS",
    "SPEAKER_EVIDENCE",
    "DIARIZATION",
    "AEC",
    "DENOISE",
    "STREAMING_STT",
    "STREAMING_TTS",
    "INTERRUPTION",
    "ENDPOINT_TRANSFER",
)

AEC_STATES = ("DISABLED", "ENABLED", "UNCERTIFIED")
WAKE_WORD_STATES = ("ARMED", "TRIGGERED", "DISARMED", "UNCERTIFIED")
PRIVACY_STATES = ("EPHEMERAL", "HARDWARE_MUTE", "SOFTWARE_MUTE", "SHARED_ROOM", "PRIVATE")


def _require_member(value: str, members: tuple[str, ...], name: str) -> str:
    if value not in members:
        raise ValueError(f"unknown {name} value: {value}")
    return value


class AudioEndpointKind:
    """Audio endpoint kind (SPEC-012 required behavior 6).

    Values are the top-ten satellite matrix plus Wyoming and Assist
    Satellite canonical terms. Unknown values are rejected.
    """

    VoicePreview = "VOICE_PREVIEW"
    Esp32S3Box3 = "ESP32_S3_BOX_3"
    AtomEcho = "ATOM_ECHO"
    Esp32S3I2S = "ESP32_S3_I2S"
    Pi5 = "PI_5"
    Pi4 = "PI_4"
    PiZero2W = "PI_ZERO_2_W"
    X86Linux = "X86_LINUX"
    Android = "ANDROID"
    IOS = "IOS"
    Wyoming = "WYOMING"
    AssistSatellite = "ASSIST_SATELLITE"


class VoiceCapability:
    """Voice pipeline capability (SPEC-012 required behaviors 1-3)."""

    Vad = "VAD"
    WakeWord = "WAKE_WORD"
    Stt = "STT"
    Tts = "TTS"
    SpeakerEvidence = "SPEAKER_EVIDENCE"
    Diarization = "DIARIZATION"
    Aec = "AEC"
    Denoise = "DENOISE"
    StreamingStt = "STREAMING_STT"
    StreamingTts = "STREAMING_TTS"
    Interruption = "INTERRUPTION"
    EndpointTransfer = "ENDPOINT_TRANSFER"


class AecState:
    """Acoustic echo cancellation certification state (SPEC-012)."""

    Disabled = "DISABLED"
    Enabled = "ENABLED"
    Uncertified = "UNCERTIFIED"


class WakeWordState:
    """Wake word machine state (SPEC-012 required behavior 3)."""

    Armed = "ARMED"
    Triggered = "TRIGGERED"
    Disarmed = "DISARMED"
    Uncertified = "UNCERTIFIED"


class PrivacyState:
    """Room/privacy state (SPEC-012 required behaviors 4, 9)."""

    Ephemeral = "EPHEMERAL"
    HardwareMute = "HARDWARE_MUTE"
    SoftwareMute = "SOFTWARE_MUTE"
    SharedRoom = "SHARED_ROOM"
    Private = "PRIVATE"


def require_endpoint_kind(value: str) -> str:
    """Validate an endpoint kind; unknown values raise ValueError."""
    return _require_member(value, AUDIO_ENDPOINT_KINDS, "endpoint kind")


def require_capability(value: str) -> str:
    """Validate a voice capability; unknown values raise ValueError."""
    return _require_member(value, VOICE_CAPABILITIES, "capability")


def require_aec_state(value: str) -> str:
    """Validate an AEC state; unknown values raise ValueError."""
    return _require_member(value, AEC_STATES, "aec state")


def require_wake_word_state(value: str) -> str:
    """Validate a wake word state; unknown values raise ValueError."""
    return _require_member(value, WAKE_WORD_STATES, "wake word state")


def require_privacy_state(value: str) -> str:
    """Validate a privacy state; unknown values raise ValueError."""
    return _require_member(value, PRIVACY_STATES, "privacy state")
