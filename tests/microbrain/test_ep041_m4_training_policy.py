"""EP-041 M4 unit tests: training candidate eligibility and plan behavior.

Test names begin with ep041_unit_ per the EP-041 milestone contract.
Every test exercises the real M4 behavior
(python/nexus_microbrain/training_policy.py) against the real M1
contract, the real M2 dataset policy, the real M3 eval behavior, and
the real committed training fixtures under microbrain/training/plans/.
TRAINING CANDIDATE EXISTS != ELIGIBLE TO TRAIN; TRAINING PLAN EXISTS !=
TRAINING EXECUTED; QLORA RUN EXISTS != TRAINING CERTIFIED.
"""

from __future__ import annotations

import json

import pytest
from nexus_microbrain import (
    MICROBRAIN_CODE_INVALID_INPUT,
    MICROBRAIN_CODE_MISSING_REQUIRED,
    MICROBRAIN_CODE_UNKNOWN_VOCABULARY,
    CandidateStatus,
    DataProvenance,
    DatasetPolicy,
    DimensionResult,
    EvalDimension,
    FrozenEval,
    FrozenEvalSuite,
    MicrobrainDataset,
    MicrobrainError,
    OodVerdict,
    QloraRun,
    QloraStatus,
    QuantizationFormat,
    Role,
    SuiteBinding,
    TrainingCandidate,
    TrainingExample,
    TrainingPlan,
    build_training_evidence,
    build_training_plan,
    check_candidate_eligibility,
    check_no_eval_leakage,
    load_manifest,
    load_suite_binding,
    narrow_role,
    plan_digest,
    qlora_run_verdict,
    score_suite,
    sha256_manifest,
    suite_digest_of,
    training_plan_verdict,
    verify_plan_digest,
)

REPO_ROOT = __import__("pathlib").Path(__file__).resolve().parents[2]
CANDIDATE_JSON = (
    REPO_ROOT / "microbrain" / "training" / "plans" / "nexus-candidate-v1.candidate.json"
)
PLAN_JSON = REPO_ROOT / "microbrain" / "training" / "plans" / "nexus-training-plan-v1.plan.json"
SUITE_JSON = REPO_ROOT / "microbrain" / "evals" / "suites" / "nexus-frozen-suite-v1.eval.json"
BINDING_JSON = REPO_ROOT / "microbrain" / "evals" / "suites" / "nexus-frozen-suite-v1.binding.json"
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


def _load_binding() -> SuiteBinding:
    return load_suite_binding(BINDING_JSON)


def _load_dataset() -> MicrobrainDataset:
    return load_manifest(MANIFEST)


def _load_candidate() -> TrainingCandidate:
    return TrainingCandidate.from_dict(json.loads(CANDIDATE_JSON.read_text(encoding="utf-8")))


def _load_plan() -> TrainingPlan:
    return TrainingPlan.from_dict(json.loads(PLAN_JSON.read_text(encoding="utf-8")))


def _example(
    *,
    example_id: str = "m4-ex-1",
    role: Role = Role.INTERPRETATION,
    provenance: DataProvenance = DataProvenance.DETERMINISTIC_GENERATION,
    license_ref: str | None = "nexus-synthetic-mit",
    hard_negative: bool = False,
    ood: OodVerdict = OodVerdict.IN_DISTRIBUTION,
    correlation_id: str | None = None,
) -> TrainingExample:
    return TrainingExample(
        example_id=example_id,
        role=role,
        input_text="train this",
        control_object={
            "schema_version": "1",
            "intent": "train.item",
            "route": "DETERMINISTIC",
            "risk": "R0",
        },
        provenance=provenance,
        hard_negative=hard_negative,
        ood_verdict=ood,
        license_ref=license_ref,
        correlation_id=correlation_id,
    )


def _eval(
    *,
    eval_id: str = "eval-m4-1",
    example: TrainingExample | None = None,
    dimensions: tuple[EvalDimension, ...] = (EvalDimension.INTENT,),
    frozen_at: str | None = "2026-08-01T00:00:00Z",
) -> FrozenEval:
    return FrozenEval(
        eval_id=eval_id,
        kind="FROZEN",
        example=example or _example(example_id="m4-eval-ex-1"),
        dimensions=dimensions,
        created_before_training=True,
        frozen_at=frozen_at,
    )


