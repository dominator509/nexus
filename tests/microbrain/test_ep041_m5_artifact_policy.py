"""EP-041 M5 unit tests: artifact, GGUF, shadow, and promotion closure.

Test names begin with ep041_unit_ per the EP-041 milestone contract.
Every test exercises the real M5 behavior
(python/nexus_microbrain/artifact_policy.py) against the real M1-M4
canonical surfaces and the real committed artifact fixtures under
microbrain/artifacts/fixtures/. GGUF ARTIFACT EXISTS != QUANTIZATION
VERIFIED; SHADOW PASSED != PROMOTED; PROMOTION DECISION != AUTONOMOUS
DEPLOYMENT. Real GGUF quantization is NOT ASSERTED - the fixture
marker is fixture-only.
"""

from __future__ import annotations

import json

import pytest
from nexus_microbrain import (
    MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD,
    MICROBRAIN_CODE_INVALID_INPUT,
    MICROBRAIN_CODE_MISSING_REQUIRED,
    MICROBRAIN_CODE_UNKNOWN_VOCABULARY,
    CandidateStatus,
    DataProvenance,
    DatasetPolicy,
    DimensionResult,
    FrozenEvalSuite,
    MicrobrainDataset,
    MicrobrainError,
    OodVerdict,
    PromotionDecision,
    PromotionEvidence,
    PromotionGate,
    PromotionPrerequisites,
    PromotionVerdict,
    QloraRun,
    QloraStatus,
    QuantizationFormat,
    QuantizedArtifact,
    Role,
    ShadowComparator,
    ShadowComparison,
    ShadowDecision,
    ShadowGateVerdict,
    SuiteBinding,
    TrainingCandidate,
    TrainingExample,
    check_candidate_eligibility,
    check_no_eval_leakage,
    load_manifest,
    load_suite_binding,
    promotion_decision_never_deploys,
    promotion_gate_decision,
    score_suite,
    sha256_file,
    shadow_gate_verdict,
    validate_artifact,
    verify_artifact_file,
)

REPO_ROOT = __import__("pathlib").Path(__file__).resolve().parents[2]
ARTIFACT_JSON = (
    REPO_ROOT / "microbrain" / "artifacts" / "fixtures" / "nexus-artifact-v1.artifact.json"
)
ARTIFACT_MARKER = (
    REPO_ROOT / "microbrain" / "artifacts" / "fixtures" / "nexus-artifact-v1.gguf.marker"
)
CANDIDATE_JSON = (
    REPO_ROOT / "microbrain" / "training" / "plans" / "nexus-candidate-v1.candidate.json"
)
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


def _load_artifact() -> QuantizedArtifact:
    return QuantizedArtifact.from_dict(json.loads(ARTIFACT_JSON.read_text(encoding="utf-8")))


def _load_suite() -> FrozenEvalSuite:
    return FrozenEvalSuite.from_dict(json.loads(SUITE_JSON.read_text(encoding="utf-8")))


def _load_binding() -> SuiteBinding:
    return load_suite_binding(BINDING_JSON)


def _load_dataset() -> MicrobrainDataset:
    return load_manifest(MANIFEST)


def _load_candidate() -> TrainingCandidate:
    return TrainingCandidate.from_dict(json.loads(CANDIDATE_JSON.read_text(encoding="utf-8")))


def _example(
    *,
    example_id: str = "m5-ex-1",
    role: Role = Role.INTERPRETATION,
    provenance: DataProvenance = DataProvenance.DETERMINISTIC_GENERATION,
    license_ref: str | None = "nexus-synthetic-mit",
    hard_negative: bool = False,
    ood: OodVerdict = OodVerdict.IN_DISTRIBUTION,
) -> TrainingExample:
    return TrainingExample(
        example_id=example_id,
        role=role,
        input_text="artifact test",
        control_object={
            "schema_version": "1",
            "intent": "artifact.item",
            "route": "DETERMINISTIC",
            "risk": "R0",
        },
        provenance=provenance,
        hard_negative=hard_negative,
        ood_verdict=ood,
        license_ref=license_ref,
    )


def _completed_run(
    *,
    run_id: str = "run-m5-1",
    candidate_ref: str = "nexus-candidate-v1",
    output_digest: str | None = None,
) -> QloraRun:
    return QloraRun(
        run_id=run_id,
        candidate_ref=candidate_ref,
        adapter="adapter-m5-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.COMPLETED,
        correlation_id=output_digest,
    )


