#!/usr/bin/env python3
"""Silero VAD ONNX worker: real model + real ONNX inference.

Reads a WAV, resamples to 16 kHz mono float32, runs the official Silero
VAD v5.1 ONNX graph over 512-sample windows with carried state, and
prints the canonical VAD JSON.
"""

from __future__ import annotations

import argparse
import json
import wave

import numpy as np
import onnxruntime as ort


def load_f32(path: str) -> tuple[np.ndarray, int]:
    with wave.open(path, "rb") as w:
        sr = w.getframerate()
        raw = w.readframes(w.getnframes())
    x = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0
    if sr != 16000:
        n = int(len(x) * 16000 / sr)
        x = np.interp(np.linspace(0, len(x) - 1, n), np.arange(len(x)), x).astype(np.float32)
    return x, 16000


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--wav", required=True)
    ap.add_argument("--model", default="/opt/nexus-voice-models/silero_vad_v5.1.onnx")
    ap.add_argument("--threshold", type=float, default=0.5)
    ap.add_argument("--segment-threshold", type=float, default=0.3)
    ap.add_argument("--min-segment-seconds", type=float, default=0.25)
    args = ap.parse_args()

    sess = ort.InferenceSession(args.model, providers=["CPUExecutionProvider"])
    x, _ = load_f32(args.wav)
    h = np.zeros((1, 1, 128), dtype=np.float32)
    c = np.zeros((1, 1, 128), dtype=np.float32)
    sr = np.array(16000, dtype=np.int64)
    probs: list[float] = []
    step = 512
    for i in range(0, len(x) - step + 1, step):
        out, state_n = sess.run(
            ["output", "stateN"],
            {
                "input": x[i : i + step][None, :],
                "state": np.concatenate([h, c], axis=0),
                "sr": sr,
            },
        )
        h, c = state_n[:1], state_n[1:]
        probs.append(float(out[0][0]))

    arr = np.asarray(probs, dtype=np.float64)
    mean = float(arr.mean()) if arr.size else 0.0
    decision = "SPEECH" if mean >= args.threshold else "SILENCE"
    # Speech segments from per-window probabilities (real VAD hangover-free).
    segments: list[list[float]] = []
    in_seg = False
    seg_start = 0.0
    for wi, p in enumerate(arr):
        t0 = wi * step / 16000
        t1 = (wi + 1) * step / 16000
        if p > args.segment_threshold and not in_seg:
            seg_start = t0
            in_seg = True
        if p <= args.segment_threshold and in_seg:
            if t1 - seg_start >= args.min_segment_seconds:
                segments.append([round(seg_start, 4), round(t1, 4)])
            in_seg = False
    if in_seg and len(arr):
        segments.append([round(seg_start, 4), round(len(arr) * step / 16000, 4)])
    print(
        json.dumps(
            {
                "decision": decision,
                "mean_prob": round(mean, 6),
                "max_prob": round(float(arr.max()), 6) if arr.size else 0.0,
                "window_count": int(arr.size),
                "speech_window_count": int((arr >= args.threshold).sum()),
                "seconds": round(len(x) / 16000, 4),
                "segments": segments,
            }
        )
    )


if __name__ == "__main__":
    main()