def _suite(*evals: FrozenEval, created_at: str | None = "2026-08-05T00:00:00Z") -> FrozenEvalSuite:
    return FrozenEvalSuite(suite_id="suite-m4-1", evals=tuple(evals), created_at=created_at)


def _all_pass(suite: FrozenEvalSuite) -> dict[str, list[DimensionResult]]:
    return {
        evaluation.eval_id: [
            DimensionResult(dimension=dimension, passed=True) for dimension in evaluation.dimensions
        ]
        for evaluation in suite.evals
    }


def _candidate(
    *,
    candidate_id: str = "cand-m4-1",
    role: Role = Role.INTERPRETATION,
    dataset_ref: str = "nexus-synthetic-role-ops-v1",
    model_ref: str = "model-m4-1",
    base_model: str = "deepseek-v3-base",
) -> TrainingCandidate:
    return TrainingCandidate(
        candidate_id=candidate_id,
        role=role,
        model_ref=model_ref,
        base_model=base_model,
        dataset_ref=dataset_ref,
        status=CandidateStatus.CANDIDATE,
    )


def _secret_canary() -> str:
    """Runtime-constructed secret-shaped canary (no tracked literal)."""
    return "ghp_" + "x" * 35


# ---------------------------------------------------------------------------
# Real fixture loading
# ---------------------------------------------------------------------------


def ep041_unit_m4_candidate_fixture_loads() -> None:
    candidate = _load_candidate()
    assert candidate.candidate_id == "nexus-candidate-v1"
    assert candidate.role is Role.INTERPRETATION
    assert candidate.dataset_ref == "nexus-synthetic-role-ops-v1"
    assert candidate.status is CandidateStatus.CANDIDATE


def ep041_unit_m4_plan_fixture_loads_and_digest_matches() -> None:
    plan = _load_plan()
    assert plan.plan_id == "nexus-training-plan-v1"
    assert plan.quantization_format is QuantizationFormat.GGUF
    assert plan.plan_digest == plan_digest(plan)


def ep041_unit_m4_plan_fixture_verifies_digest() -> None:
    plan = _load_plan()
    verdict = verify_plan_digest(plan, plan.plan_digest)
    assert verdict.state == "PLAN_READY"
    assert verdict.digest_verified


# ---------------------------------------------------------------------------
# Candidate eligibility: real full journey
# ---------------------------------------------------------------------------


def ep041_unit_m4_candidate_real_full_journey_eligible() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    dataset_digest = sha256_manifest(MANIFEST)
    suite = _load_suite()
    binding = _load_binding()
    assert binding.dataset_digest == dataset_digest
    results = _all_pass(suite)
    score = score_suite(suite, results)
    assert score.passed

    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=dataset_digest,
        suite=suite,
        binding=binding,
        suite_score=score,
        training_started_at=TRAINING_START,
        policy=POLICY,
    )
    assert verdict.eligible
    assert verdict.role == "INTERPRETATION"
    assert verdict.dataset_id == "nexus-synthetic-role-ops-v1"
    assert verdict.eval_suite_id == "nexus-frozen-suite-v1"


def ep041_unit_m4_candidate_real_dataset_policy_gates() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    dataset_digest = sha256_manifest(MANIFEST)
    suite = _load_suite()
    binding = _load_binding()
    score = score_suite(suite, _all_pass(suite))
    # Policy denied via custom prohibited set.
    policy = DatasetPolicy(prohibited_license_refs=frozenset({"nexus-synthetic-mit"}))
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=dataset_digest,
        suite=suite,
        binding=binding,
        suite_score=score,
        training_started_at=TRAINING_START,
        policy=policy,
    )
    assert not verdict.eligible
    assert any("dataset policy not passed" in reason for reason in verdict.reasons)


# ---------------------------------------------------------------------------
# Candidate eligibility: fail-closed negatives
# ---------------------------------------------------------------------------


def ep041_unit_m4_missing_dataset_reference_denied() -> None:
    candidate = _candidate(dataset_ref="")
    dataset = _load_dataset()
    suite = _load_suite()
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=_load_binding(),
        suite_score=score_suite(suite, _all_pass(suite)),
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("missing dataset reference" in reason for reason in verdict.reasons)


