"""EP-021 M3 provider adapters (project interpreter, stdlib-only).

Each adapter wraps a real engine worker (subprocess into the isolated
voice venv) and maps the canonical JSON back onto the nexus_voice
contract types. The adapters contain no inference code of their own;
every result originates from a real engine run.

Failures fail closed with typed SPEC-006 VoiceErrors (M4): engine
startup/model/runtime failures surface as UNAVAILABLE and real
subprocess timeouts surface as TIMEOUT; no raw audio ever reaches an
error surface.
"""

from __future__ import annotations

import subprocess

from nexus_voice.error import VoiceError, VoiceErrorCode

from ..engine_env import VoiceEngineError, run_worker

__all__ = ["run_engine", "VoiceEngineError"]


def run_engine(*args: str, timeout: int = 900) -> dict:
    """Run an engine worker, mapping real failures to typed VoiceErrors."""
    try:
        return run_worker(*args, timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        raise VoiceError(
            VoiceErrorCode.Timeout,
            f"voice engine timed out: {args[0]}",
            detail={"worker": args[0]},
        ) from exc
    except VoiceEngineError as exc:
        raise VoiceError(
            VoiceErrorCode.Unavailable,
            f"voice engine unavailable: {args[0]}",
            detail={"worker": args[0]},
        ) from exc
