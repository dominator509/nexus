"""EP-021 M3 composed voice pipeline (stdlib-only orchestration).

``run_chain`` proves the bounded real chain through the real engines:

    audio -> Silero VAD -> wake detection -> captured utterance
            -> whisper.cpp transcription

``synthesize`` proves the independent TTS chain:

    text -> Kokoro -> generated audio

All signal processing happens inside the engine workers; this module
only orchestrates them and reads canonical JSON.
"""

from __future__ import annotations

import tempfile
import wave
from pathlib import Path

from ..engine_env import WAKE_THRESHOLD
from . import run_engine


def run_chain(wav_path: str, wake_threshold: float = WAKE_THRESHOLD) -> dict:
    """Run the bounded voice chain over a WAV file and return evidence."""
    vad = run_engine("silero_worker.py", "--wav", wav_path)
    wake = run_engine("wake_worker.py", "--wav", wav_path, "--threshold", str(wake_threshold))
    segments = [[float(a), float(b)] for a, b in vad.get("segments", [])]
    trigger_seconds = wake.get("trigger_seconds")

    utterance_span: list[float] | None = None
    transcript = ""
    if trigger_seconds is not None and segments:
        wake_seg = None
        for a, b in segments:
            if a <= trigger_seconds <= b:
                wake_seg = (a, b)
                break
        if wake_seg is not None:
            post = [seg for seg in segments if seg[0] >= wake_seg[1]]
            if post:
                start_s, end_s = wake_seg[1], post[-1][1]
                utterance_span = [start_s, end_s]
                with tempfile.TemporaryDirectory() as td:
                    crop_wav = str(Path(td) / "utterance.wav")
                    run_engine(
                        "crop_worker.py",
                        "--wav",
                        wav_path,
                        "--start",
                        str(start_s),
                        "--end",
                        str(end_s),
                        "--out",
                        crop_wav,
                    )
                    stt = run_engine("whisper_worker.py", "--wav", crop_wav)
                    transcript = str(stt.get("transcript", ""))

    return {
        "vad_decision": vad.get("decision"),
        "vad_mean_prob": vad.get("mean_prob"),
        "vad_segments": segments,
        "wake_score": wake.get("score"),
        "wake_detected": wake.get("detected"),
        "wake_trigger_seconds": trigger_seconds,
        "utterance_span": utterance_span,
        "transcript": transcript,
    }


def synthesize(text: str, out_wav: str, voice: str = "af_heart", speed: float = 1.0) -> dict:
    """Synthesize text to a WAV through real Kokoro inference."""
    result = run_engine(
        "kokoro_worker.py",
        "--text",
        text,
        "--out",
        out_wav,
        "--voice",
        voice,
        "--speed",
        str(speed),
    )
    return result


def wav_properties(path: str) -> dict:
    """Read WAV properties from a generated file (stdlib)."""
    with wave.open(path, "rb") as w:
        return {
            "channels": w.getnchannels(),
            "sample_rate_hz": w.getframerate(),
            "frames": w.getnframes(),
            "seconds": w.getnframes() / w.getframerate(),
        }