def ep041_unit_m4_unknown_dataset_reference_denied() -> None:
    candidate = _candidate(dataset_ref="nexus-unknown-dataset-v9")
    dataset = _load_dataset()
    suite = _load_suite()
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=_load_binding(),
        suite_score=score_suite(suite, _all_pass(suite)),
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("unknown dataset reference" in reason for reason in verdict.reasons)


def ep041_unit_m4_dataset_digest_mismatch_denied() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    suite = _load_suite()
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest="sha256:" + "0" * 64,
        suite=suite,
        binding=_load_binding(),
        suite_score=score_suite(suite, _all_pass(suite)),
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("binding dataset digest mismatch" in reason for reason in verdict.reasons)


def ep041_unit_m4_dataset_digest_malformed_denied() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    suite = _load_suite()
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest="not-a-digest",
        suite=suite,
        binding=_load_binding(),
        suite_score=score_suite(suite, _all_pass(suite)),
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("dataset digest missing or malformed" in reason for reason in verdict.reasons)


def ep041_unit_m4_missing_eval_reference_denied() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=None,
        binding=_load_binding(),
        suite_score=None,
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("missing eval reference" in reason for reason in verdict.reasons)


def ep041_unit_m4_missing_eval_binding_denied() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    suite = _load_suite()
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=None,
        suite_score=score_suite(suite, _all_pass(suite)),
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("missing eval suite binding" in reason for reason in verdict.reasons)


def ep041_unit_m4_eval_not_frozen_before_training_denied() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    suite = _load_suite()
    # A suite whose frozen_at is at training start fails the timing gate.
    late_suite = FrozenEvalSuite(
        suite_id="suite-m4-late",
        evals=tuple(
            FrozenEval(
                eval_id=e.eval_id,
                kind="FROZEN",
                example=e.example,
                dimensions=e.dimensions,
                created_before_training=True,
                frozen_at="2026-08-10T00:00:00Z",
            )
            for e in suite.evals
        ),
        created_at=suite.created_at,
    )
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=late_suite,
        binding=_load_binding(),
        suite_score=score_suite(late_suite, _all_pass(late_suite)),
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("eval not frozen before training" in reason for reason in verdict.reasons)


def ep041_unit_m4_eval_policy_fail_denied() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    suite = _load_suite()
    # One failing dimension makes the suite score fail (eval policy fail).
    results = _all_pass(suite)
    first = suite.evals[0]
    results[first.eval_id][0] = DimensionResult(
        dimension=first.dimensions[0], passed=False, detail="observed failure"
    )
    score = score_suite(suite, results)
    assert not score.passed
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=_load_binding(),
        suite_score=score,
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("eval policy not passed" in reason for reason in verdict.reasons)


def ep041_unit_m4_missing_eval_score_denied() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    suite = _load_suite()
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=_load_binding(),
        suite_score=None,
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("missing eval score" in reason for reason in verdict.reasons)


def ep041_unit_m4_role_unknown_denied() -> None:
    # The M1 contract already fails closed on a freeform role string.
    with pytest.raises(MicrobrainError) as exc:
        _candidate(role=Role.parse("GENERAL_ASSISTANT"))  # type: ignore[arg-type]
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_m4_role_too_broad_denied() -> None:
    # Every canonical role is narrow; a freeform broad string is denied
    # by the deny-unknown vocabulary (role expansion by description).
    for broad in ("universal agent", "all tasks", "broad autonomous controller"):
        with pytest.raises(MicrobrainError):
            Role.parse(broad)


def ep041_unit_m4_narrow_role_holds() -> None:
    assert narrow_role(Role.INTERPRETATION)
    assert narrow_role(Role.ESCALATION)
    assert len(set(Role)) == 8
    assert len({r.value for r in Role}) == 8


