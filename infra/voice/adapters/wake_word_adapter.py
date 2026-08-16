"""openWakeWord wake provider adapter (real engine + Nexus-owned weights).

The bundled noncommercial openwakeword pretrained weights are never
used (SPEC-019). The adapter loads the Nexus-trained commercial-safe
model (``nexus_wake_hey_nexus_v1.onnx``); detection comes from the real
engine streaming loop. Production wake-model certification is DEFERRED
(see infra/voice/manifests/certification.yaml); this adapter certifies
the engine and the controlled-test model only.
"""

from __future__ import annotations

import tempfile
import wave
from pathlib import Path

from nexus_voice.audio import AudioFormat, AudioFrame
from nexus_voice.vocabulary import WakeWordState
from nexus_voice.wake import WakeWordProvider, WakeWordResult

from ..engine_env import WAKE_THRESHOLD, run_worker

WAKE_WORD_LABEL = "nexus"


def _frame_to_wav(frame: AudioFrame, path: str) -> None:
    if frame.format != AudioFormat.PcmS16LE:
        raise ValueError(f"wake engine requires PCM_S16LE frames, got {frame.format}")
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(frame.sample_rate_hz)
        w.writeframes(frame.data)


class WakeWordProviderOpenWakeWord(WakeWordProvider):
    """WakeWordProvider backed by the real openwakeword runtime."""

    def __init__(self, threshold: float = WAKE_THRESHOLD, model: str | None = None) -> None:
        self.threshold = threshold
        self.model = model

    def detect(self, frame: AudioFrame) -> WakeWordResult:
        with tempfile.TemporaryDirectory() as td:
            wav = str(Path(td) / "frame.wav")
            _frame_to_wav(frame, wav)
            args = ["--wav", wav, "--threshold", str(self.threshold)]
            if self.model:
                args += ["--model", self.model]
            result = run_worker("wake_worker.py", *args)
        detected = bool(result["detected"])
        score = max(0.0, min(1.0, float(result["score"])))
        state = WakeWordState.Triggered if detected else WakeWordState.Armed
        return WakeWordResult(
            state=state,
            word=WAKE_WORD_LABEL if detected else None,
            confidence=score,
            frame=frame,
        )
