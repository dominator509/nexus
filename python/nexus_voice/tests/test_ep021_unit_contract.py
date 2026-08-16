"""EP-021 M1 contract tests: construction, validation, serialization,
vocabulary rejection, and dependency direction (SPEC-012; ADR).

Run with: uv run --frozen pytest python/nexus_voice/tests -q -k ep021_unit
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_ROOT = REPO_ROOT / "python"
if str(PYTHON_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))

import pytest  # noqa: E402
from nexus_voice import (  # noqa: E402
    AudioEndpointKind,
    AudioFormat,
    AudioFrame,
    AudioPrivacyPolicy,
    PrivacyState,
    PrivacyZone,
    SessionState,
    SpeakerEvidence,
    SpeakerVerdict,
    SttResult,
    TtsResult,
    VadDecision,
    VadResult,
    VoiceCapability,
    VoiceError,
    VoiceErrorCode,
    VoiceSession,
    WakeWordResult,
    WakeWordState,
    require_capability,
    require_endpoint_kind,
    require_privacy_state,
)

# ---------------------------------------------------------------------------
# Construction
# ---------------------------------------------------------------------------


def ep021_unit_audio_frame_constructs() -> None:
    frame = AudioFrame(
        format=AudioFormat.PcmS16LE,
        sample_rate_hz=16000,
        channels=1,
        data=b"\x00\x00" * 160,
        endpoint_kind=AudioEndpointKind.X86Linux,
        correlation_id="0190e1c4-0000-7000-8000-000000000001",
        sequence=7,
    )
    assert frame.format == "PCM_S16LE"
    assert frame.sample_rate_hz == 16000
    assert frame.channels == 1
    assert frame.sequence == 7
    assert frame.duration_seconds() == pytest.approx(0.01)


def ep021_unit_audio_frame_rejects_unknown_format() -> None:
    with pytest.raises(ValueError):
        AudioFrame(format="MP4", sample_rate_hz=16000, channels=1, data=b"x")


def ep021_unit_audio_frame_rejects_unknown_endpoint() -> None:
    with pytest.raises(ValueError):
        AudioFrame(
            format=AudioFormat.PcmS16LE,
            sample_rate_hz=16000,
            channels=1,
            data=b"x",
            endpoint_kind="SMART_FRIDGE",
        )


def ep021_unit_audio_frame_rejects_nonpositive_rate() -> None:
    with pytest.raises(ValueError):
        AudioFrame(format=AudioFormat.PcmS16LE, sample_rate_hz=0, channels=1, data=b"x")


def ep021_unit_audio_frame_serialization_roundtrip() -> None:
    frame = AudioFrame(
        format=AudioFormat.PcmF32LE,
        sample_rate_hz=48000,
        channels=2,
        data=b"\x00\x00\x00\x00" * 96,
        endpoint_kind=AudioEndpointKind.Pi5,
        correlation_id="0190e1c4-0000-7000-8000-000000000002",
        sequence=3,
    )
    payload = frame.to_dict()
    assert payload["schema"] == "nexus.voice.audio_frame.v1"
    assert "payload_bytes" in payload
    assert "data" not in payload  # never serialize raw audio
    restored = AudioFrame.from_dict(payload)
    assert restored.format == frame.format
    assert restored.sample_rate_hz == frame.sample_rate_hz
    assert restored.channels == frame.channels
    assert restored.endpoint_kind == frame.endpoint_kind
    assert restored.sequence == frame.sequence


def ep021_unit_audio_frame_rejects_unknown_schema() -> None:
    with pytest.raises(ValueError):
        AudioFrame.from_dict({"schema": "nexus.voice.audio_frame.v9"})


# ---------------------------------------------------------------------------
# Vocabulary rejection
# ---------------------------------------------------------------------------


def ep021_unit_endpoint_kind_rejects_unknown() -> None:
    with pytest.raises(ValueError):
        require_endpoint_kind("TOASTER")
    assert require_endpoint_kind(AudioEndpointKind.AtomEcho) == "ATOM_ECHO"


def ep021_unit_capability_rejects_unknown() -> None:
    with pytest.raises(ValueError):
        require_capability("SMOKE_DETECTOR")
    assert require_capability(VoiceCapability.Aec) == "AEC"


def ep021_unit_privacy_state_rejects_unknown() -> None:
    with pytest.raises(ValueError):
        require_privacy_state("BROADCAST")
    assert require_privacy_state(PrivacyState.HardwareMute) == "HARDWARE_MUTE"


# ---------------------------------------------------------------------------
# VAD / wake / STT / TTS / speaker contracts
# ---------------------------------------------------------------------------


def ep021_unit_vad_result_validates() -> None:
    frame = AudioFrame(AudioFormat.PcmS16LE, 16000, 1, b"\x00" * 32)
    result = VadResult(decision=VadDecision.Speech, confidence=0.9, frame=frame)
    assert result.decision == "SPEECH"
    with pytest.raises(ValueError):
        VadResult(decision="MAYBE", confidence=0.5, frame=frame)
    with pytest.raises(ValueError):
        VadResult(decision=VadDecision.Speech, confidence=1.5, frame=frame)


def ep021_unit_wake_word_result_validates() -> None:
    frame = AudioFrame(AudioFormat.PcmS16LE, 16000, 1, b"\x00" * 32)
    result = WakeWordResult(
        state=WakeWordState.Triggered, word="nexus", confidence=0.95, frame=frame
    )
    assert result.state == "TRIGGERED"
    with pytest.raises(ValueError):
        WakeWordResult(state="TRIGGERED", word=None, confidence=0.5, frame=frame)
    with pytest.raises(ValueError):
        WakeWordResult(state="BEEPING", word="nexus", confidence=0.5, frame=frame)


def ep021_unit_stt_result_validates() -> None:
    result = SttResult(transcript="turn on the light", confidence=0.88, frames=12)
    assert result.transcript == "turn on the light"
    with pytest.raises(ValueError):
        SttResult(transcript="x", confidence=1.2)


def ep021_unit_tts_result_requires_audio() -> None:
    with pytest.raises(ValueError):
        TtsResult(audio=())
    frame = AudioFrame(AudioFormat.PcmS16LE, 16000, 1, b"\x00" * 32)
    result = TtsResult(audio=(frame,))
    assert result.frames == 1


def ep021_unit_speaker_evidence_never_elevates() -> None:
    evidence = SpeakerEvidence(
        verdict=SpeakerVerdict.Match, confidence=0.97, speaker_id="principal-1"
    )
    assert evidence.verdict == "MATCH"
    assert evidence.as_evidence()["schema"] == "nexus.voice.speaker_evidence.v1"
    with pytest.raises(ValueError):
        SpeakerEvidence(verdict="MATCH", confidence=0.5, speaker_id=None)
    unknown = SpeakerEvidence(verdict=SpeakerVerdict.Unknown, confidence=0.3)
    assert unknown.speaker_id is None


# ---------------------------------------------------------------------------
# Session / privacy
# ---------------------------------------------------------------------------


def ep021_unit_session_constructs_and_serializes() -> None:
    session = VoiceSession(
        session_id="0190e1c4-0000-7000-8000-00000000000a",
        principal_id="principal-1",
        endpoint_kind=AudioEndpointKind.Wyoming,
        state=SessionState.Listening,
        objective="control the living room",
        tenant_id="018f0f6f-9c1e-7b6e-8000-000000000001",
        correlation_id="0190e1c4-0000-7000-8000-000000000002",
    )
    session.append_transcript("user", "turn on the light")
    payload = session.to_dict()
    assert payload["schema"] == "nexus.voice.session.v1"
    restored = VoiceSession.from_dict(payload)
    assert restored.session_id == session.session_id
    assert restored.principal_id == session.principal_id
    assert restored.transcript == [("user", "turn on the light")]


def ep021_unit_session_transfer_preserves_context() -> None:
    session = VoiceSession(
        session_id="s1",
        principal_id="p1",
        endpoint_kind=AudioEndpointKind.X86Linux,
        objective="keep objective",
    )
    session.append_transcript("user", "hello")
    session.transfer_to(AudioEndpointKind.IOS)
    assert session.endpoint_kind == "IOS"
    assert session.objective == "keep objective"
    assert session.transcript == [("user", "hello")]
    with pytest.raises(ValueError):
        session.transfer_to("REFRIGERATOR")


def ep021_unit_privacy_policy_ephemeral_default() -> None:
    policy = AudioPrivacyPolicy(policy_id="pol-1")
    assert policy.ephemeral_by_default is True
    assert policy.allow_cloud_streaming is False
    assert policy.retention_seconds == 0
    assert policy.zone == PrivacyZone.Private
    with pytest.raises(ValueError):
        AudioPrivacyPolicy(policy_id="pol-2", ephemeral_by_default=True, retention_seconds=60)


def ep021_unit_privacy_policy_hardware_mute_propagates() -> None:
    policy = AudioPrivacyPolicy(policy_id="pol-1")
    muted = policy.apply_hardware_mute(True)
    assert muted.allow_cloud_streaming is False
    assert muted.retention_seconds == 0
    assert muted.hardware_mute_enforced is True
    # Mute is authoritative from any zone.
    shared = policy.apply_shared_room(True)
    assert shared.zone == PrivacyZone.SharedRoom
    muted_shared = shared.apply_hardware_mute(True)
    assert muted_shared.allow_cloud_streaming is False
    assert muted_shared.retention_seconds == 0
    # Unmute preserves the pre-mute policy.
    assert policy.apply_hardware_mute(False) == policy


def ep021_unit_privacy_policy_shared_room_forbids_cloud() -> None:
    policy = AudioPrivacyPolicy(policy_id="pol-1")
    shared = policy.apply_shared_room(True)
    assert shared.allow_cloud_streaming is False
    assert shared.retention_seconds == 0
    assert shared.ephemeral_by_default is True
    assert shared.zone == PrivacyZone.SharedRoom


def ep021_unit_privacy_policy_serialization_roundtrip() -> None:
    policy = AudioPrivacyPolicy(policy_id="pol-1")
    restored = AudioPrivacyPolicy.from_dict(policy.to_dict())
    assert restored == policy
    with pytest.raises(ValueError):
        AudioPrivacyPolicy.from_dict({"schema": "nexus.voice.privacy_policy.v9"})


# ---------------------------------------------------------------------------
# Typed errors
# ---------------------------------------------------------------------------


def ep021_unit_voice_error_codes_are_canonical() -> None:
    err = VoiceError(VoiceErrorCode.Unavailable, "provider down", correlation_id="c1")
    assert err.code == "UNAVAILABLE"
    assert err.correlation_id == "c1"
    assert err.as_dict() == {
        "code": "UNAVAILABLE",
        "message": "provider down",
        "correlation_id": "c1",
    }
    with pytest.raises(ValueError):
        VoiceError("NOT_A_CODE", "x")


# ---------------------------------------------------------------------------
# Dependency direction: nexus_voice must not import application layers
# ---------------------------------------------------------------------------


def ep021_unit_dependency_direction_no_application_imports() -> None:
    import nexus_voice  # noqa: F401

    app_modules = (
        "nexus_connector_sdk",
        "nexus_contracts",
        "packages",
        "crates",
        "temporal",
    )
    import sys as _sys

    for name in app_modules:
        for mod_name in list(_sys.modules):
            if mod_name == name or mod_name.startswith(name + "."):
                raise AssertionError(f"nexus_voice must not import {name}")
