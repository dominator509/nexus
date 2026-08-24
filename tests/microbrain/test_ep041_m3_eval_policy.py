"""EP-041 M3 unit tests: frozen eval suites and deterministic scoring.

Test names begin with ep041_unit_ per the EP-041 milestone contract.
Every test exercises the real eval behavior
(python/nexus_microbrain/eval_policy.py) against the real M1 contract
and the real committed eval fixtures under microbrain/evals/suites/.
FROZEN EVAL EXISTS != EVAL PASSED; timing, immutability, dataset
binding, OOD, and hard-negative gates all fail closed.
"""

from __future__ import annotations

import json

import pytest
from nexus_microbrain import (
    MICROBRAIN_CODE_MISSING_REQUIRED,
    DataProvenance,
    DatasetPolicy,
    DimensionResult,
    EvalDimension,
    FrozenEval,
    FrozenEvalSuite,
    MicrobrainDataset,
    MicrobrainError,
    OodVerdict,
    Role,
    SuiteBinding,
    TrainingExample,
    bind_suite_to_dataset,
    build_eval_evidence,
    check_eval_before_training,
    load_manifest,
    load_suite_binding,
    score_eval,
    score_suite,
    sha256_manifest,
    suite_digest,
    validate_suite,
    verify_suite_digest,
)

REPO_ROOT = __import__("pathlib").Path(__file__).resolve().parents[2]
SUITE_DIR = REPO_ROOT / "microbrain" / "evals" / "suites"
SUITE_JSON = SUITE_DIR / "nexus-frozen-suite-v1.eval.json"
BINDING_JSON = SUITE_DIR / "nexus-frozen-suite-v1.binding.json"
MANIFEST = (
    REPO_ROOT
    / "microbrain"
    / "datasets"
    / "manifests"
    / "nexus-synthetic-role-ops-v1.manifest.json"
)

TRAINING_START = "2026-08-10T00:00:00Z"
POLICY = DatasetPolicy()


def _load_suite() -> FrozenEvalSuite:
    return FrozenEvalSuite.from_dict(json.loads(SUITE_JSON.read_text(encoding="utf-8")))


def _example(
    *,
    example_id: str = "m3-ex-1",
    role: Role = Role.INTERPRETATION,
    provenance: DataProvenance = DataProvenance.DETERMINISTIC_GENERATION,
    license_ref: str | None = "nexus-synthetic-mit",
    hard_negative: bool = False,
    ood: OodVerdict = OodVerdict.IN_DISTRIBUTION,
) -> TrainingExample:
    return TrainingExample(
        example_id=example_id,
        role=role,
        input_text="evaluate this",
        control_object={
            "schema_version": "1",
            "intent": "eval.item",
            "route": "DETERMINISTIC",
            "risk": "R0",
        },
        provenance=provenance,
        hard_negative=hard_negative,
        ood_verdict=ood,
        license_ref=license_ref,
    )


def _eval(
    *,
    eval_id: str = "eval-m3-1",
    example: TrainingExample | None = None,
    dimensions: tuple[EvalDimension, ...] = (EvalDimension.INTENT,),
    frozen_at: str | None = "2026-08-01T00:00:00Z",
) -> FrozenEval:
    return FrozenEval(
        eval_id=eval_id,
        kind="FROZEN",
        example=example or _example(),
        dimensions=dimensions,
        created_before_training=True,
        frozen_at=frozen_at,
    )


def _suite(*evals: FrozenEval, created_at: str | None = "2026-08-05T00:00:00Z") -> FrozenEvalSuite:
    return FrozenEvalSuite(suite_id="suite-m3-1", evals=tuple(evals), created_at=created_at)


def _all_pass(suite: FrozenEvalSuite) -> dict[str, list[DimensionResult]]:
    return {
        evaluation.eval_id: [
            DimensionResult(dimension=dimension, passed=True) for dimension in evaluation.dimensions
        ]
        for evaluation in suite.evals
    }


def _secret_canary() -> str:
    """Runtime-constructed secret-shaped canary (no tracked literal)."""
    return "ghp_" + "x" * 35


# ---------------------------------------------------------------------------
# Real fixture loading
# ---------------------------------------------------------------------------


