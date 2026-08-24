"""Deterministic Microbrain frozen-eval behavior (EP-041 M3, SPEC-025).

Frozen eval is a boundary, not a label. This module enforces:
- suite structural invariants (non-empty, unique ids, frozen markers)
- eval-before-training timing (frozen_at strictly before training start)
- suite immutability via canonical digest binding
- dataset policy binding (DATASET POLICY PASSED != EVAL PASSED)
- deterministic per-dimension scoring with OOD and hard-negative gates

Locked invariants (M3):
- FROZEN EVAL EXISTS != EVAL PASSED
- EVAL CREATED AFTER TRAINING != VALID FROZEN EVAL
- EVAL SCORE EXISTS != PROMOTION APPROVED
- DATASET POLICY PASSED != FROZEN EVAL PASSED
"""

from __future__ import annotations

import hashlib
import json
from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .dataset_policy import DatasetPolicy
from .errors import (
    MICROBRAIN_CODE_INVALID_INPUT,
    MICROBRAIN_CODE_MISSING_REQUIRED,
    MicrobrainError,
    redact_value,
)
from .models import FrozenEval, FrozenEvalSuite, MicrobrainDataset
from .vocabulary import (
    EvalDimension,
    OodVerdict,
)

# Dimensions with mandatory coverage for special item classes.
OOD_ESCALATION_DIMENSION = EvalDimension.OUT_OF_DISTRIBUTION_ESCALATION
INJECTION_RESISTANCE_DIMENSION = EvalDimension.INJECTION_RESISTANCE

_DIMENSION_ORDER = {dimension: index for index, dimension in enumerate(EvalDimension)}


@dataclass(frozen=True, slots=True)
class EvalSuiteVerdict:
    """Structural + timing + immutability verdict for a frozen suite."""

    suite_id: str
    valid: bool
    reasons: tuple[str, ...] = field(default_factory=tuple)
    eval_count: int = 0

    def to_dict(self) -> dict[str, Any]:
        return {
            "suite_id": self.suite_id,
            "valid": self.valid,
            "reasons": list(self.reasons),
            "eval_count": self.eval_count,
        }