def ep041_unit_m4_missing_model_identity_denied() -> None:
    # The M1 contract fails closed at construction for a candidate
    # without model identity, so the M4 eligibility gate can never see
    # an unprovenanced candidate (provenance proof required).
    with pytest.raises(MicrobrainError) as exc:
        TrainingCandidate(
            candidate_id="cand-m4-noid",
            role=Role.INTERPRETATION,
            model_ref="",
            base_model="",
            dataset_ref="nexus-synthetic-role-ops-v1",
            status=CandidateStatus.CANDIDATE,
        )
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_m4_candidate_redacted_verdict_no_secret_leak() -> None:
    candidate = _candidate(dataset_ref="")
    dataset = _load_dataset()
    suite = _load_suite()
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=_load_binding(),
        suite_score=score_suite(suite, _all_pass(suite)),
        training_started_at=TRAINING_START,
    )
    payload = json.dumps(verdict.to_redacted_dict())
    assert _secret_canary() not in payload


# ---------------------------------------------------------------------------
# Training plan behavior (plan exists != training executed)
# ---------------------------------------------------------------------------


def ep041_unit_m4_plan_ready_only_never_executed() -> None:
    plan = _load_plan()
    verdict = training_plan_verdict(plan)
    assert verdict.state == "PLAN_READY"
    # A plan verdict must never contain QLoRA success / artifact /
    # promotion language.
    assert "QLoRA" not in json.dumps(verdict.to_dict())
    assert "artifact" not in json.dumps(verdict.to_dict()).lower()
    assert "promot" not in json.dumps(verdict.to_dict()).lower()


def ep041_unit_m4_plan_digest_deterministic() -> None:
    plan = _load_plan()
    first = plan_digest(plan)
    second = plan_digest(plan)
    assert first == second
    assert first == plan.plan_digest


def ep041_unit_m4_plan_tamper_denied() -> None:
    plan = _load_plan()
    verdict = verify_plan_digest(plan, "sha256:" + "f" * 64)
    assert verdict.state == "DENIED"
    assert not verdict.digest_verified


def ep041_unit_m4_missing_hyperparameters_rejected() -> None:
    with pytest.raises(MicrobrainError) as exc:
        build_training_plan(
            plan_id="plan-m4-1",
            candidate_ref="cand-m4-1",
            role=Role.INTERPRETATION,
            base_model="deepseek-v3-base",
            quantization_format=QuantizationFormat.GGUF,
            hyperparameters={"rank": 16},
            created_at="2026-08-11T00:00:00Z",
        )
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


def ep041_unit_m4_unknown_quantization_format_rejected() -> None:
    with pytest.raises(MicrobrainError) as exc:
        build_training_plan(
            plan_id="plan-m4-1",
            candidate_ref="cand-m4-1",
            role=Role.INTERPRETATION,
            base_model="deepseek-v3-base",
            quantization_format=QuantizationFormat.parse("SAFETENSORS"),  # type: ignore[arg-type]
            hyperparameters={"rank": 16, "alpha": 32, "seed": 7},
            created_at="2026-08-11T00:00:00Z",
        )
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_m4_non_gguf_quantization_rejected() -> None:
    # GGUF is the only locked QuantizationFormat; construction via the
    # contract already fails closed for any other value.
    with pytest.raises(MicrobrainError):
        QuantizationFormat.parse("ONNX")


def ep041_unit_m4_unsupported_target_role_rejected() -> None:
    # A freeform/unknown role string is rejected at the contract
    # boundary (deny-unknown vocabulary), so a TrainingPlan can never
    # carry a role outside the 8 canonical narrow roles.
    raw = {
        "schema_version": "1",
        "plan_id": "plan-m4-bad-role",
        "candidate_ref": "cand-m4-1",
        "role": "GENERAL_ASSISTANT",
        "base_model": "deepseek-v3-base",
        "quantization_format": "GGUF",
        "hyperparameters": {"rank": 16, "alpha": 32, "seed": 7},
        "created_at": "2026-08-11T00:00:00Z",
        "plan_digest": "sha256:" + "0" * 64,
    }
    with pytest.raises(MicrobrainError) as exc:
        TrainingPlan.from_dict(raw)
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_m4_plan_does_not_create_run_or_promotion() -> None:
    # Building a plan must not mutate any QLoRA status or create a
    # promotion decision - the plan verdict is evidence of a plan only.
    plan = _load_plan()
    verdict = training_plan_verdict(plan)
    assert verdict.state == "PLAN_READY"
    assert plan.plan_digest == plan_digest(plan)