def _real_prerequisites() -> PromotionPrerequisites:
    return PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=True,
        eval_frozen_before_training=True,
        candidate_eligible=True,
        no_eval_leakage=True,
        training_run_certified=True,
        artifact_verified=True,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )


def _shadow_pass() -> ShadowGateVerdict:
    comparator = ShadowComparator(
        run_id="shadow-m5-1",
        candidate_ref="nexus-candidate-v1",
        provider_ref="reflex-provider-v1",
        comparisons=(
            ShadowComparison(
                input_ref="input-1",
                candidate_decision="A",
                provider_decision="A",
                decision=ShadowDecision.MATCH,
            ),
        ),
        exact_match_rate=1.0,
        consequential_false_positives=0,
    )
    return shadow_gate_verdict(comparator)


def _declared_evidence(
    *,
    candidate_id: str = "nexus-candidate-v1",
    qlora_status: str = QloraStatus.COMPLETED.value,
    artifact_digest: str | None = None,
    shadow_run_id: str = "shadow-m5-1",
    false_positive_count: int = 0,
    certification_boundary: str = (
        "policy-surface declared evidence; real training execution certified separately"
    ),
) -> PromotionEvidence:
    """A declared promotion/evaluation evidence record (AUD-064).

    The default is internally consistent with _real_prerequisites() and
    _shadow_pass(): completed (executed) run, well-formed bound digests,
    matching shadow run, zero false positives. Hostile proofs override
    individual fields to show the gate fails closed.
    """
    artifact = _load_artifact()
    dataset = _load_dataset()
    suite = _load_suite()
    plan = json.loads(
        (
            REPO_ROOT / "microbrain" / "training" / "plans" / "nexus-training-plan-v1.plan.json"
        ).read_text(encoding="utf-8")
    )
    return PromotionEvidence(
        run_id="run-m5-evidence-aud064",
        git_commit="abc123",
        candidate_id=candidate_id,
        dataset_id=dataset.dataset_id,
        dataset_digest=sha256_file(MANIFEST),
        eval_suite_id=suite.suite_id,
        eval_suite_digest=plan.get("eval_suite_digest", "sha256:" + "a" * 64),
        plan_digest=plan.get("plan_digest", "sha256:" + "b" * 64),
        qlora_run_id="run-m5-evidence-aud064",
        qlora_status=qlora_status,
        artifact_id=artifact.artifact_id,
        artifact_digest=artifact_digest or artifact.digest,
        quantization_format=artifact.format.value,
        shadow_run_id=shadow_run_id,
        shadow_decision="LOW_RISK_CANARY",
        false_positive_count=false_positive_count,
        promotion_decision="",
        promotion_gate="",
        certification_boundary=certification_boundary,
    )


def _secret_canary() -> str:
    """Runtime-constructed secret-shaped canary (no tracked literal)."""
    return "ghp_" + "x" * 35


# ---------------------------------------------------------------------------
# Real fixture loading
# ---------------------------------------------------------------------------


def ep041_unit_m5_artifact_fixture_loads() -> None:
    artifact = _load_artifact()
    assert artifact.artifact_id == "nexus-artifact-v1"
    assert artifact.format is QuantizationFormat.GGUF
    assert artifact.candidate_ref == "nexus-candidate-v1"


def ep041_unit_m5_artifact_digest_matches_marker_file() -> None:
    artifact = _load_artifact()
    assert artifact.digest == sha256_file(ARTIFACT_MARKER)


def ep041_unit_m5_artifact_marker_is_fixture_only() -> None:
    # The marker file must carry the fixture-only label so it can never
    # be mistaken for a real quantized model.
    text = ARTIFACT_MARKER.read_text(encoding="utf-8")
    assert "fixture-only" in text
    assert "NOT" in text and "ASSERTED" in text


# ---------------------------------------------------------------------------
# Artifact validation fail-closed
# ---------------------------------------------------------------------------


def ep041_unit_m5_artifact_missing_path_denied() -> None:
    with pytest.raises(MicrobrainError) as exc:
        verify_artifact_file(
            REPO_ROOT / "microbrain" / "artifacts" / "fixtures" / "does-not-exist.gguf"
        )
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