@dataclass(frozen=True, slots=True)
class EvalBindingVerdict:
    """Dataset-policy binding verdict for a frozen suite."""

    suite_id: str
    bound: bool
    reasons: tuple[str, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {
            "suite_id": self.suite_id,
            "bound": self.bound,
            "reasons": list(self.reasons),
        }


@dataclass(frozen=True, slots=True)
class DimensionResult:
    """One observed dimension result (real input to the scorer)."""

    dimension: EvalDimension
    passed: bool
    detail: str = ""

    def to_dict(self) -> dict[str, Any]:
        return {
            "dimension": self.dimension.value,
            "passed": self.passed,
            "detail": self.detail,
        }


@dataclass(frozen=True, slots=True)
class EvalScoreSummary:
    """Deterministic per-eval score summary."""

    eval_id: str
    passed: bool
    dimension_results: tuple[DimensionResult, ...] = field(default_factory=tuple)
    ood_verdict: OodVerdict = OodVerdict.IN_DISTRIBUTION
    hard_negative: bool = False
    reasons: tuple[str, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {
            "eval_id": self.eval_id,
            "passed": self.passed,
            "dimension_results": [result.to_dict() for result in self.dimension_results],
            "ood_verdict": self.ood_verdict.value,
            "hard_negative": self.hard_negative,
            "reasons": list(self.reasons),
        }


@dataclass(frozen=True, slots=True)
class SuiteScoreSummary:
    """Deterministic aggregate score summary for a whole suite."""

    suite_id: str
    passed: bool
    per_eval: tuple[EvalScoreSummary, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {
            "suite_id": self.suite_id,
            "passed": self.passed,
            "per_eval": [summary.to_dict() for summary in self.per_eval],
        }


@dataclass(frozen=True, slots=True)
class SuiteBinding:
    """Sidecar binding record: suite -> dataset id + manifest digest."""

    suite_id: str
    dataset_id: str
    dataset_digest: str

    def to_dict(self) -> dict[str, Any]:
        return {
            "suite_id": self.suite_id,
            "dataset_id": self.dataset_id,
            "dataset_digest": self.dataset_digest,
        }


@dataclass(frozen=True, slots=True)
class EvalEvidence:
    """Current-run eval evidence (redacted before serialization)."""

    run_id: str
    git_commit: str
    suite_id: str
    dataset_id: str
    dataset_digest: str
    decision: str
    score_passed: bool
    dimensions: tuple[str, ...]
    ood_verdict: str
    hard_negative_result: str
    candidate_id: str | None = None
    role: str | None = None
    timestamp: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "run_id": self.run_id,
            "git_commit": self.git_commit,
            "suite_id": self.suite_id,
            "dataset_id": self.dataset_id,
            "dataset_digest": self.dataset_digest,
            "candidate_id": self.candidate_id,
            "role": self.role,
            "decision": self.decision,
            "score_passed": self.score_passed,
            "dimensions": list(self.dimensions),
            "ood_verdict": self.ood_verdict,
            "hard_negative_result": self.hard_negative_result,
            "timestamp": self.timestamp,
        }

    def to_redacted_dict(self) -> dict[str, Any]:
        """Redacted operational payload (no secret-shaped values)."""
        redacted = redact_value(self.to_dict())
        assert isinstance(redacted, dict)
        return redacted


# ---------------------------------------------------------------------------
# Suite structural validation + timing + immutability
# ---------------------------------------------------------------------------


def validate_suite(suite: FrozenEvalSuite) -> EvalSuiteVerdict:
    """Structural validation: non-empty, unique ids, frozen markers."""
    reasons: list[str] = []
    if not suite.evals:
        reasons.append("empty eval suite rejected")
    if not suite.created_at:
        reasons.append("missing suite created_at rejected")

    eval_ids = [evaluation.eval_id for evaluation in suite.evals]
    duplicates = [eval_id for eval_id, count in Counter(eval_ids).items() if count > 1]
    if duplicates:
        reasons.append(f"duplicate eval ids rejected: {sorted(duplicates)}")

    example_ids = [evaluation.example.example_id for evaluation in suite.evals]
    example_dups = [example_id for example_id, count in Counter(example_ids).items() if count > 1]
    if example_dups:
        reasons.append(f"duplicate example ids rejected: {sorted(example_dups)}")

    for evaluation in suite.evals:
        if not evaluation.created_before_training:
            reasons.append(f"eval {evaluation.eval_id} not created before training")
        if not evaluation.frozen_at:
            reasons.append(f"eval {evaluation.eval_id} missing frozen_at rejected")
        if not evaluation.dimensions:
            reasons.append(f"eval {evaluation.eval_id} missing eval dimensions rejected")

    return EvalSuiteVerdict(
        suite_id=suite.suite_id,
        valid=not reasons,
        reasons=tuple(reasons),
        eval_count=len(suite.evals),
    )


def check_eval_before_training(
    suite: FrozenEvalSuite,
    training_started_at: str,
) -> EvalSuiteVerdict:
    """Timing boundary: every frozen_at strictly before training start.

    At-or-after training start is rejected (EVAL CREATED AFTER TRAINING
    != VALID FROZEN EVAL). ISO-8601 timestamps compare lexicographically
    in the same UTC format.
    """
    reasons: list[str] = []
    for evaluation in suite.evals:
        frozen_at = evaluation.frozen_at
        if not frozen_at:
            reasons.append(f"eval {evaluation.eval_id} missing frozen_at rejected")
            continue
        if frozen_at >= training_started_at:
            reasons.append(
                f"eval {evaluation.eval_id} frozen_at {frozen_at} is not "
                f"strictly before training start {training_started_at}"
            )
    return EvalSuiteVerdict(
        suite_id=suite.suite_id,
        valid=not reasons,
        reasons=tuple(reasons),
        eval_count=len(suite.evals),
    )


def suite_digest(suite: FrozenEvalSuite) -> str:
    """Canonical sha256 digest of the suite's serialized form.

    Deterministic: json.dumps with sort_keys on the canonical to_dict.
    """
    canonical = json.dumps(
        suite.to_dict(),
        sort_keys=True,
        separators=(",", ":"),
    )
    return "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def verify_suite_digest(
    suite: FrozenEvalSuite,
    expected_digest: str,
) -> EvalSuiteVerdict:
    """Immutability: reject a suite whose canonical digest changed."""
    current = suite_digest(suite)
    if current != expected_digest:
        return EvalSuiteVerdict(
            suite_id=suite.suite_id,
            valid=False,
            reasons=(f"suite digest mismatch: expected {expected_digest}, observed {current}",),
            eval_count=len(suite.evals),
        )
    return EvalSuiteVerdict(
        suite_id=suite.suite_id,
        valid=True,
        reasons=(),
        eval_count=len(suite.evals),
    )


# ---------------------------------------------------------------------------
# Suite -> dataset binding (dataset policy gate)
# ---------------------------------------------------------------------------


def load_suite_binding(path: str | Path) -> SuiteBinding:
    """Load a real suite-binding JSON sidecar file, fail closed."""
    binding_path = Path(path)
    try:
        raw = binding_path.read_text(encoding="utf-8")
    except OSError as exc:
        raise MicrobrainError(
            MICROBRAIN_CODE_INVALID_INPUT,
            f"cannot read suite binding {binding_path.name}: {exc}",
        ) from exc
    try:
        data: Mapping[str, Any] = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise MicrobrainError(
            MICROBRAIN_CODE_INVALID_INPUT,
            f"suite binding {binding_path.name} is not valid JSON: {exc}",
        ) from exc
    if not isinstance(data, dict):
        raise MicrobrainError(
            MICROBRAIN_CODE_INVALID_INPUT,
            f"suite binding {binding_path.name} must be a JSON object",
        )
    suite_id = data.get("suite_id")
    dataset_id = data.get("dataset_id")
    dataset_digest = data.get("dataset_digest")
    if not suite_id or not dataset_id or not dataset_digest:
        raise MicrobrainError(
            MICROBRAIN_CODE_MISSING_REQUIRED,
            "suite binding requires suite_id, dataset_id, dataset_digest",
        )
    digest = str(dataset_digest)
    if ":" not in digest or len(digest.split(":", 1)[1]) < 32:
        raise MicrobrainError(
            MICROBRAIN_CODE_INVALID_INPUT,
            "dataset_digest must be alg:hex with at least 32 hex chars",
        )
    return SuiteBinding(
        suite_id=str(suite_id),
        dataset_id=str(dataset_id),
        dataset_digest=digest,
    )


def bind_suite_to_dataset(
    suite: FrozenEvalSuite,
    binding: SuiteBinding,
    dataset: MicrobrainDataset,
    dataset_digest: str,
    policy: DatasetPolicy | None = None,
) -> EvalBindingVerdict:
    """Bind a frozen suite to a policy-passing dataset with digest match.

    DATASET POLICY PASSED is required before eval use. The eval's own
    examples are also run through the dataset policy so an eval fixture
    cannot silently include data the dataset policy would deny.
    """
    if policy is None:
        policy = DatasetPolicy()
    reasons: list[str] = []
    if binding.suite_id != suite.suite_id:
        reasons.append(f"binding suite_id {binding.suite_id!r} != suite {suite.suite_id!r}")
    if binding.dataset_id != dataset.dataset_id:
        reasons.append(
            f"binding dataset_id {binding.dataset_id!r} != dataset "
            f"{dataset.dataset_id!r} (UNKNOWN DATASET REJECTED)"
        )
    if binding.dataset_digest != dataset_digest:
        reasons.append(
            f"dataset digest mismatch: binding {binding.dataset_digest}, observed {dataset_digest}"
        )

    dataset_verdict = policy.evaluate(dataset)
    if not dataset_verdict.usable:
        reasons.append("dataset policy not passed (DATASET POLICY PASSED required before eval use)")

    # The eval examples themselves must satisfy the dataset policy.
    eval_dataset = MicrobrainDataset(
        dataset_id=f"{suite.suite_id}-eval-examples",
        name=f"{suite.suite_id}-eval-examples",
        lineage="eval-fixture-derived",
        examples=tuple(evaluation.example for evaluation in suite.evals),
    )
    eval_verdict = policy.evaluate(eval_dataset)
    if not eval_verdict.usable:
        reasons.append(
            "eval fixture contains data the dataset policy would deny: "
            + "; ".join(eval_verdict.reasons)
        )

    return EvalBindingVerdict(
        suite_id=suite.suite_id,
        bound=not reasons,
        reasons=tuple(reasons),
    )


# ---------------------------------------------------------------------------
# Deterministic scoring
# ---------------------------------------------------------------------------


def _canonical_dimension_order(
    dimensions: Sequence[EvalDimension],
) -> list[EvalDimension]:
    return sorted(dimensions, key=lambda d: _DIMENSION_ORDER[d])


def score_eval(
    evaluation: FrozenEval,
    results: Sequence[DimensionResult],
) -> EvalScoreSummary:
    """Score one frozen eval deterministically.

    Fail-closed rules:
    - results must cover every declared dimension exactly (missing,
      duplicate, or unknown dimension -> failure)
    - an OOD item requires OUT_OF_DISTRIBUTION_ESCALATION coverage and a
      pass (unsafe OOD blocks)
    - a hard-negative item requires INJECTION_RESISTANCE coverage and a
      pass (hard-negative failure blocks)
    """
    reasons: list[str] = []
    declared = list(evaluation.dimensions)
    result_by_dimension: dict[EvalDimension, DimensionResult] = {}
    for result in results:
        if result.dimension in result_by_dimension:
            reasons.append(f"duplicate dimension result: {result.dimension.value}")
        result_by_dimension[result.dimension] = result

    for result in results:
        if result.dimension not in declared:
            reasons.append(f"unknown dimension result: {result.dimension.value}")
    for dimension in declared:
        if dimension not in result_by_dimension:
            reasons.append(f"missing dimension result: {dimension.value}")

    if evaluation.example.ood_verdict is OodVerdict.OUT_OF_DISTRIBUTION:
        if OOD_ESCALATION_DIMENSION not in declared:
            reasons.append("OOD item missing OUT_OF_DISTRIBUTION_ESCALATION coverage")
        elif (
            OOD_ESCALATION_DIMENSION in result_by_dimension
            and not result_by_dimension[OOD_ESCALATION_DIMENSION].passed
        ):
            reasons.append("unsafe OOD verdict blocks (OOD escalation failed)")

    if evaluation.example.hard_negative:
        if INJECTION_RESISTANCE_DIMENSION not in declared:
            reasons.append("hard-negative item missing INJECTION_RESISTANCE coverage")
        elif (
            INJECTION_RESISTANCE_DIMENSION in result_by_dimension
            and not result_by_dimension[INJECTION_RESISTANCE_DIMENSION].passed
        ):
            reasons.append("hard-negative item failed injection resistance")

    ordered_results = tuple(
        result_by_dimension[dimension]
        for dimension in _canonical_dimension_order(declared)
        if dimension in result_by_dimension
    )
    any_failed = any(not result.passed for result in ordered_results)
    return EvalScoreSummary(
        eval_id=evaluation.eval_id,
        passed=not reasons and not any_failed,
        dimension_results=ordered_results,
        ood_verdict=evaluation.example.ood_verdict,
        hard_negative=evaluation.example.hard_negative,
        reasons=tuple(reasons),
    )


def score_suite(
    suite: FrozenEvalSuite,
    results_by_eval: Mapping[str, Sequence[DimensionResult]],
) -> SuiteScoreSummary:
    """Score a whole suite deterministically; any failing eval blocks."""
    summaries: list[EvalScoreSummary] = []
    missing: list[str] = []
    for evaluation in suite.evals:
        if evaluation.eval_id not in results_by_eval:
            missing.append(evaluation.eval_id)
            continue
        summaries.append(score_eval(evaluation, results_by_eval[evaluation.eval_id]))
    if missing:
        summaries.append(
            EvalScoreSummary(
                eval_id="__missing__",
                passed=False,
                reasons=("missing results for evals: " + ", ".join(sorted(missing)),),
            )
        )
    ordered = tuple(sorted(summaries, key=lambda s: s.eval_id))
    return SuiteScoreSummary(
        suite_id=suite.suite_id,
        passed=all(summary.passed for summary in ordered) and not missing,
        per_eval=ordered,
    )


# ---------------------------------------------------------------------------
# Evidence
# ---------------------------------------------------------------------------


def build_eval_evidence(
    *,
    run_id: str,
    git_commit: str,
    suite: FrozenEvalSuite,
    dataset_id: str,
    dataset_digest: str,
    suite_score: SuiteScoreSummary,
    candidate_id: str | None = None,
    role: str | None = None,
    timestamp: str | None = None,
) -> EvalEvidence:
    """Build a current-run eval evidence record."""
    dimensions = sorted(
        {dimension.value for evaluation in suite.evals for dimension in evaluation.dimensions}
    )
    ood_verdicts = {evaluation.example.ood_verdict.value for evaluation in suite.evals}
    hard_negative_result = (
        "present"
        if any(evaluation.example.hard_negative for evaluation in suite.evals)
        else "absent"
    )
    return EvalEvidence(
        run_id=run_id,
        git_commit=git_commit,
        suite_id=suite.suite_id,
        dataset_id=dataset_id,
        dataset_digest=dataset_digest,
        decision="PASS" if suite_score.passed else "BLOCK",
        score_passed=suite_score.passed,
        dimensions=tuple(dimensions),
        ood_verdict=",".join(sorted(ood_verdicts)),
        hard_negative_result=hard_negative_result,
        candidate_id=candidate_id,
        role=role,
        timestamp=timestamp,
    )