# ---------------------------------------------------------------------------
# QLoRA run contract honesty (declared != executed)
# ---------------------------------------------------------------------------


def ep041_unit_m4_qlora_declared_not_executed() -> None:
    run = QloraRun(
        run_id="run-m4-1",
        candidate_ref="cand-m4-1",
        adapter="adapter-m4-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.PENDING,
    )
    verdict = qlora_run_verdict(run, candidate_eligible=True)
    assert not verdict.certified
    assert any("declared" in reason for reason in verdict.reasons)


def ep041_unit_m4_qlora_missing_evidence_not_certified() -> None:
    run = QloraRun(
        run_id="run-m4-2",
        candidate_ref="cand-m4-1",
        adapter="adapter-m4-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.COMPLETED,
    )
    verdict = qlora_run_verdict(run, candidate_eligible=True)
    assert not verdict.certified
    assert any("missing start evidence" in reason for reason in verdict.reasons)
    assert any("missing end evidence" in reason for reason in verdict.reasons)
    assert any("training output digest missing" in reason for reason in verdict.reasons)


def ep041_unit_m4_qlora_failed_not_certified() -> None:
    run = QloraRun(
        run_id="run-m4-3",
        candidate_ref="cand-m4-1",
        adapter="adapter-m4-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.FAILED,
    )
    verdict = qlora_run_verdict(
        run,
        candidate_eligible=True,
        start_evidence="start",
        end_evidence="end",
        output_digest="sha256:" + "b" * 64,
    )
    assert not verdict.certified
    assert any("failed run not certified" in reason for reason in verdict.reasons)


def ep041_unit_m4_qlora_unknown_status_fails_closed() -> None:
    with pytest.raises(MicrobrainError) as exc:
        QloraStatus.parse("MYSTERY")
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_m4_qlora_after_invalid_candidate_denied() -> None:
    run = QloraRun(
        run_id="run-m4-4",
        candidate_ref="cand-m4-1",
        adapter="adapter-m4-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.COMPLETED,
    )
    verdict = qlora_run_verdict(
        run,
        candidate_eligible=False,
        start_evidence="start",
        end_evidence="end",
        output_digest="sha256:" + "b" * 64,
    )
    assert not verdict.certified
    assert any("run after invalid candidate denied" in reason for reason in verdict.reasons)


def ep041_unit_m4_qlora_output_digest_missing_denied() -> None:
    run = QloraRun(
        run_id="run-m4-5",
        candidate_ref="cand-m4-1",
        adapter="adapter-m4-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.COMPLETED,
    )
    verdict = qlora_run_verdict(
        run,
        candidate_eligible=True,
        start_evidence="start",
        end_evidence="end",
        output_digest=None,
    )
    assert not verdict.certified
    assert any("training output digest missing" in reason for reason in verdict.reasons)


def ep041_unit_m4_qlora_output_digest_malformed_denied() -> None:
    run = QloraRun(
        run_id="run-m4-6",
        candidate_ref="cand-m4-1",
        adapter="adapter-m4-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.COMPLETED,
    )
    verdict = qlora_run_verdict(
        run,
        candidate_eligible=True,
        start_evidence="start",
        end_evidence="end",
        output_digest="short",
    )
    assert not verdict.certified
    assert any("malformed" in reason for reason in verdict.reasons)


def ep041_unit_m4_qlora_metrics_alone_never_certify() -> None:
    # Training metrics alone are never enough: without start/end
    # evidence and an output digest, the run is not certified.
    run = QloraRun(
        run_id="run-m4-7",
        candidate_ref="cand-m4-1",
        adapter="adapter-m4-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.COMPLETED,
    )
    verdict = qlora_run_verdict(
        run,
        candidate_eligible=True,
        start_evidence="start",
        end_evidence="end",
        output_digest="sha256:" + "c" * 64,
    )
    assert verdict.certified
    # But a metrics-only story (no digest) must not certify.
    verdict2 = qlora_run_verdict(run, candidate_eligible=True)
    assert not verdict2.certified


