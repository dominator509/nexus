"""EP-021 M2 unit tests: wake model core behavior (SPEC-012, SPEC-019).

Run with: uv run --frozen pytest models/wake/tests -q -o 'python_functions=ep021_unit_*'
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_ROOT = REPO_ROOT / "python"
WAKE_ROOT = REPO_ROOT / "models" / "wake"
for _root in (PYTHON_ROOT, WAKE_ROOT):
    if str(_root) not in sys.path:
        sys.path.insert(0, str(_root))

import pytest  # noqa: E402
from nexus_voice import AudioEndpointKind, AudioFormat, AudioFrame, WakeWordState  # noqa: E402
from nexus_voice.error import VoiceError, VoiceErrorCode  # noqa: E402
from nexus_wake import (  # noqa: E402
    LicenseClass,
    WakeDecisionEngine,
    WakeModelConflict,
    WakeModelManifest,
    WakeModelManifestError,
    WakeModelNotFound,
    WakeModelRegistry,
    WakeModelScore,
    verify_weights_digest,
)


def _frame() -> AudioFrame:
    return AudioFrame(AudioFormat.PcmS16LE, 16000, 1, b"\x00" * 32, AudioEndpointKind.X86Linux)


def _manifest(
    model_id: str = "nexus-hello", license_class: str = LicenseClass.Permissive
) -> WakeModelManifest:
    return WakeModelManifest(
        model_id=model_id,
        version="1.0.0",
        digest_sha256="a" * 64,
        license_class=license_class,
        license_name="Apache-2.0",
        provenance="nexus training pipeline (controlled corpus)",
        owner="nexus-voice",
        replacement_boundary="wake registry id",
    )


def _manifest_for_weights(weights: bytes, model_id: str = "nexus-hello") -> WakeModelManifest:
    import hashlib

    return WakeModelManifest(
        model_id=model_id,
        version="1.0.0",
        digest_sha256=hashlib.sha256(weights).hexdigest(),
        license_class=LicenseClass.Permissive,
        license_name="Apache-2.0",
        provenance="nexus training pipeline (controlled corpus)",
        owner="nexus-voice",
        replacement_boundary="wake registry id",
    )


# ---------------------------------------------------------------------------
# Manifest / license safety (SPEC-019)
# ---------------------------------------------------------------------------


def ep021_unit_manifest_accepts_permissive() -> None:
    manifest = _manifest()
    assert manifest.commercial_safe is True
    assert manifest.digest_sha256 == "a" * 64


def ep021_unit_manifest_rejects_noncommercial() -> None:
    with pytest.raises(WakeModelManifestError):
        _manifest(license_class=LicenseClass.NonCommercial)


def ep021_unit_manifest_rejects_bad_digest_shape() -> None:
    with pytest.raises(ValueError):
        WakeModelManifest(
            model_id="m",
            version="1",
            digest_sha256="zz",
            license_class=LicenseClass.Permissive,
            license_name="Apache-2.0",
            provenance="p",
            owner="o",
        )


def ep021_unit_manifest_rejects_empty_ids() -> None:
    with pytest.raises(WakeModelManifestError):
        WakeModelManifest(
            model_id="",
            version="1",
            digest_sha256="a" * 64,
            license_class=LicenseClass.Permissive,
            license_name="Apache-2.0",
            provenance="p",
            owner="o",
        )


def ep021_unit_digest_verification() -> None:
    import hashlib

    weights = b"real-weights-bytes"
    manifest = WakeModelManifest(
        model_id="m",
        version="1",
        digest_sha256=hashlib.sha256(weights).hexdigest(),
        license_class=LicenseClass.Permissive,
        license_name="Apache-2.0",
        provenance="p",
        owner="o",
    )
    assert verify_weights_digest(manifest, weights) is True
    assert verify_weights_digest(manifest, b"tampered") is False


# ---------------------------------------------------------------------------
# Registry (idempotent, conflict, digest gate)
# ---------------------------------------------------------------------------


def ep021_unit_registry_register_and_get() -> None:
    registry = WakeModelRegistry()
    manifest = _manifest_for_weights(b"weights")
    assert registry.register(manifest, b"weights") is True
    assert registry.contains("nexus-hello")
    got_manifest, got_weights = registry.get("nexus-hello")
    assert got_manifest == manifest
    assert got_weights == b"weights"
    assert len(registry) == 1


def ep021_unit_registry_idempotent_duplicate() -> None:
    registry = WakeModelRegistry()
    assert registry.register(_manifest_for_weights(b"weights"), b"weights") is True
    assert registry.register(_manifest_for_weights(b"weights"), b"weights") is False
    assert len(registry) == 1


def ep021_unit_registry_conflict_on_different_digest() -> None:
    registry = WakeModelRegistry()
    registry.register(_manifest_for_weights(b"weights"), b"weights")
    different = _manifest_for_weights(b"other-weights")
    with pytest.raises(WakeModelConflict):
        registry.register(different, b"other-weights")


def ep021_unit_registry_rejects_digest_mismatch() -> None:
    registry = WakeModelRegistry()
    with pytest.raises(ValueError):
        registry.register(_manifest(), b"not-the-right-bytes")


def ep021_unit_registry_missing_model_raises() -> None:
    registry = WakeModelRegistry()
    with pytest.raises(WakeModelNotFound):
        registry.get("missing")


# ---------------------------------------------------------------------------
# Decision engine (deterministic state machine)
# ---------------------------------------------------------------------------


def ep021_unit_decision_trigger_on_threshold() -> None:
    engine = WakeDecisionEngine(threshold=0.5)
    engine.arm("nexus-hello")
    result = engine.decide(WakeModelScore("nexus-hello", "hello", 0.9), _frame())
    assert result.state == WakeWordState.Triggered
    assert result.word == "hello"
    assert result.confidence == pytest.approx(0.9)


def ep021_unit_decision_no_trigger_below_threshold() -> None:
    engine = WakeDecisionEngine(threshold=0.5)
    engine.arm("nexus-hello")
    result = engine.decide(WakeModelScore("nexus-hello", "hello", 0.2), _frame())
    assert result.state == WakeWordState.Armed
    assert result.word is None


def ep021_unit_decision_unarmed_is_uncertified() -> None:
    engine = WakeDecisionEngine()
    with pytest.raises(VoiceError) as excinfo:
        engine.decide(WakeModelScore("nexus-hello", "hello", 0.9), _frame())
    assert excinfo.value.code == VoiceErrorCode.Unavailable


def ep021_unit_decision_rejects_wrong_model_mix() -> None:
    engine = WakeDecisionEngine()
    engine.arm("nexus-hello")
    with pytest.raises(VoiceError) as excinfo:
        engine.decide(WakeModelScore("other-model", "hello", 0.9), _frame())
    assert excinfo.value.code == VoiceErrorCode.Unavailable


def ep021_unit_decision_disarm_is_idempotent() -> None:
    engine = WakeDecisionEngine()
    engine.arm("nexus-hello")
    engine.disarm()
    engine.disarm()
    assert engine.armed_model is None


def ep021_unit_decision_arm_is_idempotent() -> None:
    engine = WakeDecisionEngine()
    engine.arm("nexus-hello")
    engine.arm("nexus-hello")
    assert engine.armed_model == "nexus-hello"


def ep021_unit_decision_rejects_bad_threshold() -> None:
    with pytest.raises(ValueError):
        WakeDecisionEngine(threshold=1.5)
    with pytest.raises(ValueError):
        WakeDecisionEngine(threshold=-0.1)


def ep021_unit_decision_score_validates() -> None:
    with pytest.raises(ValueError):
        WakeModelScore("m", "word", 1.5)
    with pytest.raises(ValueError):
        WakeModelScore("", "word", 0.5)
    with pytest.raises(ValueError):
        WakeModelScore("m", "", 0.5)


# ---------------------------------------------------------------------------
# Dependency direction: nexus_wake must not import application layers
# ---------------------------------------------------------------------------


def ep021_unit_dependency_direction_wake_core() -> None:
    import sys as _sys

    for name in ("nexus_connector_sdk", "nexus_contracts", "packages", "temporal"):
        for mod_name in list(_sys.modules):
            if mod_name == name or mod_name.startswith(name + "."):
                raise AssertionError(f"nexus_wake must not import {name}")