def ep041_unit_m3_suite_fixture_loads() -> None:
    suite = _load_suite()
    assert suite.suite_id == "nexus-frozen-suite-v1"
    assert len(suite.evals) == 11


def ep041_unit_m3_binding_fixture_loads() -> None:
    binding = load_suite_binding(BINDING_JSON)
    assert binding.suite_id == "nexus-frozen-suite-v1"
    assert binding.dataset_id == "nexus-synthetic-role-ops-v1"
    assert binding.dataset_digest.startswith("sha256:")


def ep041_unit_m3_binding_digest_matches_manifest() -> None:
    binding = load_suite_binding(BINDING_JSON)
    assert binding.dataset_digest == sha256_manifest(MANIFEST)


def ep041_unit_m3_binding_malformed_fails_closed(tmp_path) -> None:
    bad = tmp_path / "bad.binding.json"
    bad.write_text("{not json", encoding="utf-8")
    with pytest.raises(MicrobrainError):
        load_suite_binding(bad)


def ep041_unit_m3_binding_missing_field_fails_closed(tmp_path) -> None:
    bad = tmp_path / "missing.binding.json"
    bad.write_text(json.dumps({"suite_id": "s"}), encoding="utf-8")
    with pytest.raises(MicrobrainError) as exc:
        load_suite_binding(bad)
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


# ---------------------------------------------------------------------------
# Suite structural validation
# ---------------------------------------------------------------------------


def ep041_unit_m3_real_suite_validates() -> None:
    verdict = validate_suite(_load_suite())
    assert verdict.valid is True
    assert verdict.eval_count == 11


def ep041_unit_m3_empty_suite_rejected() -> None:
    with pytest.raises(MicrobrainError) as exc:
        _suite()
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


def ep041_unit_m3_duplicate_eval_ids_rejected() -> None:
    suite = _suite(
        _eval(eval_id="dup"), _eval(eval_id="dup", example=_example(example_id="m3-ex-2"))
    )
    verdict = validate_suite(suite)
    assert verdict.valid is False
    assert any("duplicate eval ids" in reason for reason in verdict.reasons)


def ep041_unit_m3_duplicate_example_ids_rejected() -> None:
    suite = _suite(_eval(), _eval(eval_id="eval-m3-2"))
    verdict = validate_suite(suite)
    assert verdict.valid is False
    assert any("duplicate example ids" in reason for reason in verdict.reasons)


def ep041_unit_m3_missing_frozen_at_rejected() -> None:
    suite = _suite(_eval(frozen_at=None))
    verdict = validate_suite(suite)
    assert verdict.valid is False
    assert any("missing frozen_at" in reason for reason in verdict.reasons)


def ep041_unit_m3_missing_suite_created_at_rejected() -> None:
    suite = _suite(_eval(), created_at=None)
    verdict = validate_suite(suite)
    assert verdict.valid is False
    assert any("created_at" in reason for reason in verdict.reasons)


def ep041_unit_m3_missing_dimensions_rejected_at_contract() -> None:
    with pytest.raises(MicrobrainError) as exc:
        _eval(dimensions=())
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


# ---------------------------------------------------------------------------
# Eval-before-training timing
# ---------------------------------------------------------------------------


def ep041_unit_m3_eval_before_training_eligible() -> None:
    verdict = check_eval_before_training(_load_suite(), TRAINING_START)
    assert verdict.valid is True


def ep041_unit_m3_eval_at_training_start_rejected() -> None:
    verdict = check_eval_before_training(_load_suite(), "2026-08-05T00:00:00Z")
    assert verdict.valid is False
    assert any("strictly before" in reason for reason in verdict.reasons)


def ep041_unit_m3_eval_after_training_rejected() -> None:
    suite = _suite(_eval(frozen_at="2026-08-11T00:00:00Z"))
    verdict = check_eval_before_training(suite, TRAINING_START)
    assert verdict.valid is False


# ---------------------------------------------------------------------------
# Suite immutability (digest binding)
# ---------------------------------------------------------------------------


def ep041_unit_m3_suite_digest_is_deterministic() -> None:
    suite = _load_suite()
    assert suite_digest(suite) == suite_digest(suite)


def ep041_unit_m3_suite_digest_matches() -> None:
    suite = _load_suite()
    verdict = verify_suite_digest(suite, suite_digest(suite))
    assert verdict.valid is True


