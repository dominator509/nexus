#!/usr/bin/env python3
"""Kokoro TTS worker: real model + real CPU inference -> new waveform.

Synthesizes text through Kokoro (torch CPU), writes a 24 kHz mono WAV,
and prints canonical TTS JSON (duration, RMS, SHA-256 of the WAV).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import wave

import numpy as np
from kokoro import KPipeline

SAMPLE_RATE = 24000


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--text", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--voice", default="af_heart")
    ap.add_argument("--speed", type=float, default=1.0)
    args = ap.parse_args()

    pipeline = KPipeline(lang_code="a", repo_id="hexgrad/Kokoro-82M")
    parts = [audio for _, _, audio in pipeline(args.text, voice=args.voice, speed=args.speed)]
    if not parts:
        raise SystemExit("kokoro produced no audio")
    audio = np.concatenate(parts).astype(np.float32)
    pcm = (np.clip(audio, -1.0, 1.0) * 32767).astype(np.int16)
    with wave.open(args.out, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SAMPLE_RATE)
        w.writeframes(pcm.tobytes())

    with open(args.out, "rb") as f:
        sha256 = hashlib.sha256(f.read()).hexdigest()
    rms = float(np.sqrt(np.mean(audio.astype(np.float64) ** 2)))
    print(
        json.dumps(
            {
                "wav": args.out,
                "sample_rate_hz": SAMPLE_RATE,
                "duration_seconds": round(len(pcm) / SAMPLE_RATE, 4),
                "rms": round(rms, 6),
                "sha256": sha256,
                "voice": args.voice,
            }
        )
    )


if __name__ == "__main__":
    main()
