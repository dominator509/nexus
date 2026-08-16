"""Nexus wake model core (SPEC-012 behavior 1; SPEC-019; EP-021 M2).

Owns the deterministic wake-model machinery: model manifests, license
safety (commercial-safe weights only; noncommercial weights are
prohibited by SPEC-019 required behavior 2 and SPEC-012 non-goal
"shipping noncommercial wake weights"), digest verification, an
idempotent model registry, and the armed/triggered/disarmed/uncertified
decision state machine. The real openWakeWord runtime inference plugs
in behind the score port (M3 infra/voice); this core never fabricates a
trigger and never ships noncommercial weights.
"""

from __future__ import annotations

from .decision import WakeDecisionEngine, WakeModelScore
from .manifest import (
    LicenseClass,
    WakeModelManifest,
    WakeModelManifestError,
    verify_weights_digest,
)
from .registry import (
    WakeModelConflict,
    WakeModelNotFound,
    WakeModelRegistry,
)

__all__ = [
    "LicenseClass",
    "WakeDecisionEngine",
    "WakeModelConflict",
    "WakeModelManifest",
    "WakeModelManifestError",
    "WakeModelNotFound",
    "WakeModelRegistry",
    "WakeModelScore",
    "verify_weights_digest",
]
