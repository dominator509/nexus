"""whisper.cpp STT provider adapter (real native binary + real model)."""

from __future__ import annotations

import tempfile
import wave
from pathlib import Path

from nexus_voice.audio import AudioFormat, AudioFrame
from nexus_voice.stt import SttProvider, SttResult

from ..engine_env import run_worker


def _frames_to_wav(frames: list[AudioFrame], path: str) -> None:
    if not frames:
        raise ValueError("cannot transcribe an empty frame list")
    fmt = frames[0].format
    rate = frames[0].sample_rate_hz
    if fmt != AudioFormat.PcmS16LE:
        raise ValueError(f"whisper adapter requires PCM_S16LE frames, got {fmt}")
    payload = b"".join(frame.data for frame in frames)
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(payload)


class SttProviderWhisperCpp(SttProvider):
    """SttProvider backed by whisper.cpp."""

    def transcribe(self, frames: list[AudioFrame]) -> SttResult:
        with tempfile.TemporaryDirectory() as td:
            wav = str(Path(td) / "frames.wav")
            _frames_to_wav(frames, wav)
            result = run_worker("whisper_worker.py", "--wav", wav)
        return SttResult(
            transcript=str(result["transcript"]),
            confidence=None,
            language=str(result.get("language")),
            frames=len(frames),
        )
