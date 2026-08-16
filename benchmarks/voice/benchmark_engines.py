#!/usr/bin/env python3
"""EP-021 M5 voice engine benchmark (real wall-clock measurements).

Runs each real engine against the controlled fixtures and records
actual latency. Output is machine-readable and written to
.agent/state/evidence/. This is measurement, not certification: it
documents current CPU-VPS performance for the voice engines.
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
for _root in (REPO_ROOT, PYTHON_ROOT):
    if str(_root) not in sys.path:
        sys.path.insert(0, str(_root))

from infra.voice.engine_env import FIXTURES_DIR, run_worker  # noqa: E402

RUNS = 3


def timed(worker: str, *args: str) -> tuple[float, dict]:
    start = time.monotonic()
    result = run_worker(worker, *args)
    return time.monotonic() - start, result


def main() -> None:
    results: dict[str, object] = {}
    for name, worker, args in [
        ("silero_vad", "silero_worker.py", ["--wav", str(FIXTURES_DIR / "stt_phrase.wav")]),
        ("wake_detection", "wake_worker.py", ["--wav", str(FIXTURES_DIR / "wake_pos_0.wav")]),
        ("whisper_stt", "whisper_worker.py", ["--wav", str(FIXTURES_DIR / "stt_phrase.wav")]),
    ]:
        latencies = []
        last: dict | None = None
        for _ in range(RUNS):
            seconds, last = timed(worker, *args)
            latencies.append(round(seconds, 4))
        results[name] = {
            "runs": latencies,
            "mean_seconds": round(sum(latencies) / len(latencies), 4),
            "max_seconds": round(max(latencies), 4),
            "sample": last,
        }
    # TTS: synthesize a short fixed phrase.
    tts_latencies = []
    last_tts: dict | None = None
    for _ in range(RUNS):
        out = f"/tmp/bench_tts_{_}.wav"
        seconds, last_tts = timed("kokoro_worker.py", "--text", "hello from nexus", "--out", out)
        tts_latencies.append(round(seconds, 4))
    results["kokoro_tts"] = {
        "runs": tts_latencies,
        "mean_seconds": round(sum(tts_latencies) / len(tts_latencies), 4),
        "max_seconds": round(max(tts_latencies), 4),
        "sample": last_tts,
    }

    evidence_dir = REPO_ROOT / ".agent/state/evidence"
    evidence_dir.mkdir(parents=True, exist_ok=True)
    evidence_path = evidence_dir / "EP-021-M5-voice-benchmarks.md"
    with open(evidence_path, "w", encoding="utf-8") as f:
        f.write(
            "# EP-021 M5 voice engine benchmarks (real measurements)\n\n"
            "Wall-clock latency per engine on the controlled fixtures (CPU VPS).\n\n"
            f"```json\n{json.dumps(results, indent=2, sort_keys=True)}\n```\n"
        )
    print(json.dumps(results, indent=2, sort_keys=True))
    print(f"evidence: {evidence_path}")


if __name__ == "__main__":
    main()
