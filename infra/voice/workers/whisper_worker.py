#!/usr/bin/env python3
"""whisper.cpp STT worker: real native binary + real model.

Resamples the input to 16 kHz int16 PCM (whisper-cli requires 16 kHz),
invokes the pinned whisper-cli with the pinned model, and prints the
recognized transcript.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
import wave
from pathlib import Path

import numpy as np


def load_resample(path: str) -> np.ndarray:
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
    ap.add_argument("--model", default="/opt/nexus-voice-models/ggml-tiny.en.bin")
    ap.add_argument("--binary", default="/opt/nexus-whisper/build/bin/whisper-cli")
    args = ap.parse_args()

    x = load_resample(args.wav)
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
        tmp_path = tmp.name
        with wave.open(tmp, "wb") as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(16000)
            w.writeframes(x.tobytes())

    proc = subprocess.run(
        [
            args.binary,
            "-m",
            args.model,
            "-f",
            tmp_path,
            "-nt",
            "-np",
            "-l",
            "en",
            "-t",
            "2",
        ],
        capture_output=True,
        text=True,
        timeout=900,
    )
    Path(tmp_path).unlink(missing_ok=True)
    if proc.returncode != 0:
        raise SystemExit(f"whisper-cli failed rc={proc.returncode}: {proc.stderr[-800:]}")

    transcript = " ".join(line for line in proc.stdout.splitlines() if line.strip()).strip()
    print(
        json.dumps(
            {
                "transcript": transcript,
                "language": "en",
                "seconds": round(len(x) / 16000, 4),
                "returncode": proc.returncode,
            }
        )
    )


if __name__ == "__main__":
    main()
