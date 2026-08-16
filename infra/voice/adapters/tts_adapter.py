"""Kokoro TTS provider adapter (real model + CPU inference)."""

from __future__ import annotations

import tempfile
import wave
from pathlib import Path

from nexus_voice.audio import AudioFormat, AudioFrame
from nexus_voice.tts import TtsProvider, TtsResult

from . import run_engine

CHUNK_SAMPLES = 24000  # one second at the Kokoro output rate


class TtsProviderKokoro(TtsProvider):
    """TtsProvider backed by Kokoro (torch CPU)."""

    def __init__(self, voice: str = "af_heart", speed: float = 1.0) -> None:
        self.voice = voice
        self.speed = speed

    def synthesize(self, text: str) -> TtsResult:
        with tempfile.TemporaryDirectory() as td:
            out = str(Path(td) / "tts.wav")
            run_engine(
                "kokoro_worker.py",
                "--text",
                text,
                "--out",
                out,
                "--voice",
                self.voice,
                "--speed",
                str(self.speed),
            )
            with wave.open(out, "rb") as w:
                rate = w.getframerate()
                channels = w.getnchannels()
                payload = w.readframes(w.getnframes())
        frames: list[AudioFrame] = []
        for sequence, offset in enumerate(range(0, len(payload), CHUNK_SAMPLES * 2)):
            chunk = payload[offset : offset + CHUNK_SAMPLES * 2]
            if not chunk:
                break
            frames.append(
                AudioFrame(
                    format=AudioFormat.PcmS16LE,
                    sample_rate_hz=rate,
                    channels=channels,
                    data=chunk,
                    sequence=sequence,
                )
            )
        if not frames:
            raise ValueError("kokoro produced no audio frames")
        return TtsResult(audio=tuple(frames))
