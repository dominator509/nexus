"""EP-021 M3 engine environment resolver and worker runner (stdlib only).

The voice engines run in an isolated sidecar environment
(``/opt/nexus-voice-engines`` by default; override with
``NEXUS_VOICE_ENGINE_VENV``) so the main Nexus Python environment stays
frozen (EP-021 directive G). This module is importable by the project
interpreter (3.14) and by tests; the workers themselves run under the
engine venv python (3.12) and exchange canonical JSON on stdout.

Runtime artifact roots (never committed):
  - engines:   ``$NEXUS_VOICE_ENGINE_VENV`` (default /opt/nexus-voice-engines)
  - models:    ``$NEXUS_VOICE_MODELS``     (default /opt/nexus-voice-models)
  - fixtures:  ``$NEXUS_VOICE_FIXTURES``   (default /opt/nexus-voice-fixtures)
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

ENGINE_VENV = Path(os.environ.get("NEXUS_VOICE_ENGINE_VENV", "/opt/nexus-voice-engines"))
MODELS_DIR = Path(os.environ.get("NEXUS_VOICE_MODELS", "/opt/nexus-voice-models"))
FIXTURES_DIR = Path(os.environ.get("NEXUS_VOICE_FIXTURES", "/opt/nexus-voice-fixtures"))
WORKERS_DIR = Path(__file__).resolve().parent / "workers"

SILERO_MODEL = MODELS_DIR / "silero_vad_v5.1.onnx"
WAKE_MODEL = MODELS_DIR / "nexus_wake_hey_nexus_v1.onnx"
WHISPER_MODEL = MODELS_DIR / "ggml-tiny.en.bin"
WHISPER_BINARY = Path("/opt/nexus-whisper/build/bin/whisper-cli")

SILERO_THRESHOLD = 0.5
WAKE_THRESHOLD = 0.7


class VoiceEngineError(RuntimeError):
    """A real engine worker failed; the stderr is preserved."""


def engine_python() -> Path:
    python = ENGINE_VENV / "bin" / "python"
    if not python.exists():
        raise VoiceEngineError(f"engine venv python missing: {python} (run infra/voice setup)")
    return python


def require_models() -> None:
    for model in (SILERO_MODEL, WAKE_MODEL, WHISPER_MODEL):
        if not model.exists():
            raise VoiceEngineError(f"required model missing: {model}")


def run_worker(worker: str, *args: str, timeout: int = 900) -> dict:
    """Run one engine worker under the sidecar venv and parse its JSON.

    The ambient VIRTUAL_ENV is stripped so the sidecar interpreter is not
    shadowed by the outer project/hermes environment (spaCy resolution).
    """
    worker_path = WORKERS_DIR / worker
    if not worker_path.exists():
        raise VoiceEngineError(f"worker missing: {worker_path}")
    cmd = [str(engine_python()), str(worker_path), *args]
    env = dict(os.environ)
    env.pop("VIRTUAL_ENV", None)
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=env)
    if proc.returncode != 0:
        raise VoiceEngineError(f"{worker} failed rc={proc.returncode}: {proc.stderr[-2000:]}")
    lines = [line for line in proc.stdout.splitlines() if line.strip()]
    if not lines:
        raise VoiceEngineError(f"{worker} produced no output")
    try:
        return json.loads(lines[-1])
    except json.JSONDecodeError as exc:  # pragma: no cover - diagnostic
        raise VoiceEngineError(f"{worker} bad JSON: {exc}: {proc.stdout[-500:]}") from exc