def ep041_unit_m5_artifact_missing_digest_denied() -> None:
    # The M1 contract fails closed at construction for an artifact
    # without a digest, so the M5 gate can never see a missing-digest
    # artifact (DIGEST PRESENT != ARTIFACT VERIFIED, enforced at the
    # contract boundary).
    with pytest.raises(MicrobrainError) as exc:
        QuantizedArtifact(
            artifact_id="art-m5-missing-digest",
            candidate_ref="nexus-candidate-v1",
            format=QuantizationFormat.GGUF,
            quantization="Q4_K_M",
            digest="",
            size_bytes=1,
        )
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_m5_artifact_malformed_digest_denied() -> None:
    # Same boundary: a malformed digest cannot construct.
    with pytest.raises(MicrobrainError) as exc:
        QuantizedArtifact(
            artifact_id="art-m5-malformed",
            candidate_ref="nexus-candidate-v1",
            format=QuantizationFormat.GGUF,
            quantization="Q4_K_M",
            digest="short",
            size_bytes=1,
        )
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_m5_artifact_digest_mismatch_denied() -> None:
    artifact = _load_artifact()
    verdict = validate_artifact(
        artifact,
        run=_completed_run(output_digest="sha256:" + "0" * 64),
        expected_digest="sha256:" + "0" * 64,
    )
    assert not verdict.verified
    assert any("digest mismatch" in reason for reason in verdict.reasons)


def ep041_unit_m5_artifact_non_gguf_denied() -> None:
    with pytest.raises(MicrobrainError):
        QuantizationFormat.parse("SAFETENSORS")  # type: ignore[arg-type]


def ep041_unit_m5_artifact_from_failed_run_denied() -> None:
    artifact = _load_artifact()
    run = QloraRun(
        run_id="run-m5-fail",
        candidate_ref="nexus-candidate-v1",
        adapter="adapter-m5-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.FAILED,
    )
    verdict = validate_artifact(artifact, run=run)
    assert not verdict.verified
    assert any("failed QLoRA run" in reason for reason in verdict.reasons)


def ep041_unit_m5_artifact_from_declared_run_denied() -> None:
    artifact = _load_artifact()
    run = QloraRun(
        run_id="run-m5-pending",
        candidate_ref="nexus-candidate-v1",
        adapter="adapter-m5-1",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=QloraStatus.PENDING,
    )
    verdict = validate_artifact(artifact, run=run)
    assert not verdict.verified
    assert any("declared-only" in reason for reason in verdict.reasons)


def ep041_unit_m5_artifact_without_run_binding_denied() -> None:
    artifact = _load_artifact()
    verdict = validate_artifact(artifact, run=None)
    assert not verdict.verified
    assert any("without run binding" in reason for reason in verdict.reasons)


def ep041_unit_m5_artifact_wrong_candidate_denied() -> None:
    artifact = _load_artifact()
    run = _completed_run(candidate_ref="some-other-candidate")
    verdict = validate_artifact(artifact, run=run)
    assert not verdict.verified
    assert any("wrong candidate" in reason for reason in verdict.reasons)


def ep041_unit_m5_artifact_candidate_mismatch_denied() -> None:
    artifact = _load_artifact()
    candidate = TrainingCandidate(
        candidate_id="other-candidate",
        role=Role.INTERPRETATION,
        model_ref="m",
        base_model="b",
        dataset_ref="nexus-synthetic-role-ops-v1",
        status=CandidateStatus.CANDIDATE,
    )
    verdict = validate_artifact(artifact, run=_completed_run(), candidate=candidate)
    assert not verdict.verified
    assert any("wrong candidate" in reason for reason in verdict.reasons)


def ep041_unit_m5_artifact_real_fixture_verified_with_completed_run() -> None:
    artifact = _load_artifact()
    run = _completed_run(output_digest=artifact.digest)
    verdict = validate_artifact(artifact, run=run, expected_digest=artifact.digest)
    assert verdict.verified
    assert verdict.digest_verified
    assert verdict.format_verified
    assert verdict.run_bound
    # File digest also verifies against the committed marker bytes.
    file_verification = verify_artifact_file(ARTIFACT_MARKER, artifact.digest)
    assert file_verification.verified


