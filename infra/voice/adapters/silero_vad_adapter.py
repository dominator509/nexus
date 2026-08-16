"""Silero VAD provider adapter (real ONNX inference via sidecar worker)."""

from __future__ import annotations

import tempfile
import wave
from pathlib import Path

from nexus_voice.audio import AudioFormat, AudioFrame
from nexus_voice.vad import VadDecision, VadProvider, VadResult

from ..engine_env import SILERO_THRESHOLD
from . import run_engine


def _frame_to_wav(frame: AudioFrame, path: str) -> None:
    if frame.format == AudioFormat.PcmS16LE:
        pcm = frame.data
    elif frame.format == AudioFormat.PcmF32LE:
        import struct

        pcm = b"".join(
            struct.pack("<h", max(-32768, min(32767, int(sample * 32767))))
            for (sample,) in struct.iter_unpack("<f", frame.data)
        )
    else:
        raise ValueError(f"unsupported frame format for VAD: {frame.format}")
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(frame.sample_rate_hz)
        w.writeframes(pcm)


class VadProviderSilero(VadProvider):
    """VadProvider backed by the real Silero VAD v5.1 ONNX graph."""

    def __init__(self, threshold: float = SILERO_THRESHOLD, model: str | None = None) -> None:
        self.threshold = threshold
        self.model = model

    def detect(self, frame: AudioFrame) -> VadResult:
        with tempfile.TemporaryDirectory() as td:
            wav = str(Path(td) / "frame.wav")
            _frame_to_wav(frame, wav)
            args = ["--wav", wav, "--threshold", str(self.threshold)]
            if self.model:
                args += ["--model", self.model]
            result = run_engine("silero_worker.py", *args)
        decision = result["decision"]
        if decision not in (VadDecision.Speech, VadDecision.Silence):
            raise ValueError(f"worker returned unknown decision: {decision!r}")
        confidence = max(0.0, min(1.0, float(result["mean_prob"])))
        return VadResult(decision=decision, confidence=confidence, frame=frame)
