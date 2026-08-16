#!/usr/bin/env python3
"""Audio crop worker: real signal crop of a WAV region (seconds)."""

from __future__ import annotations

import argparse
import json
import wave

import numpy as np


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--wav", required=True)
    ap.add_argument("--start", type=float, required=True)
    ap.add_argument("--end", type=float, required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    with wave.open(args.wav, "rb") as w:
        sr = w.getframerate()
        raw = w.readframes(w.getnframes())
    x = np.frombuffer(raw, dtype=np.int16)
    lo = max(0, int(args.start * sr))
    hi = min(len(x), int(args.end * sr))
    if hi <= lo:
        raise SystemExit(f"empty crop [{args.start}, {args.end}]s")
    with wave.open(args.out, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(x[lo:hi].tobytes())
    print(
        json.dumps(
            {
                "wav": args.out,
                "start": args.start,
                "end": args.end,
                "duration_seconds": round((hi - lo) / sr, 4),
                "sample_rate_hz": sr,
            }
        )
    )


if __name__ == "__main__":
    main()