def ep041_unit_m5_artifact_file_digest_mismatch_denied(tmp_path) -> None:
    tmp = tmp_path / "tampered.gguf"
    tmp.write_bytes(b"tampered bytes")
    file_verification = verify_artifact_file(tmp, "sha256:" + "1" * 64)
    assert not file_verification.verified


def ep041_unit_m5_artifact_redacted_verdict_no_secret_leak() -> None:
    artifact = _load_artifact()
    verdict = validate_artifact(artifact, run=None)
    payload = json.dumps(verdict.to_redacted_dict())
    assert _secret_canary() not in payload


# ---------------------------------------------------------------------------
# QLoRA-output honesty
# ---------------------------------------------------------------------------


def ep041_unit_m5_qlora_metrics_alone_never_certify_artifact() -> None:
    # An artifact bound only to a metrics story (no digest, no run
    # evidence) must never verify.
    artifact = QuantizedArtifact(
        artifact_id="art-m5-metrics",
        candidate_ref="nexus-candidate-v1",
        format=QuantizationFormat.GGUF,
        quantization="Q4_K_M",
        digest="sha256:" + "e" * 64,
        size_bytes=10,
    )
    verdict = validate_artifact(artifact, run=None)
    assert not verdict.verified
    assert any("without run binding" in reason for reason in verdict.reasons)


def ep041_unit_m5_qlora_adapter_digest_missing_denied() -> None:
    run = _completed_run(output_digest=None)
    assert run.status is QloraStatus.COMPLETED
    # M4's qlora_run_verdict proves a completed run without output
    # digest is not certified; M5's artifact layer independently
    # requires a matching digest.
    artifact = _load_artifact()
    verdict = validate_artifact(artifact, run=run, expected_digest=artifact.digest)
    assert verdict.verified  # digest of the artifact itself is present
    # But the run's own output digest binding is a separate M4 proof;
    # assert the invariant relationship here via the M4 verdict.
    from nexus_microbrain import qlora_run_verdict

    run_verdict = qlora_run_verdict(run, candidate_eligible=True)
    assert not run_verdict.certified


# ---------------------------------------------------------------------------
# Shadow comparator: shadow pass != promoted
# ---------------------------------------------------------------------------


def ep041_unit_m5_shadow_pass_advances_to_canary_not_promote() -> None:
    verdict = _shadow_pass()
    assert verdict.passed
    assert verdict.next_gate == "LOW_RISK_CANARY"
    assert verdict.next_gate != "PROMOTED"


def ep041_unit_m5_shadow_missing_evidence_fails_closed() -> None:
    verdict = shadow_gate_verdict(None)
    assert not verdict.passed
    assert any("missing shadow evidence" in reason for reason in verdict.reasons)


def ep041_unit_m5_shadow_empty_comparisons_fails_closed() -> None:
    comparator = ShadowComparator(
        run_id="shadow-m5-empty",
        candidate_ref="nexus-candidate-v1",
        provider_ref="reflex-provider-v1",
        comparisons=(),
        exact_match_rate=1.0,
        consequential_false_positives=0,
    )
    verdict = shadow_gate_verdict(comparator)
    assert not verdict.passed
    assert any("missing shadow comparison evidence" in reason for reason in verdict.reasons)


def ep041_unit_m5_shadow_false_positives_block() -> None:
    comparator = ShadowComparator(
        run_id="shadow-m5-fp",
        candidate_ref="nexus-candidate-v1",
        provider_ref="reflex-provider-v1",
        comparisons=(),
        exact_match_rate=1.0,
        consequential_false_positives=2,
    )
    verdict = shadow_gate_verdict(comparator)
    assert not verdict.passed
    assert any("false positives" in reason for reason in verdict.reasons)


def ep041_unit_m5_shadow_low_match_rate_blocks() -> None:
    comparator = ShadowComparator(
        run_id="shadow-m5-low",
        candidate_ref="nexus-candidate-v1",
        provider_ref="reflex-provider-v1",
        comparisons=(),
        exact_match_rate=0.5,
        consequential_false_positives=0,
    )
    verdict = shadow_gate_verdict(comparator)
    assert not verdict.passed
    assert any("exact match rate" in reason for reason in verdict.reasons)