def ep041_unit_m3_suite_digest_tamper_rejected() -> None:
    suite = _load_suite()
    data = suite.to_dict()
    data["evals"][0]["example"]["input_text"] = "tampered"
    tampered = FrozenEvalSuite.from_dict(data)
    verdict = verify_suite_digest(tampered, suite_digest(suite))
    assert verdict.valid is False
    assert any("digest mismatch" in reason for reason in verdict.reasons)


# ---------------------------------------------------------------------------
# Dataset policy binding
# ---------------------------------------------------------------------------


def ep041_unit_m3_real_suite_binds_to_real_dataset() -> None:
    suite = _load_suite()
    binding = load_suite_binding(BINDING_JSON)
    dataset = load_manifest(MANIFEST)
    verdict = bind_suite_to_dataset(suite, binding, dataset, sha256_manifest(MANIFEST), POLICY)
    assert verdict.bound is True


def ep041_unit_m3_binding_suite_id_mismatch_rejected() -> None:
    suite = _load_suite()
    binding = load_suite_binding(BINDING_JSON)
    dataset = load_manifest(MANIFEST)
    wrong = SuiteBinding(
        suite_id="other-suite",
        dataset_id=binding.dataset_id,
        dataset_digest=binding.dataset_digest,
    )
    verdict = bind_suite_to_dataset(suite, wrong, dataset, sha256_manifest(MANIFEST), POLICY)
    assert verdict.bound is False


def ep041_unit_m3_unknown_dataset_rejected() -> None:
    suite = _load_suite()
    binding = load_suite_binding(BINDING_JSON)
    dataset = load_manifest(MANIFEST)
    wrong = SuiteBinding(
        suite_id=binding.suite_id,
        dataset_id="unknown-dataset",
        dataset_digest=binding.dataset_digest,
    )
    verdict = bind_suite_to_dataset(suite, wrong, dataset, sha256_manifest(MANIFEST), POLICY)
    assert verdict.bound is False
    assert any("UNKNOWN DATASET" in reason for reason in verdict.reasons)


def ep041_unit_m3_dataset_digest_mismatch_rejected() -> None:
    suite = _load_suite()
    binding = load_suite_binding(BINDING_JSON)
    dataset = load_manifest(MANIFEST)
    verdict = bind_suite_to_dataset(
        suite,
        binding,
        dataset,
        "sha256:" + "0" * 64,
        POLICY,
    )
    assert verdict.bound is False
    assert any("digest mismatch" in reason for reason in verdict.reasons)


def ep041_unit_m3_dataset_policy_denied_rejected() -> None:
    suite = _suite(_eval())
    binding = load_suite_binding(BINDING_JSON)
    dataset = MicrobrainDataset(
        dataset_id="nexus-synthetic-role-ops-v1",
        name="unlicensed",
        lineage="lineage",
        examples=(_example(license_ref=None),),
    )
    verdict = bind_suite_to_dataset(
        suite,
        binding,
        dataset,
        sha256_manifest(MANIFEST),
        POLICY,
    )
    assert verdict.bound is False
    assert any("dataset policy not passed" in reason for reason in verdict.reasons)


def ep041_unit_m3_eval_with_m2_denied_teacher_data_rejected() -> None:
    teacher_example = _example(
        example_id="m3-ex-teacher",
        provenance=DataProvenance.TEACHER_CONSENSUS,
        license_ref="cc-by-nc-4.0",
    )
    suite = _suite(_eval(eval_id="eval-teacher", example=teacher_example))
    binding = load_suite_binding(BINDING_JSON)
    dataset = load_manifest(MANIFEST)
    verdict = bind_suite_to_dataset(suite, binding, dataset, sha256_manifest(MANIFEST), POLICY)
    assert verdict.bound is False
    assert any("dataset policy would deny" in reason for reason in verdict.reasons)


# ---------------------------------------------------------------------------
# Deterministic scoring
# ---------------------------------------------------------------------------


def ep041_unit_m3_all_pass_scoring() -> None:
    evaluation = _eval(dimensions=(EvalDimension.INTENT, EvalDimension.EXACT_SCHEMA))
    summary = score_eval(
        evaluation,
        [
            DimensionResult(EvalDimension.INTENT, True),
            DimensionResult(EvalDimension.EXACT_SCHEMA, True),
        ],
    )
    assert summary.passed is True
    assert len(summary.dimension_results) == 2


