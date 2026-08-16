#!/usr/bin/env python3
"""openWakeWord wake worker: real engine + custom commercial-safe model.

Streams 16 kHz int16 audio in 1280-sample frames through the real
openwakeword ``Model.predict`` loop (which applies the engine's own
feature frontend and prediction buffer semantics) and prints the max
score, trigger frame, and detected flag.
"""

from __future__ import annotations

import argparse
import json
import wave

import numpy as np
from openwakeword.model import Model as WakeModel


def load_int16(path: str) -> np.ndarray:
    with wave.open(path, "rb") as w:
        sr = w.getframerate()
        raw = w.readframes(w.getnframes())
    x = np.frombuffer(raw, dtype=np.int16)
    if sr != 16000:
        n = int(len(x) * 16000 / sr)
        x = np.interp(
            np.linspace(0, len(x) - 1, n), np.arange(len(x)), x.astype(np.float64)
        ).astype(np.int16)
    return x


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--wav", required=True)
    ap.add_argument("--model", default="/opt/nexus-voice-models/nexus_wake_hey_nexus_v1.onnx")
    ap.add_argument("--threshold", type=float, default=0.7)
    args = ap.parse_args()

    x = load_int16(args.wav)
    engine = WakeModel(
        wakeword_model_paths=[args.model],
        class_mapping_dicts=[{"0": "negative", "1": "nexus"}],
    )
    step = 1280
    best = 0.0
    trigger_frame: int | None = None
    for i in range(0, len(x) - step + 1, step):
        pred = engine.predict(x[i : i + step])
        score = max(pred.values())
        if score > best:
            best = float(score)
        if score >= args.threshold and trigger_frame is None:
            trigger_frame = i

    print(
        json.dumps(
            {
                "score": round(best, 6),
                "detected": bool(best >= args.threshold),
                "trigger_frame": trigger_frame,
                "trigger_seconds": round(trigger_frame / 16000, 4)
                if trigger_frame is not None
                else None,
                "threshold": args.threshold,
                "frames": int((len(x) - step + 1) // step),
            }
        )
    )


if __name__ == "__main__":
    main()
