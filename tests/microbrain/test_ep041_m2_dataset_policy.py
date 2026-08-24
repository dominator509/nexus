"""EP-041 M2 unit tests: deterministic Microbrain dataset policy.

Test names begin with ep041_unit_ per the EP-041 milestone contract.
Every test exercises the real dataset policy (python/nexus_microbrain/
dataset_policy.py) against the real M1 contract and the real committed
manifest fixtures under microbrain/datasets/manifests/. Fail-closed
negative cases prove DATASET EXISTS != USABLE TRAINING DATA.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from nexus_microbrain import (
    MICROBRAIN_CODE_INVALID_INPUT,
    MICROBRAIN_CODE_MISSING_REQUIRED,
    MICROBRAIN_CODE_UNSUPPORTED_VERSION,
    DataProvenance,
    DatasetPolicy,
    DatasetVerdict,
    MicrobrainDataset,
    MicrobrainError,
    OodVerdict,
    Role,
    TrainingExample,
    load_manifest,
    sha256_manifest,
    verify_manifest_file,
)

REPO_ROOT = Path(__file__).resolve().parents[2]
MANIFEST_DIR = REPO_ROOT / "microbrain" / "datasets" / "manifests"
SYNTHETIC_MANIFEST = MANIFEST_DIR / "nexus-synthetic-role-ops-v1.manifest.json"
TEACHER_MANIFEST = MANIFEST_DIR / "nexus-teacher-consensus-v1.manifest.json"

POLICY = DatasetPolicy()


def _example(
    *,
    provenance: DataProvenance = DataProvenance.DETERMINISTIC_GENERATION,
    license_ref: str | None = "nexus-synthetic-mit",
    hard_negative: bool = False,
    role: Role = Role.INTERPRETATION,
    ood: OodVerdict = OodVerdict.IN_DISTRIBUTION,
) -> TrainingExample:
    return TrainingExample(
        example_id="ex-policy-1",
        role=role,
        input_text="set the thermostat",
        control_object={
            "schema_version": "1",
            "intent": "home.climate.set",
            "route": "DETERMINISTIC",
            "risk": "R0",
        },
        provenance=provenance,
        hard_negative=hard_negative,
        ood_verdict=ood,
        license_ref=license_ref,
    )


def _dataset(*examples: TrainingExample) -> MicrobrainDataset:
    return MicrobrainDataset(
        dataset_id="ds-policy-1",
        name="ds-policy-1",
        lineage="test-policy-lineage-v1",
        examples=tuple(examples),
    )


def _secret_canary() -> str:
    """Runtime-constructed secret-shaped canary (no tracked literal)."""
    return "sk-" + "live-" + "a" * 30


# ---------------------------------------------------------------------------
# Real manifest loading (M1 contract boundary)
# ---------------------------------------------------------------------------


def ep041_unit_m2_synthetic_manifest_loads() -> None:
    dataset = load_manifest(SYNTHETIC_MANIFEST)
    assert dataset.dataset_id == "nexus-synthetic-role-ops-v1"
    assert len(dataset.examples) == 12


def ep041_unit_m2_teacher_manifest_loads() -> None:
    dataset = load_manifest(TEACHER_MANIFEST)
    assert dataset.dataset_id == "nexus-teacher-consensus-v1"
    assert len(dataset.examples) == 6


def ep041_unit_m2_manifest_missing_file_fails_closed(tmp_path) -> None:
    missing = tmp_path / "missing.manifest.json"
    with pytest.raises(MicrobrainError) as exc:
        load_manifest(missing)
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_m2_manifest_malformed_json_fails_closed(tmp_path) -> None:
    bad = tmp_path / "bad.manifest.json"
    bad.write_text("{not json", encoding="utf-8")
    with pytest.raises(MicrobrainError) as exc:
        load_manifest(bad)
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_m2_manifest_non_object_fails_closed(tmp_path) -> None:
    bad = tmp_path / "list.manifest.json"
    bad.write_text("[1, 2, 3]", encoding="utf-8")
    with pytest.raises(MicrobrainError) as exc:
        load_manifest(bad)
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_m2_manifest_unsupported_version_fails_closed(tmp_path) -> None:
    data = json.loads(SYNTHETIC_MANIFEST.read_text(encoding="utf-8"))
    data["schema_version"] = "999"
    bad = tmp_path / "version.manifest.json"
    bad.write_text(json.dumps(data), encoding="utf-8")
    with pytest.raises(MicrobrainError) as exc:
        load_manifest(bad)
    assert exc.value.code == MICROBRAIN_CODE_UNSUPPORTED_VERSION


# ---------------------------------------------------------------------------
# Policy positive cases on real manifests
# ---------------------------------------------------------------------------


def ep041_unit_m2_synthetic_manifest_policy_usable() -> None:
    dataset = load_manifest(SYNTHETIC_MANIFEST)
    verdict = POLICY.evaluate(dataset)
    assert verdict.usable is True
    assert verdict.licensed is True
    assert verdict.privacy_safe is True
    assert verdict.example_count == 12
    assert verdict.hard_negative_count == 2
    assert verdict.out_of_distribution_count == 4


def ep041_unit_m2_teacher_manifest_policy_usable() -> None:
    dataset = load_manifest(TEACHER_MANIFEST)
    verdict = POLICY.evaluate(dataset)
    assert verdict.usable is True
    assert verdict.licensed is True
    assert verdict.example_count == 6
    assert verdict.provenance_counts["TEACHER_CONSENSUS"] == 4
    assert verdict.provenance_counts["OPTED_IN_SCRUBBED_CORRECTION"] == 2


def ep041_unit_m2_synthetic_manifest_covers_all_roles() -> None:
    dataset = load_manifest(SYNTHETIC_MANIFEST)
    verdict = POLICY.evaluate(dataset)
    assert set(verdict.role_counts) == {
        "INTERPRETATION",
        "CAPABILITY_SELECTION",
        "ROUTING",
        "RISK",
        "PRIVACY",
        "AMBIGUITY",
        "QUOTED_INSTRUCTION",
        "ESCALATION",
    }


def ep041_unit_m2_policy_evaluation_is_deterministic() -> None:
    dataset = load_manifest(SYNTHETIC_MANIFEST)
    first = POLICY.evaluate(dataset)
    second = POLICY.evaluate(dataset)
    assert first == second
    assert first.to_dict() == second.to_dict()


# ---------------------------------------------------------------------------
# Policy negative cases (fail closed)
# ---------------------------------------------------------------------------


def ep041_unit_m2_empty_dataset_denied() -> None:
    verdict = POLICY.evaluate(_dataset())
    assert verdict.usable is False
    assert any("no examples" in reason for reason in verdict.reasons)


def ep041_unit_m2_missing_license_denied() -> None:
    dataset = _dataset(_example(license_ref=None))
    verdict = POLICY.evaluate(dataset)
    assert verdict.usable is False
    assert any("no license_ref" in reason for reason in verdict.reasons)
    assert verdict.licensed is False


def ep041_unit_m2_prohibited_license_denied() -> None:
    dataset = _dataset(_example(license_ref="cc-by-nc-4.0"))
    verdict = POLICY.evaluate(dataset)
    assert verdict.usable is False
    assert any("prohibited license" in reason for reason in verdict.reasons)


def ep041_unit_m2_unknown_license_denied() -> None:
    dataset = _dataset(_example(license_ref="unknown-license-xyz"))
    verdict = POLICY.evaluate(dataset)
    assert verdict.usable is False
    assert any("unknown license" in reason for reason in verdict.reasons)


def ep041_unit_m2_hard_negative_flag_without_provenance_denied() -> None:
    dataset = _dataset(
        _example(
            provenance=DataProvenance.DETERMINISTIC_GENERATION,
            hard_negative=True,
        )
    )
    verdict = POLICY.evaluate(dataset)
    assert verdict.usable is False
    assert any("hard_negative" in reason for reason in verdict.reasons)


def ep041_unit_m2_custom_prohibited_license_set_applies() -> None:
    policy = DatasetPolicy(prohibited_license_refs=frozenset({"mit"}))
    dataset = _dataset(_example(license_ref="mit"))
    verdict = policy.evaluate(dataset)
    assert verdict.usable is False
    assert any("prohibited license" in reason for reason in verdict.reasons)


def ep041_unit_m2_custom_unknown_prefix_applies() -> None:
    policy = DatasetPolicy(unknown_license_prefixes=("proprietary-",))
    dataset = _dataset(_example(license_ref="proprietary-nexus-v1"))
    verdict = policy.evaluate(dataset)
    assert verdict.usable is False
    assert any("unknown license" in reason for reason in verdict.reasons)


def ep041_unit_m2_mixed_denials_list_all_reasons() -> None:
    dataset = _dataset(
        _example(license_ref=None),
        _example(license_ref="cc-by-nc-4.0"),
        _example(
            provenance=DataProvenance.DETERMINISTIC_GENERATION,
            hard_negative=True,
        ),
    )
    verdict = POLICY.evaluate(dataset)
    assert verdict.usable is False
    assert len(verdict.reasons) >= 3


# ---------------------------------------------------------------------------
# Digest verification (manifest freshness binding)
# ---------------------------------------------------------------------------


def ep041_unit_m2_sha256_manifest_returns_alg_hex() -> None:
    digest = sha256_manifest(SYNTHETIC_MANIFEST)
    alg, _, hex_part = digest.partition(":")
    assert alg == "sha256"
    assert len(hex_part) == 64


def ep041_unit_m2_digest_verification_matches_current_bytes() -> None:
    digest = sha256_manifest(SYNTHETIC_MANIFEST)
    verification = verify_manifest_file(SYNTHETIC_MANIFEST, expected_digest=digest)
    assert verification.verified is True
    assert verification.digest == digest


def ep041_unit_m2_digest_mismatch_denied() -> None:
    verification = verify_manifest_file(
        SYNTHETIC_MANIFEST,
        expected_digest="sha256:" + "0" * 64,
    )
    assert verification.verified is False
    assert "digest mismatch" in (verification.reason or "")


def ep041_unit_m2_current_run_digest_recorded() -> None:
    verification = verify_manifest_file(SYNTHETIC_MANIFEST)
    assert verification.verified is True
    assert verification.digest.startswith("sha256:")
    assert verification.digest == sha256_manifest(SYNTHETIC_MANIFEST)


def ep041_unit_m2_verify_missing_file_fails_closed(tmp_path) -> None:
    missing = tmp_path / "missing.manifest.json"
    with pytest.raises(MicrobrainError) as exc:
        verify_manifest_file(missing)
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


# ---------------------------------------------------------------------------
# Composition: load -> verify -> evaluate
# ---------------------------------------------------------------------------


def ep041_unit_m2_load_verify_evaluate_compose() -> None:
    digest = sha256_manifest(SYNTHETIC_MANIFEST)
    verification = verify_manifest_file(SYNTHETIC_MANIFEST, expected_digest=digest)
    assert verification.verified is True
    dataset = load_manifest(SYNTHETIC_MANIFEST)
    verdict = POLICY.evaluate(dataset)
    assert verdict.usable is True
    assert verdict.example_count == len(dataset.examples)


# ---------------------------------------------------------------------------
# Redaction of verdict payloads
# ---------------------------------------------------------------------------


def ep041_unit_m2_verdict_redaction_scrubs_canary() -> None:
    canary = _secret_canary()
    verdict = DatasetVerdict(
        dataset_id="ds-redact",
        usable=False,
        reasons=(f"license leaked {canary}",),
    )
    payload = verdict.to_redacted_dict()
    assert canary not in json.dumps(payload)
    assert "[REDACTED]" in payload["reasons"][0]


def ep041_unit_m2_verdict_to_dict_is_not_redacted() -> None:
    verdict = DatasetVerdict(dataset_id="ds-plain", usable=True)
    payload = verdict.to_dict()
    assert payload["dataset_id"] == "ds-plain"
    assert payload["usable"] is True