def ep041_unit_m3_any_fail_blocks() -> None:
    evaluation = _eval(dimensions=(EvalDimension.INTENT, EvalDimension.EXACT_SCHEMA))
    summary = score_eval(
        evaluation,
        [
            DimensionResult(EvalDimension.INTENT, True),
            DimensionResult(EvalDimension.EXACT_SCHEMA, False, "schema drift"),
        ],
    )
    assert summary.passed is False


def ep041_unit_m3_missing_dimension_fails_closed() -> None:
    evaluation = _eval(dimensions=(EvalDimension.INTENT, EvalDimension.EXACT_SCHEMA))
    summary = score_eval(
        evaluation,
        [DimensionResult(EvalDimension.INTENT, True)],
    )
    assert summary.passed is False
    assert any("missing dimension" in reason for reason in summary.reasons)


def ep041_unit_m3_unknown_dimension_fails_closed() -> None:
    evaluation = _eval(dimensions=(EvalDimension.INTENT,))
    summary = score_eval(
        evaluation,
        [
            DimensionResult(EvalDimension.INTENT, True),
            DimensionResult(EvalDimension.LATENCY, True),
        ],
    )
    assert summary.passed is False
    assert any("unknown dimension" in reason for reason in summary.reasons)


def ep041_unit_m3_duplicate_dimension_fails_closed() -> None:
    evaluation = _eval(dimensions=(EvalDimension.INTENT,))
    summary = score_eval(
        evaluation,
        [
            DimensionResult(EvalDimension.INTENT, True),
            DimensionResult(EvalDimension.INTENT, True),
        ],
    )
    assert summary.passed is False
    assert any("duplicate dimension" in reason for reason in summary.reasons)


def ep041_unit_m3_scoring_is_deterministic() -> None:
    evaluation = _eval(dimensions=(EvalDimension.INTENT, EvalDimension.EXACT_SCHEMA))
    results = [
        DimensionResult(EvalDimension.INTENT, True),
        DimensionResult(EvalDimension.EXACT_SCHEMA, False, "drift"),
    ]
    assert score_eval(evaluation, results) == score_eval(evaluation, results)


def ep041_unit_m3_ood_item_without_ood_dimension_fails_closed() -> None:
    evaluation = _eval(
        example=_example(ood=OodVerdict.OUT_OF_DISTRIBUTION),
        dimensions=(EvalDimension.INTENT,),
    )
    summary = score_eval(
        evaluation,
        [DimensionResult(EvalDimension.INTENT, True)],
    )
    assert summary.passed is False
    assert any("OUT_OF_DISTRIBUTION_ESCALATION" in reason for reason in summary.reasons)


def ep041_unit_m3_ood_escalation_failed_blocks() -> None:
    evaluation = _eval(
        example=_example(ood=OodVerdict.OUT_OF_DISTRIBUTION),
        dimensions=(
            EvalDimension.INTENT,
            EvalDimension.OUT_OF_DISTRIBUTION_ESCALATION,
        ),
    )
    summary = score_eval(
        evaluation,
        [
            DimensionResult(EvalDimension.INTENT, True),
            DimensionResult(EvalDimension.OUT_OF_DISTRIBUTION_ESCALATION, False),
        ],
    )
    assert summary.passed is False
    assert any("unsafe OOD" in reason for reason in summary.reasons)


def ep041_unit_m3_hard_negative_without_injection_fails_closed() -> None:
    evaluation = _eval(
        example=_example(
            provenance=DataProvenance.HARD_NEGATIVE,
            hard_negative=True,
        ),
        dimensions=(EvalDimension.INTENT,),
    )
    summary = score_eval(
        evaluation,
        [DimensionResult(EvalDimension.INTENT, True)],
    )
    assert summary.passed is False
    assert any("INJECTION_RESISTANCE" in reason for reason in summary.reasons)


def ep041_unit_m3_hard_negative_failed_injection_blocks() -> None:
    evaluation = _eval(
        example=_example(
            provenance=DataProvenance.HARD_NEGATIVE,
            hard_negative=True,
        ),
        dimensions=(EvalDimension.INTENT, EvalDimension.INJECTION_RESISTANCE),
    )
    summary = score_eval(
        evaluation,
        [
            DimensionResult(EvalDimension.INTENT, True),
            DimensionResult(EvalDimension.INJECTION_RESISTANCE, False),
        ],
    )
    assert summary.passed is False
    assert any("hard-negative" in reason for reason in summary.reasons)


