"""EP-021 M4 forced-failure tests (real mechanisms, never mocks).

Each test exercises a REAL failure of the integrated engines: a missing
model file, a corrupt WAV, a permission-denied model, a missing sidecar
venv, and a genuine subprocess timeout. Providers must fail closed with
typed SPEC-006 VoiceErrors (UNAVAILABLE/TIMEOUT) and never fabricate
results; error surfaces must not carry raw audio.

Run with: uv run --frozen pytest tests/voice/core -q -k ep021_failure
"""

from __future__ import annotations

import stat
import sys
import wave
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_ROOT = REPO_ROOT / "python"
for _root in (REPO_ROOT, PYTHON_ROOT):
    if str(_root) not in sys.path:
        sys.path.insert(0, str(_root))

import pytest  # noqa: E402
from nexus_voice import AudioFormat, AudioFrame, VoiceError, VoiceErrorCode  # noqa: E402

import infra.voice.engine_env as engine_env  # noqa: E402
from infra.voice.adapters import run_engine  # noqa: E402
from infra.voice.adapters.silero_vad_adapter import VadProviderSilero  # noqa: E402
from infra.voice.adapters.stt_adapter import SttProviderWhisperCpp  # noqa: E402
from infra.voice.adapters.wake_word_adapter import WakeWordProviderOpenWakeWord  # noqa: E402
from infra.voice.engine_env import FIXTURES_DIR, require_models  # noqa: E402


@pytest.fixture(autouse=True)
def _engines_present() -> None:
    require_models()


def _frame_from_wav(name: str) -> AudioFrame:
    path = FIXTURES_DIR / name
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


def _corrupt_wav(tmp_path: Path) -> str:
    bad = tmp_path / "corrupt.wav"
    bad.write_bytes(b"RIFF\x00\x00\x00\x00not a real wav body\xff\xfe")
    return str(bad)


def _assert_unavailable(exc: VoiceError, worker_hint: str) -> None:
    assert exc.code == VoiceErrorCode.Unavailable
    assert worker_hint in exc.message
    payload = exc.as_dict()
    assert "data" not in payload and "audio" not in payload["message"]


def test_ep021_failure_missing_silero_model(tmp_path: Path) -> None:
    frame = _frame_from_wav("stt_phrase.wav")
    missing = str(tmp_path / "missing.onnx")
    with pytest.raises(VoiceError) as raised:
        VadProviderSilero(model=missing).detect(frame)
    _assert_unavailable(raised.value, "silero_worker")


def test_ep021_failure_missing_wake_model(tmp_path: Path) -> None:
    frame = _frame_from_wav("wake_pos_0.wav")
    missing = str(tmp_path / "missing.onnx")
    with pytest.raises(VoiceError) as raised:
        WakeWordProviderOpenWakeWord(model=missing).detect(frame)
    _assert_unavailable(raised.value, "wake_worker")


def test_ep021_failure_missing_whisper_model(tmp_path: Path) -> None:
    frame = _frame_from_wav("stt_phrase.wav")
    missing = str(tmp_path / "missing.bin")
    with pytest.raises(VoiceError) as raised:
        SttProviderWhisperCpp(model=missing).transcribe([frame])
    _assert_unavailable(raised.value, "whisper_worker")


def test_ep021_failure_unsupported_frame_format() -> None:
    # Malformed input at the adapter boundary: the provider refuses a
    # compressed frame before any engine is invoked (typed validation).
    frame = AudioFrame(
        format=AudioFormat.Mp3,
        sample_rate_hz=24000,
        channels=1,
        data=b"\xff\xfb\x90\x64",
    )
    with pytest.raises(ValueError):
        VadProviderSilero().detect(frame)
    with pytest.raises(ValueError):
        WakeWordProviderOpenWakeWord().detect(frame)


def test_ep021_failure_corrupt_wav_input(tmp_path: Path) -> None:
    # Malformed input reaching the real engine: the worker itself fails
    # on a corrupt WAV and the typed UNAVAILABLE error is raised.
    corrupt = _corrupt_wav(tmp_path)
    with pytest.raises(VoiceError) as raised:
        run_engine("silero_worker.py", "--wav", corrupt)
    assert raised.value.code == VoiceErrorCode.Unavailable
    payload = raised.value.as_dict()
    assert "audio" not in payload["message"]


def test_ep021_failure_model_permission_denied(tmp_path: Path) -> None:
    frame = _frame_from_wav("stt_phrase.wav")
    denied = tmp_path / "denied.onnx"
    denied.write_bytes(b"\x00" * 64)
    denied.chmod(0)
    try:
        with pytest.raises(VoiceError) as raised:
            VadProviderSilero(model=str(denied)).detect(frame)
        _assert_unavailable(raised.value, "silero_worker")
    finally:
        denied.chmod(stat.S_IRUSR | stat.S_IWUSR)


def test_ep021_failure_missing_engine_venv(monkeypatch: pytest.MonkeyPatch) -> None:
    frame = _frame_from_wav("stt_phrase.wav")
    monkeypatch.setattr(engine_env, "ENGINE_VENV", Path("/nonexistent-voice-venv"))
    with pytest.raises(VoiceError) as raised:
        VadProviderSilero().detect(frame)
    assert raised.value.code == VoiceErrorCode.Unavailable


def test_ep021_failure_engine_timeout(monkeypatch: pytest.MonkeyPatch) -> None:
    frame = _frame_from_wav("stt_phrase.wav")
    # The whisper worker on a real 3+ second clip cannot finish in 1s:
    # this is a real subprocess timeout, not a fake timer.
    provider = SttProviderWhisperCpp(timeout=1)
    with pytest.raises(VoiceError) as raised:
        provider.transcribe([frame])
    assert raised.value.code == VoiceErrorCode.Timeout
    payload = raised.value.as_dict()
    assert "audio" not in payload["message"]


def test_ep021_failure_run_engine_maps_worker_failure(tmp_path: Path) -> None:
    missing = str(tmp_path / "nope.onnx")
    with pytest.raises(VoiceError) as raised:
        run_engine(
            "silero_worker.py", "--wav", str(FIXTURES_DIR / "silence.wav"), "--model", missing
        )
    assert raised.value.code == VoiceErrorCode.Unavailable