def ep041_unit_m5_shadow_differ_blocks() -> None:
    comparator = ShadowComparator(
        run_id="shadow-m5-differ",
        candidate_ref="nexus-candidate-v1",
        provider_ref="reflex-provider-v1",
        comparisons=(
            ShadowComparison(
                input_ref="input-1",
                candidate_decision="A",
                provider_decision="B",
                decision=ShadowDecision.DIFFER,
            ),
        ),
        exact_match_rate=1.0,
        consequential_false_positives=0,
    )
    verdict = shadow_gate_verdict(comparator)
    assert not verdict.passed
    assert any("differs from provider" in reason for reason in verdict.reasons)


def ep041_unit_m5_shadow_unknown_decision_fails_closed() -> None:
    with pytest.raises(MicrobrainError) as exc:
        ShadowDecision.parse("MYSTERY")
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_m5_cannot_promote_directly_from_shadow() -> None:
    # The M1 contract refuses a PROMOTE verdict at the SHADOW gate.
    with pytest.raises(MicrobrainError) as exc:
        PromotionDecision(
            decision_id="dec-m5-direct",
            verdict=PromotionVerdict.PROMOTE,
            gate=PromotionGate.SHADOW,
            candidate_ref="nexus-candidate-v1",
            eval_ref="nexus-frozen-suite-v1",
            shadow_ref="shadow-m5-1",
            zero_consequential_false_positives=True,
            reason="direct from shadow",
        )
    assert exc.value.code == MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD


def ep041_unit_m5_cannot_promote_with_false_positives() -> None:
    with pytest.raises(MicrobrainError) as exc:
        PromotionDecision(
            decision_id="dec-m5-fp",
            verdict=PromotionVerdict.PROMOTE,
            gate=PromotionGate.GRADUAL,
            candidate_ref="nexus-candidate-v1",
            eval_ref="nexus-frozen-suite-v1",
            shadow_ref="shadow-m5-1",
            zero_consequential_false_positives=False,
            reason="fp present",
        )
    assert exc.value.code == MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD


# ---------------------------------------------------------------------------
# Strict promotion gate
# ---------------------------------------------------------------------------


def ep041_unit_m5_promotion_all_prerequisites_met() -> None:
    candidate = _load_candidate()
    shadow = _shadow_pass()
    decision = promotion_gate_decision(
        decision_id="dec-m5-ok",
        candidate=candidate,
        prerequisites=_real_prerequisites(),
        shadow=shadow,
        eval_ref="nexus-frozen-suite-v1",
        evidence=_declared_evidence(),
    )
    assert decision.verdict is PromotionVerdict.PROMOTE
    assert decision.gate is PromotionGate.GRADUAL
    assert decision.zero_consequential_false_positives


def ep041_unit_m5_promotion_missing_dataset_policy_denies() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=False,
        frozen_eval_passed=True,
        eval_frozen_before_training=True,
        candidate_eligible=True,
        no_eval_leakage=True,
        training_run_certified=True,
        artifact_verified=True,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-nodata",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "dataset policy not passed" in decision.reason


def ep041_unit_m5_promotion_missing_eval_denies() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=False,
        eval_frozen_before_training=True,
        candidate_eligible=True,
        no_eval_leakage=True,
        training_run_certified=True,
        artifact_verified=True,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-noeval",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "frozen eval not passed" in decision.reason


def ep041_unit_m5_promotion_eval_after_training_denies() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=True,
        eval_frozen_before_training=False,
        candidate_eligible=True,
        no_eval_leakage=True,
        training_run_certified=True,
        artifact_verified=True,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-timing",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "eval not frozen before training" in decision.reason


def ep041_unit_m5_promotion_candidate_not_eligible_denies() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=True,
        eval_frozen_before_training=True,
        candidate_eligible=False,
        no_eval_leakage=True,
        training_run_certified=True,
        artifact_verified=True,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-nocand",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "candidate not eligible" in decision.reason


def ep041_unit_m5_promotion_leakage_denies() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=True,
        eval_frozen_before_training=True,
        candidate_eligible=True,
        no_eval_leakage=False,
        training_run_certified=True,
        artifact_verified=True,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-leak",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "eval leakage present" in decision.reason


def ep041_unit_m5_promotion_run_not_certified_denies() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=True,
        eval_frozen_before_training=True,
        candidate_eligible=True,
        no_eval_leakage=True,
        training_run_certified=False,
        artifact_verified=True,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-norun",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "training run not certified" in decision.reason