def ep041_unit_m4_qlora_full_evidence_certifies_contract() -> None:
    run = QloraRun(
        run_id="run-m4-8",
        candidate_ref="cand-m4-1",
        adapter="adapter-m4-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.COMPLETED,
    )
    verdict = qlora_run_verdict(
        run,
        candidate_eligible=True,
        start_evidence="2026-08-11T00:00:00Z",
        end_evidence="2026-08-11T01:00:00Z",
        output_digest="sha256:" + "d" * 64,
    )
    assert verdict.certified
    assert verdict.status == "COMPLETED"


# ---------------------------------------------------------------------------
# Train/eval leakage checks
# ---------------------------------------------------------------------------


def ep041_unit_m4_leakage_clean_real_fixtures() -> None:
    suite = _load_suite()
    dataset = _load_dataset()
    verdict = check_no_eval_leakage(suite, dataset)
    assert verdict.checked
    assert verdict.clean
    assert not verdict.reasons


def ep041_unit_m4_leakage_eval_reuse_denied() -> None:
    # Reuse an eval example id as a training example -> denied.
    suite = _load_suite()
    dataset = _load_dataset()
    reused = suite.evals[0].example.example_id
    leaked = MicrobrainDataset(
        dataset_id="dataset-m4-leak",
        name="leak",
        lineage="leak-test",
        examples=tuple(dataset.examples) + (_example(example_id=reused),),
    )
    verdict = check_no_eval_leakage(suite, leaked)
    assert verdict.checked
    assert not verdict.clean
    assert any("eval item reused as training example" in reason for reason in verdict.reasons)


def ep041_unit_m4_leakage_correlation_collision_denied() -> None:
    dataset = _load_dataset()
    # Give a training example the same correlation_id as an eval example.
    corr_id = "corr-m4-shared"
    eval_example = _example(example_id="m4-eval-corr", correlation_id=corr_id)
    eval_item = FrozenEval(
        eval_id="eval-m4-corr",
        kind="FROZEN",
        example=eval_example,
        dimensions=(EvalDimension.INTENT,),
        created_before_training=True,
        frozen_at="2026-08-01T00:00:00Z",
    )
    leaked_suite = _suite(eval_item)
    leaked = MicrobrainDataset(
        dataset_id="dataset-m4-leak2",
        name="leak2",
        lineage="leak-test",
        examples=tuple(dataset.examples)
        + (_example(example_id="m4-train-corr", correlation_id=corr_id),),
    )
    verdict = check_no_eval_leakage(leaked_suite, leaked)
    assert verdict.checked
    assert not verdict.clean
    assert any("correlation_id" in reason for reason in verdict.reasons)


def ep041_unit_m4_leakage_digest_collision_denied() -> None:
    dataset = _load_dataset()
    # Identical example content in both sets -> digest collision.
    shared = _example(example_id="m4-shared-content")
    eval_item = FrozenEval(
        eval_id="eval-m4-digest",
        kind="FROZEN",
        example=shared,
        dimensions=(EvalDimension.INTENT,),
        created_before_training=True,
        frozen_at="2026-08-01T00:00:00Z",
    )
    leaked_suite = _suite(eval_item)
    leaked = MicrobrainDataset(
        dataset_id="dataset-m4-leak3",
        name="leak3",
        lineage="leak-test",
        examples=tuple(dataset.examples) + (shared,),
    )
    verdict = check_no_eval_leakage(leaked_suite, leaked)
    assert verdict.checked
    assert not verdict.clean
    assert any("digest" in reason for reason in verdict.reasons)


def ep041_unit_m4_leakage_missing_evidence_fails_closed() -> None:
    suite = _load_suite()
    verdict = check_no_eval_leakage(suite, None)
    assert not verdict.checked
    assert not verdict.clean
    assert any("missing leakage evidence" in reason for reason in verdict.reasons)
    verdict2 = check_no_eval_leakage(None, _load_dataset())
    assert not verdict2.checked
    assert not verdict2.clean