def ep041_unit_m3_real_suite_scores_pass_with_all_results() -> None:
    suite = _load_suite()
    summary = score_suite(suite, _all_pass(suite))
    assert summary.passed is True
    assert len(summary.per_eval) == 11


def ep041_unit_m3_suite_score_fails_when_one_eval_fails() -> None:
    suite = _load_suite()
    results = _all_pass(suite)
    results["eval-risk-001"][0] = DimensionResult(EvalDimension.RISK, False)
    summary = score_suite(suite, results)
    assert summary.passed is False


def ep041_unit_m3_suite_score_missing_eval_blocks() -> None:
    suite = _load_suite()
    results = _all_pass(suite)
    del results["eval-quoted-001"]
    summary = score_suite(suite, results)
    assert summary.passed is False
    assert any("__missing__" in item.eval_id for item in summary.per_eval)


# ---------------------------------------------------------------------------
# Evidence
# ---------------------------------------------------------------------------


def ep041_unit_m3_evidence_binds_current_run() -> None:
    suite = _load_suite()
    score = score_suite(suite, _all_pass(suite))
    evidence = build_eval_evidence(
        run_id="run-041-m3-1",
        git_commit="abc123",
        suite=suite,
        dataset_id="nexus-synthetic-role-ops-v1",
        dataset_digest=sha256_manifest(MANIFEST),
        suite_score=score,
        candidate_id="cand-routing-v1",
        role="ROUTING",
        timestamp="2026-08-24T00:00:00Z",
    )
    payload = evidence.to_dict()
    assert payload["run_id"] == "run-041-m3-1"
    assert payload["git_commit"] == "abc123"
    assert payload["dataset_digest"].startswith("sha256:")
    assert payload["decision"] == "PASS"
    assert payload["role"] == "ROUTING"


def ep041_unit_m3_evidence_decision_blocks_on_failure() -> None:
    suite = _load_suite()
    results = _all_pass(suite)
    results["eval-risk-001"][0] = DimensionResult(EvalDimension.RISK, False)
    score = score_suite(suite, results)
    evidence = build_eval_evidence(
        run_id="run-041-m3-2",
        git_commit="abc123",
        suite=suite,
        dataset_id="nexus-synthetic-role-ops-v1",
        dataset_digest=sha256_manifest(MANIFEST),
        suite_score=score,
    )
    assert evidence.to_dict()["decision"] == "BLOCK"


def ep041_unit_m3_evidence_redaction_scrubs_canary() -> None:
    canary = _secret_canary()
    suite = _load_suite()
    score = score_suite(suite, _all_pass(suite))
    evidence = build_eval_evidence(
        run_id=canary,
        git_commit="abc123",
        suite=suite,
        dataset_id="nexus-synthetic-role-ops-v1",
        dataset_digest=sha256_manifest(MANIFEST),
        suite_score=score,
    )
    payload = evidence.to_redacted_dict()
    assert canary not in json.dumps(payload)
    assert "[REDACTED]" in payload["run_id"]


# ---------------------------------------------------------------------------
# Composition: real fixture full journey
# ---------------------------------------------------------------------------


def ep041_unit_m3_real_fixture_full_journey() -> None:
    suite = _load_suite()
    binding = load_suite_binding(BINDING_JSON)
    dataset = load_manifest(MANIFEST)
    digest = sha256_manifest(MANIFEST)

    assert validate_suite(suite).valid is True
    assert check_eval_before_training(suite, TRAINING_START).valid is True
    assert verify_suite_digest(suite, suite_digest(suite)).valid is True
    assert bind_suite_to_dataset(suite, binding, dataset, digest, POLICY).bound is True

    score = score_suite(suite, _all_pass(suite))
    assert score.passed is True

    evidence = build_eval_evidence(
        run_id="run-041-m3-journey",
        git_commit="abc123",
        suite=suite,
        dataset_id=dataset.dataset_id,
        dataset_digest=digest,
        suite_score=score,
    )
    assert evidence.to_dict()["decision"] == "PASS"
