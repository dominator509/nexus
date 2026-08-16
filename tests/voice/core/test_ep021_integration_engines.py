"""EP-021 M3 real engine integration tests (directive J proof matrix).

Each test drives a real engine with real models and asserts on the real
output; fixtures supply deterministic audio/text inputs only, never
engine outputs. The engines run in the isolated sidecar venv
(/opt/nexus-voice-engines); the models/fixtures live under /opt
(see infra/voice/engine_env.py).

Run with: uv run --frozen pytest tests/voice/core -q -k ep021_integration
"""

from __future__ import annotations

import sys
import wave
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_ROOT = REPO_ROOT / "python"
for _root in (REPO_ROOT, PYTHON_ROOT):
    if str(_root) not in sys.path:
        sys.path.insert(0, str(_root))

import pytest  # noqa: E402
from nexus_voice import AudioFormat, AudioFrame, VadDecision, WakeWordState  # noqa: E402

from infra.voice.adapters.silero_vad_adapter import VadProviderSilero  # noqa: E402
from infra.voice.adapters.stt_adapter import SttProviderWhisperCpp  # noqa: E402
from infra.voice.adapters.tts_adapter import TtsProviderKokoro  # noqa: E402
from infra.voice.adapters.wake_word_adapter import WakeWordProviderOpenWakeWord  # noqa: E402
from infra.voice.engine_env import FIXTURES_DIR, VoiceEngineError, require_models  # noqa: E402


@pytest.fixture(autouse=True)
def _engines_present() -> None:
    require_models()


def _frame_from_wav(name: str) -> AudioFrame:
    path = FIXTURES_DIR / name
    if not path.exists():
        raise VoiceEngineError(f"fixture missing: {path}")
    with wave.open(str(path), "rb") as w:
        rate = w.getframerate()
        channels = w.getnchannels()
        payload = w.readframes(w.getnframes())
    return AudioFrame(
        format=AudioFormat.PcmS16LE,
        sample_rate_hz=rate,
        channels=channels,
        data=payload,
    )


def test_ep021_integration_silero_detects_real_speech() -> None:
    frame = _frame_from_wav("stt_phrase.wav")
    result = VadProviderSilero().detect(frame)
    assert result.decision == VadDecision.Speech
    assert result.confidence >= 0.5
    assert result.frame is frame


def test_ep021_integration_silero_rejects_real_silence() -> None:
    frame = _frame_from_wav("silence.wav")
    result = VadProviderSilero().detect(frame)
    assert result.decision == VadDecision.Silence
    assert result.confidence < 0.1


def test_ep021_integration_wake_detects_real_trigger() -> None:
    frame = _frame_from_wav("wake_pos_0.wav")
    result = WakeWordProviderOpenWakeWord().detect(frame)
    assert result.state == WakeWordState.Triggered
    assert result.word == "nexus"
    assert result.confidence >= 0.7


def test_ep021_integration_wake_rejects_real_nonwake() -> None:
    provider = WakeWordProviderOpenWakeWord()
    for name in ("wake_neg_0.wav", "silence.wav", "noise.wav"):
        result = provider.detect(_frame_from_wav(name))
        assert result.state == WakeWordState.Armed, name
        assert result.confidence < 0.3, name
        assert result.word is None, name


def test_ep021_integration_whisper_transcribes_real_audio() -> None:
    frame = _frame_from_wav("stt_phrase.wav")
    result = SttProviderWhisperCpp().transcribe([frame])
    assert result.transcript, "whisper returned empty transcript"
    assert "quick brown fox" in result.transcript.lower()
    assert result.frames == 1


def test_ep021_integration_kokoro_synthesizes_new_audio() -> None:
    result = TtsProviderKokoro().synthesize("hello from nexus")
    assert result.frames >= 1
    first = result.audio[0]
    assert first.sample_rate_hz == 24000
    assert first.channels == 1
    total_seconds = sum(frame.duration_seconds() for frame in result.audio)
    assert total_seconds >= 1.0
