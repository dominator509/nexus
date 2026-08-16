"""Nexus voice core - Python binding (SPEC-012; EP-021).

Provider-neutral contracts for the local-first voice pipeline: audio
frames, VAD, wake word, STT, TTS, speaker evidence, voice sessions, and
audio privacy policy. This package is the contract surface only; it
never records or transmits raw room audio and never elevates voice or
speaker evidence to authority (INV-003: voice is evidence, not
cryptographic authentication).

The local defaults are Silero VAD, custom commercially compatible
openWakeWord models, whisper.cpp STT, and Kokoro TTS (SPEC-012 required
behavior 1). Cloud fallbacks (Deepgram/OpenAI STT, ElevenLabs/Azure
TTS) fit the same provider-neutral contracts and are never certified
operational until real provider evidence exists (SPEC-012 required
behavior 2; Reality rule).
"""

from __future__ import annotations

from .audio import AudioFormat, AudioFrame
from .error import VoiceError, VoiceErrorCode
from .privacy import AudioPrivacyPolicy, PrivacyZone
from .session import SessionState, VoiceSession
from .speaker import SpeakerEvidence, SpeakerEvidenceProvider, SpeakerVerdict
from .stt import SttProvider, SttResult
from .tts import TtsProvider, TtsResult
from .vad import VadDecision, VadProvider, VadResult
from .vocabulary import (
    VOICE_VOCABULARY,
    AecState,
    AudioEndpointKind,
    PrivacyState,
    VoiceCapability,
    WakeWordState,
    require_aec_state,
    require_capability,
    require_endpoint_kind,
    require_privacy_state,
    require_wake_word_state,
)
from .wake import WakeWordProvider, WakeWordResult

__all__ = [
    "AudioEndpointKind",
    "AudioFormat",
    "AudioFrame",
    "AudioPrivacyPolicy",
    "AecState",
    "PrivacyState",
    "PrivacyZone",
    "SessionState",
    "SpeakerEvidence",
    "SpeakerEvidenceProvider",
    "SpeakerVerdict",
    "SttProvider",
    "SttResult",
    "TtsProvider",
    "TtsResult",
    "VadDecision",
    "VadProvider",
    "VadResult",
    "VoiceCapability",
    "VoiceError",
    "VoiceErrorCode",
    "VoiceSession",
    "VOICE_VOCABULARY",
    "WakeWordProvider",
    "WakeWordResult",
    "WakeWordState",
    "require_aec_state",
    "require_capability",
    "require_endpoint_kind",
    "require_privacy_state",
    "require_wake_word_state",
]
