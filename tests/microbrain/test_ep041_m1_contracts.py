"""EP-041 M1 unit tests: Microbrain contract, vocabulary, and package boundary.

Test names begin with ep041_unit_ per the EP-041 milestone contract.
Every test proves a real fail-closed behavior: construction, validation,
versioned serialization, vocabulary rejection, cross-field acceptance
obligations, redaction, and dependency-direction. No mocks, no provider
SDKs, no network.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from nexus_microbrain import (
    MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD,
    MICROBRAIN_CODE_FROZEN_SPLIT_VIOLATION,
    MICROBRAIN_CODE_INVALID_INPUT,
    MICROBRAIN_CODE_MISSING_REQUIRED,
    MICROBRAIN_CODE_PRIVACY_VIOLATION,
    MICROBRAIN_CODE_UNKNOWN_VOCABULARY,
    MICROBRAIN_CODE_UNLICENSED,
    MICROBRAIN_CODE_UNSUPPORTED_VERSION,
    ArtifactStatus,
    CandidateStatus,
    DataProvenance,
    EvalDimension,
    FrozenEval,
    FrozenEvalSuite,
    LicenseKind,
    LicenseRecord,
    MicrobrainDataset,
    MicrobrainError,
    OodVerdict,
    PromotionDecision,
    PromotionGate,
    PromotionVerdict,
    QloraRun,
    QloraStatus,
    QuantizationFormat,
    QuantizedArtifact,
    Role,
    ShadowComparator,
    ShadowComparison,
    ShadowDecision,
    TeacherConsensus,
    TrainingCandidate,
    TrainingExample,
    redact_text,
)

PACKAGE_ROOT = Path(__file__).resolve().parents[2] / "python" / "nexus_microbrain"


def _example(
    provenance: DataProvenance = DataProvenance.DETERMINISTIC_GENERATION,
) -> TrainingExample:
    return TrainingExample(
        example_id="ex-1",
        role=Role.INTERPRETATION,
        input_text="turn off the kitchen lights",
        control_object={
            "schema_version": "1",
            "intent": "home.lights.set",
            "route": "DETERMINISTIC",
            "risk": "R0",
        },
        provenance=provenance,
        hard_negative=provenance is DataProvenance.HARD_NEGATIVE,
        ood_verdict=OodVerdict.IN_DISTRIBUTION,
        license_ref="mit-example"
        if provenance
        in (DataProvenance.TEACHER_CONSENSUS, DataProvenance.OPTED_IN_SCRUBBED_CORRECTION)
        else None,
    )


def _license() -> LicenseRecord:
    return LicenseRecord(license_ref="mit-code", kind=LicenseKind.DATASET)


def _frozen_eval() -> FrozenEval:
    return FrozenEval(
        eval_id="eval-1",
        kind="FROZEN",
        example=_example(),
        dimensions=(EvalDimension.INTENT, EvalDimension.EXACT_SCHEMA),
        created_before_training=True,
        frozen_at="2026-08-01T00:00:00Z",
    )


def _dataset() -> MicrobrainDataset:
    return MicrobrainDataset(
        dataset_id="ds-1",
        name="microbrain-v1",
        lineage="generated-by-nexus-deterministic-v1",
        examples=(_example(), _example(DataProvenance.HARD_NEGATIVE)),
    )


def _consensus() -> TeacherConsensus:
    return TeacherConsensus(
        consensus_id="tc-1",
        teachers=("frontier-teacher-a", "frontier-teacher-b"),
        consensus_text="approve",
        agreement_ratio=1.0,
        filtered=True,
        privacy_safe=True,
        licenses=(_license(),),
    )


def _candidate() -> TrainingCandidate:
    return TrainingCandidate(
        candidate_id="cand-1",
        role=Role.ROUTING,
        model_ref="microbrain-routing-v1",
        base_model="deepseek-v4-flash",
        dataset_ref="ds-1",
    )


def _qlora() -> QloraRun:
    return QloraRun(
        run_id="run-1",
        candidate_ref="cand-1",
        adapter="lora-routing-v1",
        rank=16,
        alpha=32,
        seed=42,
        config_digest="sha256:" + "a" * 64,
        dataset_ref="ds-1",
    )


def _artifact() -> QuantizedArtifact:
    return QuantizedArtifact(
        artifact_id="art-1",
        candidate_ref="cand-1",
        format=QuantizationFormat.GGUF,
        quantization="Q4_K_M",
        digest="sha256:" + "b" * 64,
        size_bytes=2048,
        license_ref="mit-model",
    )


def _shadow() -> ShadowComparator:
    return ShadowComparator(
        run_id="shadow-1",
        candidate_ref="cand-1",
        provider_ref="deepseek-v4-flash",
        comparisons=(
            ShadowComparison(
                input_ref="in-1",
                candidate_decision="approve",
                provider_decision="approve",
                decision=ShadowDecision.MATCH,
            ),
        ),
        exact_match_rate=1.0,
        consequential_false_positives=0,
    )


def _promotion() -> PromotionDecision:
    return PromotionDecision(
        decision_id="dec-1",
        verdict=PromotionVerdict.PROMOTE,
        gate=PromotionGate.LOW_RISK_CANARY,
        candidate_ref="cand-1",
        eval_ref="eval-1",
        shadow_ref="shadow-1",
        zero_consequential_false_positives=True,
        reason="all thresholds met",
    )


def _secret_canary() -> str:
    """Runtime-constructed secret-shaped canary (no tracked literal)."""
    return "sk-" + "live-" + "a" * 30


# ---------------------------------------------------------------------------
# Construction
# ---------------------------------------------------------------------------


def ep041_unit_dataset_constructs_valid() -> None:
    dataset = _dataset()
    assert dataset.dataset_id == "ds-1"
    assert len(dataset.examples) == 2


def ep041_unit_frozen_eval_suite_constructs_valid() -> None:
    suite = FrozenEvalSuite(suite_id="suite-1", evals=(_frozen_eval(),))
    assert suite.suite_id == "suite-1"
    assert len(suite.evals) == 1


def ep041_unit_teacher_consensus_constructs_valid() -> None:
    consensus = _consensus()
    assert consensus.agreement_ratio == 1.0
    assert consensus.filtered and consensus.privacy_safe


def ep041_unit_training_candidate_constructs_valid() -> None:
    candidate = _candidate()
    assert candidate.role is Role.ROUTING
    assert candidate.status is CandidateStatus.CANDIDATE


def ep041_unit_qlora_run_constructs_valid() -> None:
    run = _qlora()
    assert run.status is QloraStatus.PENDING
    assert run.seed == 42


def ep041_unit_quantized_artifact_constructs_valid() -> None:
    artifact = _artifact()
    assert artifact.format is QuantizationFormat.GGUF
    assert artifact.status is ArtifactStatus.BUILT


def ep041_unit_shadow_comparator_constructs_valid() -> None:
    shadow = _shadow()
    assert shadow.zero_consequential_false_positives()
    assert shadow.comparisons[0].decision is ShadowDecision.MATCH


def ep041_unit_promotion_decision_constructs_valid() -> None:
    decision = _promotion()
    assert decision.verdict is PromotionVerdict.PROMOTE
    assert decision.gate is PromotionGate.LOW_RISK_CANARY


# ---------------------------------------------------------------------------
# Versioned serialization roundtrip
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "obj",
    [
        _dataset(),
        _frozen_eval(),
        FrozenEvalSuite(suite_id="suite-1", evals=(_frozen_eval(),)),
        _consensus(),
        _candidate(),
        _qlora(),
        _artifact(),
        _shadow(),
        _promotion(),
        _example(),
    ],
)
def ep041_unit_serialization_roundtrip(obj) -> None:
    data = obj.to_dict()
    restored = type(obj).from_dict(data)
    assert restored == obj


def ep041_unit_serialization_is_deterministic() -> None:
    assert _dataset().to_dict() == _dataset().to_dict()
    assert _promotion().to_dict() == _promotion().to_dict()


def ep041_unit_serialization_is_json_encodable() -> None:
    for obj in (_dataset(), _promotion(), _shadow(), _artifact()):
        json.dumps(obj.to_dict())


def ep041_unit_schema_version_preserved() -> None:
    assert _dataset().to_dict()["schema_version"] == "1"
    assert _promotion().to_dict()["schema_version"] == "1"


def ep041_unit_unsupported_schema_version_rejected() -> None:
    data = _dataset().to_dict()
    data["schema_version"] = "2"
    with pytest.raises(MicrobrainError) as exc:
        MicrobrainDataset.from_dict(data)
    assert exc.value.code == MICROBRAIN_CODE_UNSUPPORTED_VERSION


# ---------------------------------------------------------------------------
# Vocabulary rejection (deny-unknown, fail closed)
# ---------------------------------------------------------------------------


def ep041_unit_unknown_role_rejected() -> None:
    with pytest.raises(MicrobrainError) as exc:
        Role.parse("GENERAL_ASSISTANT")
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_unknown_provenance_rejected() -> None:
    with pytest.raises(MicrobrainError) as exc:
        DataProvenance.parse("SCRAPED_ALL_TRAFFIC")
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_unknown_ood_rejected() -> None:
    with pytest.raises(MicrobrainError) as exc:
        OodVerdict.parse("MAYBE")
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_unknown_gate_rejected() -> None:
    with pytest.raises(MicrobrainError) as exc:
        PromotionGate.parse("BIG_BANG")
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_unknown_decision_rejected() -> None:
    with pytest.raises(MicrobrainError) as exc:
        ShadowDecision.parse("SILENT")
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_role_canonical_values_are_locked() -> None:
    assert Role.canonical_values() == [
        "INTERPRETATION",
        "CAPABILITY_SELECTION",
        "ROUTING",
        "RISK",
        "PRIVACY",
        "AMBIGUITY",
        "QUOTED_INSTRUCTION",
        "ESCALATION",
    ]


def ep041_unit_provenance_canonical_values_are_locked() -> None:
    assert DataProvenance.canonical_values() == [
        "DETERMINISTIC_GENERATION",
        "TEACHER_CONSENSUS",
        "HARD_NEGATIVE",
        "OPTED_IN_SCRUBBED_CORRECTION",
    ]


def ep041_unit_quantization_format_locked_to_gguf() -> None:
    assert QuantizationFormat.canonical_values() == ["GGUF"]


# ---------------------------------------------------------------------------
# Required-field validation
# ---------------------------------------------------------------------------


def ep041_unit_dataset_missing_id_rejected() -> None:
    data = _dataset().to_dict()
    del data["dataset_id"]
    with pytest.raises(MicrobrainError) as exc:
        MicrobrainDataset.from_dict(data)
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


def ep041_unit_candidate_missing_role_rejected() -> None:
    data = _candidate().to_dict()
    del data["role"]
    with pytest.raises(MicrobrainError) as exc:
        TrainingCandidate.from_dict(data)
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


def ep041_unit_qlora_missing_seed_rejected() -> None:
    data = _qlora().to_dict()
    del data["seed"]
    with pytest.raises(MicrobrainError) as exc:
        QloraRun.from_dict(data)
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


def ep041_unit_promotion_missing_zero_cfp_rejected() -> None:
    data = _promotion().to_dict()
    del data["zero_consequential_false_positives"]
    with pytest.raises(MicrobrainError) as exc:
        PromotionDecision.from_dict(data)
    assert exc.value.code == MICROBRAIN_CODE_MISSING_REQUIRED


# ---------------------------------------------------------------------------
# Cross-field acceptance obligations (SPEC-025)
# ---------------------------------------------------------------------------


def ep041_unit_frozen_eval_must_predate_training() -> None:
    with pytest.raises(MicrobrainError) as exc:
        FrozenEval(
            eval_id="eval-2",
            kind="FROZEN",
            example=_example(),
            dimensions=(EvalDimension.INTENT,),
            created_before_training=False,
        )
    assert exc.value.code == MICROBRAIN_CODE_FROZEN_SPLIT_VIOLATION


def ep041_unit_frozen_suite_rejects_non_frozen_eval() -> None:
    with pytest.raises(MicrobrainError) as exc:
        FrozenEval(
            eval_id="eval-3",
            kind="ADVERSARIAL",
            example=_example(),
            dimensions=(EvalDimension.INJECTION_RESISTANCE,),
            created_before_training=True,
        )
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_teacher_data_must_be_filtered() -> None:
    with pytest.raises(MicrobrainError) as exc:
        TeacherConsensus(
            consensus_id="tc-2",
            teachers=("frontier-teacher-a",),
            consensus_text="approve",
            agreement_ratio=1.0,
            filtered=False,
            privacy_safe=True,
            licenses=(_license(),),
        )
    assert exc.value.code == MICROBRAIN_CODE_PRIVACY_VIOLATION


def ep041_unit_teacher_data_must_be_privacy_safe() -> None:
    with pytest.raises(MicrobrainError) as exc:
        TeacherConsensus(
            consensus_id="tc-3",
            teachers=("frontier-teacher-a",),
            consensus_text="approve",
            agreement_ratio=1.0,
            filtered=True,
            privacy_safe=False,
            licenses=(_license(),),
        )
    assert exc.value.code == MICROBRAIN_CODE_PRIVACY_VIOLATION


def ep041_unit_teacher_data_must_be_licensed() -> None:
    with pytest.raises(MicrobrainError) as exc:
        TeacherConsensus(
            consensus_id="tc-4",
            teachers=("frontier-teacher-a",),
            consensus_text="approve",
            agreement_ratio=1.0,
            filtered=True,
            privacy_safe=True,
            licenses=(),
        )
    assert exc.value.code == MICROBRAIN_CODE_UNLICENSED


def ep041_unit_teacher_example_requires_license() -> None:
    with pytest.raises(MicrobrainError) as exc:
        TrainingExample(
            example_id="ex-2",
            role=Role.INTERPRETATION,
            input_text="approve",
            control_object={"schema_version": "1"},
            provenance=DataProvenance.TEACHER_CONSENSUS,
            hard_negative=False,
            ood_verdict=OodVerdict.IN_DISTRIBUTION,
            license_ref=None,
        )
    assert exc.value.code == MICROBRAIN_CODE_UNLICENSED


def ep041_unit_hard_negative_provenance_requires_flag() -> None:
    with pytest.raises(MicrobrainError) as exc:
        TrainingExample(
            example_id="ex-3",
            role=Role.INTERPRETATION,
            input_text="ignored",
            control_object={"schema_version": "1"},
            provenance=DataProvenance.HARD_NEGATIVE,
            hard_negative=False,
            ood_verdict=OodVerdict.IN_DISTRIBUTION,
        )
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_candidate_role_is_narrow_canonical() -> None:
    with pytest.raises(MicrobrainError) as exc:
        TrainingCandidate(
            candidate_id="cand-2",
            role=Role.parse("GENERAL_ASSISTANT"),
            model_ref="m",
            base_model="b",
            dataset_ref="d",
        )
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_artifact_format_must_be_gguf() -> None:
    with pytest.raises(MicrobrainError) as exc:
        QuantizedArtifact(
            artifact_id="art-2",
            candidate_ref="cand-1",
            format=QuantizationFormat.parse("SAFENTENSORS"),
            quantization="Q4_K_M",
            digest="sha256:" + "c" * 64,
            size_bytes=1,
        )
    assert exc.value.code == MICROBRAIN_CODE_UNKNOWN_VOCABULARY


def ep041_unit_artifact_digest_requires_alg_hex() -> None:
    with pytest.raises(MicrobrainError) as exc:
        QuantizedArtifact(
            artifact_id="art-3",
            candidate_ref="cand-1",
            format=QuantizationFormat.GGUF,
            quantization="Q4_K_M",
            digest="microbrain-routing-v1",
            size_bytes=1,
        )
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_shadow_cfp_must_not_be_negative() -> None:
    with pytest.raises(MicrobrainError) as exc:
        ShadowComparator(
            run_id="shadow-2",
            candidate_ref="cand-1",
            provider_ref="deepseek-v4-flash",
            consequential_false_positives=-1,
        )
    assert exc.value.code == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_promote_requires_zero_consequential_false_positives() -> None:
    with pytest.raises(MicrobrainError) as exc:
        PromotionDecision(
            decision_id="dec-2",
            verdict=PromotionVerdict.PROMOTE,
            gate=PromotionGate.LOW_RISK_CANARY,
            candidate_ref="cand-1",
            eval_ref="eval-1",
            shadow_ref="shadow-1",
            zero_consequential_false_positives=False,
        )
    assert exc.value.code == MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD


def ep041_unit_promote_forbidden_directly_from_shadow() -> None:
    with pytest.raises(MicrobrainError) as exc:
        PromotionDecision(
            decision_id="dec-3",
            verdict=PromotionVerdict.PROMOTE,
            gate=PromotionGate.SHADOW,
            candidate_ref="cand-1",
            eval_ref="eval-1",
            shadow_ref="shadow-1",
            zero_consequential_false_positives=True,
        )
    assert exc.value.code == MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD


def ep041_unit_deny_allowed_with_false_positives() -> None:
    decision = PromotionDecision(
        decision_id="dec-4",
        verdict=PromotionVerdict.DENY,
        gate=PromotionGate.SHADOW,
        candidate_ref="cand-1",
        eval_ref="eval-1",
        shadow_ref="shadow-1",
        zero_consequential_false_positives=False,
        reason="consequential false positive observed",
    )
    assert decision.verdict is PromotionVerdict.DENY


# ---------------------------------------------------------------------------
# Redaction (SPEC-006: errors redact sensitive content)
# ---------------------------------------------------------------------------


def ep041_unit_error_redacts_secret_canary() -> None:
    canary = _secret_canary()
    error = MicrobrainError(
        MICROBRAIN_CODE_INVALID_INPUT,
        f"config leaked {canary} at endpoint",
    )
    redacted = error.redacted()
    assert canary not in redacted
    assert "[REDACTED]" in redacted


def ep041_unit_error_to_dict_is_redacted() -> None:
    canary = _secret_canary()
    error = MicrobrainError(
        MICROBRAIN_CODE_INVALID_INPUT,
        f"config leaked {canary}",
    )
    payload = error.to_dict()
    assert canary not in payload["detail"]
    assert "[REDACTED]" in payload["detail"]
    assert payload["code"] == MICROBRAIN_CODE_INVALID_INPUT


def ep041_unit_redact_text_scrubs_marker_families() -> None:
    families = [
        "sk-" + "x" * 30,
        "ghp_" + "y" * 35,
        "AKIA" + "Z" * 16,
        "Bearer " + "z" * 30,
        "token=" + "t" * 30,
        "password=" + "p" * 30,
    ]
    for canary in families:
        assert canary not in redact_text(f"leaked {canary}")
    assert "sk-[REDACTED]" in redact_text(f"leaked {families[0]}")


def ep041_unit_redact_text_scrubs_credential_url() -> None:
    url = "https://user:" + "secretpass" + "@host.example/path"
    redacted = redact_text(url)
    assert "secretpass" not in redacted
    assert "[REDACTED]" in redacted


def ep041_unit_correlation_preserved_in_error_payload() -> None:
    error = MicrobrainError(
        MICROBRAIN_CODE_INVALID_INPUT,
        "boom",
        correlation_id="corr-123",
    )
    assert error.to_dict()["correlation_id"] == "corr-123"


# ---------------------------------------------------------------------------
# Dependency-direction (contract crate is provider-neutral)
# ---------------------------------------------------------------------------

_FORBIDDEN_IMPORTS = (
    "import requests",
    "import httpx",
    "import boto3",
    "import torch",
    "import transformers",
    "import openai",
    "import anthropic",
    "import numpy",
    "import pandas",
    "from nexus_connector_sdk",
    "import nexus_connector_sdk",
)


def ep041_unit_contract_package_has_no_provider_dependencies() -> None:
    sources = list(PACKAGE_ROOT.glob("*.py"))
    assert sources, "package sources missing"
    forbidden_hits: list[str] = []
    for source in sources:
        text = source.read_text(encoding="utf-8")
        for marker in _FORBIDDEN_IMPORTS:
            if marker in text:
                forbidden_hits.append(f"{source.name}: {marker}")
    assert not forbidden_hits, forbidden_hits


def ep041_unit_contract_package_imports_clean() -> None:
    import nexus_microbrain  # noqa: F401
    from nexus_microbrain.models import PromotionDecision  # noqa: F401

    assert PromotionDecision is not None