def ep041_unit_m5_promotion_artifact_not_verified_denies() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=True,
        eval_frozen_before_training=True,
        candidate_eligible=True,
        no_eval_leakage=True,
        training_run_certified=True,
        artifact_verified=False,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-noart",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "quantized artifact not verified" in decision.reason


def ep041_unit_m5_promotion_shadow_not_passed_denies() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=True,
        eval_frozen_before_training=True,
        candidate_eligible=True,
        no_eval_leakage=True,
        training_run_certified=True,
        artifact_verified=True,
        shadow_gate_passed=False,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-noshadow",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=shadow_gate_verdict(None),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "shadow gate not passed" in decision.reason


def ep041_unit_m5_promotion_false_positives_deny() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=True,
        eval_frozen_before_training=True,
        candidate_eligible=True,
        no_eval_leakage=True,
        training_run_certified=True,
        artifact_verified=True,
        shadow_gate_passed=True,
        zero_consequential_false_positives=False,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-fps",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "consequential false positives" in decision.reason


def ep041_unit_m5_promotion_ood_unsafe_denies() -> None:
    candidate = _load_candidate()
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=True,
        eval_frozen_before_training=True,
        candidate_eligible=True,
        no_eval_leakage=True,
        training_run_certified=True,
        artifact_verified=True,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=False,
    )
    decision = promotion_gate_decision(
        decision_id="dec-m5-ood",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "OOD verdict not safe" in decision.reason


def ep041_unit_m5_promotion_decision_never_deploys() -> None:
    decision = promotion_gate_decision(
        decision_id="dec-m5-nodeploy",
        candidate=_load_candidate(),
        prerequisites=_real_prerequisites(),
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
        evidence=_declared_evidence(),
    )
    assert decision.verdict is PromotionVerdict.PROMOTE
    assert promotion_decision_never_deploys(decision)
    assert decision.gate is not PromotionGate.PROMOTED


# ---------------------------------------------------------------------------
# Final live-fire composition
# ---------------------------------------------------------------------------


def ep041_unit_m5_final_live_fire_composition_honest_not_promoted() -> None:
    """Compose the whole real local surface.

    M1 contract -> M2 dataset policy -> M3 frozen eval -> M4 candidate
    eligibility -> M5 artifact/shadow/promotion. The honest result:
    the local surface supports eligibility but the QLoRA run is
    declared-only (no real execution), so the artifact is not
    verifiable and promotion must not be issued.
    """
    candidate = _load_candidate()
    dataset = _load_dataset()
    dataset_digest = sha256_file(MANIFEST)
    suite = _load_suite()
    binding = _load_binding()
    results = {
        evaluation.eval_id: [
            DimensionResult(dimension=dimension, passed=True) for dimension in evaluation.dimensions
        ]
        for evaluation in suite.evals
    }
    score = score_suite(suite, results)
    assert score.passed

    eligibility = check_candidate_eligibility(
        candidate,
        dataset=dataset,
        dataset_digest=dataset_digest,
        suite=suite,
        binding=binding,
        suite_score=score,
        training_started_at=TRAINING_START,
        policy=POLICY,
    )
    assert eligibility.eligible

    leakage = check_no_eval_leakage(suite, dataset)
    assert leakage.clean

    # The real committed run story: declared-only QLoRA (no execution).
    declared_run = QloraRun(
        run_id="run-livefire-declared",
        candidate_ref=candidate.candidate_id,
        adapter="adapter-livefire",
        rank=16,
        alpha=32,
        seed=7,
        config_digest="sha256:" + "a" * 64,
        dataset_ref=dataset.dataset_id,
        status=QloraStatus.PENDING,
    )
    artifact = _load_artifact()
    artifact_verdict = validate_artifact(artifact, run=declared_run)
    assert not artifact_verdict.verified

    # Promotion must be denied: training run not certified.
    prereqs = PromotionPrerequisites(
        dataset_policy_passed=True,
        frozen_eval_passed=score.passed,
        eval_frozen_before_training=True,
        candidate_eligible=eligibility.eligible,
        no_eval_leakage=leakage.clean,
        training_run_certified=False,
        artifact_verified=artifact_verdict.verified,
        shadow_gate_passed=True,
        zero_consequential_false_positives=True,
        ood_safe=True,
    )
    decision = promotion_gate_decision(
        decision_id="dec-livefire",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=_shadow_pass(),
        eval_ref=suite.suite_id,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "training run not certified" in decision.reason


def ep041_unit_m5_final_live_fire_current_run_redacted_evidence() -> None:
    from nexus_microbrain import (
        PromotionEvidence,
        TrainingPlan,
        suite_digest_of,
    )

    candidate = _load_candidate()
    dataset = _load_dataset()
    dataset_digest = sha256_file(MANIFEST)
    suite = _load_suite()
    plan = TrainingPlan.from_dict(
        json.loads(
            (
                REPO_ROOT / "microbrain" / "training" / "plans" / "nexus-training-plan-v1.plan.json"
            ).read_text(encoding="utf-8")
        )
    )
    artifact = _load_artifact()
    shadow = _shadow_pass()
    # AUD-064: the committed QLoRA run story is declared-only (PENDING),
    # so the DECLARED promotion/evaluation evidence can never support a
    # PROMOTE verdict. Even with every owned prerequisite asserted, the
    # gate must fail closed and the recorded evidence must say DENY.
    declared_only_evidence = PromotionEvidence(
        run_id="ep041-m5-final-1",
        git_commit="abc123",
        candidate_id=candidate.candidate_id,
        dataset_id=dataset.dataset_id,
        dataset_digest=dataset_digest,
        eval_suite_id=suite.suite_id,
        eval_suite_digest=suite_digest_of(suite),
        plan_digest=plan.plan_digest,
        qlora_run_id="run-m5-evidence",
        qlora_status=QloraStatus.PENDING.value,
        artifact_id=artifact.artifact_id,
        artifact_digest=artifact.digest,
        quantization_format=artifact.format.value,
        shadow_run_id=shadow.run_id,
        shadow_decision=shadow.next_gate,
        false_positive_count=0,
        promotion_decision="",
        promotion_gate="",
        certification_boundary="artifact/GGUF manifest + shadow + promotion behavior "
        "INTERNAL BEHAVIOR CERTIFIED for exact exercised local surface; "
        "real GGUF quantization NOT ASSERTED; declared-only QLoRA run "
        "PENDING - training NOT executed",
        timestamp="2026-08-24T00:00:00Z",
    )
    decision = promotion_gate_decision(
        decision_id="dec-evidence",
        candidate=candidate,
        prerequisites=_real_prerequisites(),
        shadow=shadow,
        eval_ref=suite.suite_id,
        evidence=declared_only_evidence,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "training run not COMPLETED" in decision.reason

    evidence = PromotionEvidence(
        run_id=declared_only_evidence.run_id,
        git_commit=declared_only_evidence.git_commit,
        candidate_id=declared_only_evidence.candidate_id,
        dataset_id=declared_only_evidence.dataset_id,
        dataset_digest=declared_only_evidence.dataset_digest,
        eval_suite_id=declared_only_evidence.eval_suite_id,
        eval_suite_digest=declared_only_evidence.eval_suite_digest,
        plan_digest=declared_only_evidence.plan_digest,
        qlora_run_id=declared_only_evidence.qlora_run_id,
        qlora_status=declared_only_evidence.qlora_status,
        artifact_id=declared_only_evidence.artifact_id,
        artifact_digest=declared_only_evidence.artifact_digest,
        quantization_format=declared_only_evidence.quantization_format,
        shadow_run_id=declared_only_evidence.shadow_run_id,
        shadow_decision=declared_only_evidence.shadow_decision,
        false_positive_count=declared_only_evidence.false_positive_count,
        promotion_decision=decision.verdict.value,
        promotion_gate=decision.gate.value,
        certification_boundary=declared_only_evidence.certification_boundary,
        timestamp=declared_only_evidence.timestamp,
    )
    payload = evidence.to_dict()
    assert payload["run_id"] == "ep041-m5-final-1"
    assert payload["git_commit"] == "abc123"
    assert payload["promotion_decision"] == "DENY"
    assert payload["qlora_status"] == "PENDING"
    redacted = json.dumps(evidence.to_redacted_dict())
    assert _secret_canary() not in redacted


def ep041_unit_m5_redaction_canary_scrubbed() -> None:
    from nexus_microbrain import redact_text

    canary = _secret_canary()
    redacted = redact_text(canary)
    assert "REDACTED" in redacted
    assert canary not in redacted