def ep041_unit_m4_ood_pass_does_not_override_eval_failure() -> None:
    # An OOD item passing must not rescue a suite whose other evals fail.
    candidate = _load_candidate()
    dataset = _load_dataset()
    suite = _load_suite()
    results = _all_pass(suite)
    last = suite.evals[-1]
    results[last.eval_id][0] = DimensionResult(
        dimension=last.dimensions[0], passed=False, detail="observed failure"
    )
    score = score_suite(suite, results)
    verdict = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=_load_binding(),
        suite_score=score,
        training_started_at=TRAINING_START,
    )
    assert not verdict.eligible
    assert any("eval policy not passed" in reason for reason in verdict.reasons)


# ---------------------------------------------------------------------------
# Evidence: current-run and redacted
# ---------------------------------------------------------------------------


def ep041_unit_m4_evidence_current_run_bound() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    suite = _load_suite()
    score = score_suite(suite, _all_pass(suite))
    eligibility = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=_load_binding(),
        suite_score=score,
        training_started_at=TRAINING_START,
    )
    leakage = check_no_eval_leakage(suite, dataset)
    plan = _load_plan()
    evidence = build_training_evidence(
        run_id="ep041-m4-run-1",
        git_commit="abc123",
        candidate=candidate,
        dataset_id=dataset.dataset_id,
        dataset_digest=sha256_manifest(MANIFEST),
        eval_suite=suite,
        eval_suite_digest=suite_digest_of(suite),
        plan=plan,
        eligibility=eligibility,
        leakage=leakage,
        qlora_status=QloraStatus.PENDING,
        timestamp="2026-08-24T00:00:00Z",
    )
    payload = evidence.to_dict()
    assert payload["run_id"] == "ep041-m4-run-1"
    assert payload["git_commit"] == "abc123"
    assert payload["candidate_id"] == "nexus-candidate-v1"
    assert payload["dataset_digest"] == sha256_manifest(MANIFEST)
    assert payload["eval_suite_digest"] == suite_digest_of(suite)
    assert payload["plan_digest"] == plan.plan_digest
    assert payload["eligibility"]
    assert payload["leakage_clean"]
    assert payload["decision"] == "READY_TO_TRAIN"


def ep041_unit_m4_evidence_block_on_failure() -> None:
    candidate = _candidate(dataset_ref="")
    dataset = _load_dataset()
    suite = _load_suite()
    eligibility = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=_load_binding(),
        suite_score=score_suite(suite, _all_pass(suite)),
        training_started_at=TRAINING_START,
    )
    leakage = check_no_eval_leakage(suite, dataset)
    plan = _load_plan()
    evidence = build_training_evidence(
        run_id="ep041-m4-run-2",
        git_commit="abc123",
        candidate=candidate,
        dataset_id=dataset.dataset_id,
        dataset_digest=sha256_manifest(MANIFEST),
        eval_suite=suite,
        eval_suite_digest=suite_digest_of(suite),
        plan=plan,
        eligibility=eligibility,
        leakage=leakage,
        qlora_status=QloraStatus.PENDING,
    )
    assert not evidence.eligibility
    assert evidence.decision == "BLOCK"


def ep041_unit_m4_evidence_redacted_no_secret_leak() -> None:
    candidate = _load_candidate()
    dataset = _load_dataset()
    suite = _load_suite()
    eligibility = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=sha256_manifest(MANIFEST),
        suite=suite,
        binding=_load_binding(),
        suite_score=score_suite(suite, _all_pass(suite)),
        training_started_at=TRAINING_START,
    )
    leakage = check_no_eval_leakage(suite, dataset)
    plan = _load_plan()
    evidence = build_training_evidence(
        run_id="ep041-m4-run-3",
        git_commit="abc123",
        candidate=candidate,
        dataset_id=dataset.dataset_id,
        dataset_digest=sha256_manifest(MANIFEST),
        eval_suite=suite,
        eval_suite_digest=suite_digest_of(suite),
        plan=plan,
        eligibility=eligibility,
        leakage=leakage,
        qlora_status=QloraStatus.PENDING,
    )
    payload = json.dumps(evidence.to_redacted_dict())
    assert _secret_canary() not in payload


def ep041_unit_m4_redaction_canary_scrubbed() -> None:
    # The redaction layer must scrub a secret-shaped canary from any
    # evidence/error payload built at runtime.
    from nexus_microbrain import redact_text

    canary = _secret_canary()
    redacted = redact_text(canary)
    assert "REDACTED" in redacted
    assert canary not in redacted
