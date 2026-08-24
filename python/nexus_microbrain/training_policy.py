"""Deterministic Microbrain training-candidate behavior (EP-041 M4, SPEC-025).

Training candidate eligibility is a boundary, not a label. This module
enforces, above the M1/M2/M3 canonical surfaces:

- candidate eligibility (dataset reference, policy pass, digest match,
  eval reference and policy pass, eval-before-training timing,
  narrow canonical role, provenance/license/privacy proof)
- training plan behavior (PLAN_READY only - a plan never implies a
  QLoRA success, an artifact, or a promotion decision)
- QloraRun contract honesty (declared != executed; no certification
  without real start/end evidence and an output digest)
- train/eval leakage checks (eval items never reused as training data)
- narrow-role enforcement (exactly one of the 8 canonical roles)
- current-run redacted training evidence

Locked invariants (M4):
- TRAINING CANDIDATE EXISTS != ELIGIBLE TO TRAIN
- TRAINING PLAN EXISTS != TRAINING EXECUTED
- QLORA RUN EXISTS != TRAINING CERTIFIED
- TRAINING METRICS EXIST != MODEL QUALITY CERTIFIED
- EVAL SCORE EXISTS != PROMOTION APPROVED
"""

from __future__ import annotations

import hashlib
import json
from collections.abc import Mapping
from dataclasses import dataclass, field
from typing import Any

from .dataset_policy import DatasetPolicy
from .errors import (
    MICROBRAIN_CODE_INVALID_INPUT,
    MICROBRAIN_CODE_MISSING_REQUIRED,
    MICROBRAIN_CODE_UNVERIFIED,
    MicrobrainError,
    redact_value,
)
from .eval_policy import (
    SuiteBinding,
    SuiteScoreSummary,
    check_eval_before_training,
    suite_digest,
    validate_suite,
)
from .models import FrozenEvalSuite, MicrobrainDataset, QloraRun, TrainingCandidate
from .vocabulary import (
    QloraStatus,
    QuantizationFormat,
    Role,
)

# The 8 canonical narrow NexusControlObject interpretation roles.
NARROW_ROLES: frozenset[Role] = frozenset(Role)

# Canonical training plan hyperparameter keys (all required, positive).
REQUIRED_PLAN_HYPERPARAMETERS: tuple[str, ...] = ("rank", "alpha", "seed")

# QLoRA statuses that never imply execution evidence.
_DECLARED_STATUSES: frozenset[QloraStatus] = frozenset({QloraStatus.PENDING, QloraStatus.RUNNING})


@dataclass(frozen=True, slots=True)
class CandidateEligibilityVerdict:
    """Deterministic candidate eligibility result.

    eligible is true only when every M4 rule passes. reasons carries the
    exact failing rules. A candidate object existing is never enough:
    dataset policy, eval policy, timing, role, and provenance all gate.
    """

    candidate_id: str
    eligible: bool
    reasons: tuple[str, ...] = field(default_factory=tuple)
    role: str | None = None
    dataset_id: str | None = None
    eval_suite_id: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "candidate_id": self.candidate_id,
            "eligible": self.eligible,
            "reasons": list(self.reasons),
            "role": self.role,
            "dataset_id": self.dataset_id,
            "eval_suite_id": self.eval_suite_id,
        }

    def to_redacted_dict(self) -> dict[str, Any]:
        redacted = redact_value(self.to_dict())
        assert isinstance(redacted, dict)
        return redacted


@dataclass(frozen=True, slots=True)
class TrainingPlan:
    """A deterministic training plan (PLAN_READY only, never executed).

    plan_digest is the canonical sha256 of the plan's serialized form;
    verifying it proves the plan is what was reviewed, not that any
    training happened.
    """

    plan_id: str
    candidate_ref: str
    role: Role
    base_model: str
    quantization_format: QuantizationFormat
    hyperparameters: dict[str, int | float]
    created_at: str
    plan_digest: str

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": "1",
            "plan_id": self.plan_id,
            "candidate_ref": self.candidate_ref,
            "role": self.role.value,
            "base_model": self.base_model,
            "quantization_format": self.quantization_format.value,
            "hyperparameters": dict(self.hyperparameters),
            "created_at": self.created_at,
            "plan_digest": self.plan_digest,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> TrainingPlan:
        plan_id = data.get("plan_id")
        candidate_ref = data.get("candidate_ref")
        role_raw = data.get("role")
        base_model = data.get("base_model")
        qformat_raw = data.get("quantization_format")
        hyper = data.get("hyperparameters")
        created_at = data.get("created_at")
        plan_digest = data.get("plan_digest")
        if not plan_id or not candidate_ref or not base_model or not created_at or not plan_digest:
            raise MicrobrainError(
                MICROBRAIN_CODE_MISSING_REQUIRED,
                "training plan requires plan_id, candidate_ref, base_model, "
                "created_at, plan_digest",
            )
        if not isinstance(hyper, dict) or not hyper:
            raise MicrobrainError(
                MICROBRAIN_CODE_MISSING_REQUIRED,
                "training plan requires non-empty hyperparameters",
            )
        return cls(
            plan_id=str(plan_id),
            candidate_ref=str(candidate_ref),
            role=Role.parse(role_raw),
            base_model=str(base_model),
            quantization_format=QuantizationFormat.parse(qformat_raw),
            hyperparameters={
                str(k): int(v) if isinstance(v, bool) else v for k, v in hyper.items()
            },
            created_at=str(created_at),
            plan_digest=str(plan_digest),
        )

    def _validate_hyperparameters(self) -> None:
        missing = [key for key in REQUIRED_PLAN_HYPERPARAMETERS if key not in self.hyperparameters]
        if missing:
            raise MicrobrainError(
                MICROBRAIN_CODE_MISSING_REQUIRED,
                f"training plan missing hyperparameters: {sorted(missing)}",
            )
        for key in REQUIRED_PLAN_HYPERPARAMETERS:
            value = self.hyperparameters[key]
            if not isinstance(value, int) or value <= 0:
                raise MicrobrainError(
                    MICROBRAIN_CODE_INVALID_INPUT,
                    f"hyperparameter {key} must be a positive integer",
                )

    def validate(self) -> None:
        """Fail-closed structural validation (defense in depth)."""
        if not self.plan_id.strip() or not self.candidate_ref.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "plan_id and candidate_ref must not be empty",
            )
        if self.role not in NARROW_ROLES:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                f"unsupported target role: {self.role.value!r}",
            )
        if not self.base_model.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "base_model must not be empty",
            )
        if self.quantization_format is not QuantizationFormat.GGUF:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                f"unsupported quantization format: {self.quantization_format.value!r}",
            )
        self._validate_hyperparameters()
        if plan_digest(self) != self.plan_digest:
            raise MicrobrainError(
                MICROBRAIN_CODE_UNVERIFIED,
                "plan_digest does not match the plan's canonical digest",
            )


@dataclass(frozen=True, slots=True)
class PlanVerdict:
    """Training plan verdict: PLAN_READY only - never executed."""

    plan_id: str
    state: str
    digest_verified: bool
    reasons: tuple[str, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {
            "plan_id": self.plan_id,
            "state": self.state,
            "digest_verified": self.digest_verified,
            "reasons": list(self.reasons),
        }


@dataclass(frozen=True, slots=True)
class QloraRunVerdict:
    """QLoRA run contract verdict (honest: declared != executed)."""

    run_id: str
    certified: bool
    status: str
    reasons: tuple[str, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {
            "run_id": self.run_id,
            "certified": self.certified,
            "status": self.status,
            "reasons": list(self.reasons),
        }


@dataclass(frozen=True, slots=True)
class LeakageVerdict:
    """Train/eval leakage check result (absent evidence fails closed)."""

    checked: bool
    clean: bool
    reasons: tuple[str, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {
            "checked": self.checked,
            "clean": self.clean,
            "reasons": list(self.reasons),
        }


@dataclass(frozen=True, slots=True)
class TrainingEvidence:
    """Current-run training evidence (redacted before serialization)."""

    run_id: str
    git_commit: str
    candidate_id: str
    dataset_id: str
    dataset_digest: str
    eval_suite_id: str
    eval_suite_digest: str
    role: str
    plan_digest: str
    eligibility: bool
    leakage_clean: bool
    qlora_status: str
    decision: str
    timestamp: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "run_id": self.run_id,
            "git_commit": self.git_commit,
            "candidate_id": self.candidate_id,
            "dataset_id": self.dataset_id,
            "dataset_digest": self.dataset_digest,
            "eval_suite_id": self.eval_suite_id,
            "eval_suite_digest": self.eval_suite_digest,
            "role": self.role,
            "plan_digest": self.plan_digest,
            "eligibility": self.eligibility,
            "leakage_clean": self.leakage_clean,
            "qlora_status": self.qlora_status,
            "decision": self.decision,
            "timestamp": self.timestamp,
        }

    def to_redacted_dict(self) -> dict[str, Any]:
        redacted = redact_value(self.to_dict())
        assert isinstance(redacted, dict)
        return redacted


# ---------------------------------------------------------------------------
# Narrow-role enforcement
# ---------------------------------------------------------------------------


def narrow_role(role: Role) -> bool:
    """A candidate may only hold one of the 8 canonical narrow roles."""
    return role in NARROW_ROLES


# ---------------------------------------------------------------------------
# Candidate eligibility
# ---------------------------------------------------------------------------


def check_candidate_eligibility(
    candidate: TrainingCandidate,
    *,
    dataset: MicrobrainDataset,
    dataset_digest: str,
    suite: FrozenEvalSuite | None,
    binding: SuiteBinding | None,
    suite_score: SuiteScoreSummary | None,
    training_started_at: str,
    policy: DatasetPolicy | None = None,
) -> CandidateEligibilityVerdict:
    """Determine whether a candidate is eligible to train.

    Every gate fails closed with a typed reason:
    - missing/unknown dataset reference -> denied
    - dataset policy fail -> denied
    - dataset digest mismatch -> denied
    - missing eval reference / eval policy fail -> denied
    - eval not frozen before training -> denied
    - role unknown or too broad -> denied
    - missing provenance / license / privacy proof -> denied

    CANDIDATE OBJECT EXISTS != ELIGIBLE TO TRAIN.
    """
    if policy is None:
        policy = DatasetPolicy()
    reasons: list[str] = []

    if not candidate.dataset_ref.strip():
        reasons.append("missing dataset reference denied")
    if candidate.dataset_ref != dataset.dataset_id:
        reasons.append(
            f"unknown dataset reference: {candidate.dataset_ref!r} != {dataset.dataset_id!r}"
        )
    if not dataset_digest.startswith("sha256:") or len(dataset_digest.split(":", 1)[1]) < 32:
        reasons.append("dataset digest missing or malformed")
    else:
        dataset_verdict = policy.evaluate(dataset)
        if not dataset_verdict.usable:
            reasons.append("dataset policy not passed: " + "; ".join(dataset_verdict.reasons))
        if not dataset_verdict.licensed:
            reasons.append("dataset license proof missing (LICENSE PRESENT != LICENSE VERIFIED)")
        if not dataset_verdict.privacy_safe:
            reasons.append("dataset privacy proof missing (LICENSED != PRIVACY SAFE)")

    if suite is None:
        reasons.append("missing eval reference denied")
    else:
        if binding is None:
            reasons.append("missing eval suite binding denied")
        else:
            if binding.suite_id != suite.suite_id:
                reasons.append(f"binding suite_id {binding.suite_id!r} != suite {suite.suite_id!r}")
            if binding.dataset_id != dataset.dataset_id:
                reasons.append(
                    f"binding dataset_id {binding.dataset_id!r} != dataset {dataset.dataset_id!r}"
                )
            if binding.dataset_digest != dataset_digest:
                reasons.append("binding dataset digest mismatch")
        structural = validate_suite(suite)
        if not structural.valid:
            reasons.append("eval suite structurally invalid: " + "; ".join(structural.reasons))
        timing = check_eval_before_training(suite, training_started_at)
        if not timing.valid:
            reasons.append("eval not frozen before training: " + "; ".join(timing.reasons))
        if suite_score is None:
            reasons.append("missing eval score (EVAL POLICY FAIL -> DENIED)")
        elif not suite_score.passed:
            reasons.append("eval policy not passed (EVAL SCORE EXISTS != PROMOTION APPROVED)")

    if not narrow_role(candidate.role):
        reasons.append(f"role too broad or unknown: {candidate.role.value!r}")

    if not candidate.base_model.strip() or not candidate.model_ref.strip():
        reasons.append("candidate missing model identity (provenance proof)")

    return CandidateEligibilityVerdict(
        candidate_id=candidate.candidate_id,
        eligible=not reasons,
        reasons=tuple(reasons),
        role=candidate.role.value,
        dataset_id=dataset.dataset_id,
        eval_suite_id=suite.suite_id if suite is not None else None,
    )


# ---------------------------------------------------------------------------
# Training plan behavior (plan exists != training executed)
# ---------------------------------------------------------------------------


def plan_digest(plan: TrainingPlan) -> str:
    """Canonical sha256 digest of a training plan's serialized form."""
    payload = {
        "plan_id": plan.plan_id,
        "candidate_ref": plan.candidate_ref,
        "role": plan.role.value,
        "base_model": plan.base_model,
        "quantization_format": plan.quantization_format.value,
        "hyperparameters": {k: plan.hyperparameters[k] for k in sorted(plan.hyperparameters)},
        "created_at": plan.created_at,
    }
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    return "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def build_training_plan(
    *,
    plan_id: str,
    candidate_ref: str,
    role: Role,
    base_model: str,
    quantization_format: QuantizationFormat,
    hyperparameters: dict[str, int | float],
    created_at: str,
) -> TrainingPlan:
    """Build a validated training plan with its canonical digest."""
    plan = TrainingPlan(
        plan_id=plan_id,
        candidate_ref=candidate_ref,
        role=role,
        base_model=base_model,
        quantization_format=quantization_format,
        hyperparameters=dict(hyperparameters),
        created_at=created_at,
        plan_digest="",
    )
    digest = plan_digest(plan)
    plan = TrainingPlan(
        plan_id=plan_id,
        candidate_ref=candidate_ref,
        role=role,
        base_model=base_model,
        quantization_format=quantization_format,
        hyperparameters=dict(hyperparameters),
        created_at=created_at,
        plan_digest=digest,
    )
    plan.validate()
    return plan


def verify_plan_digest(plan: TrainingPlan, expected_digest: str) -> PlanVerdict:
    """Verify a plan's digest deterministically (tamper -> denied)."""
    if plan_digest(plan) != expected_digest:
        return PlanVerdict(
            plan_id=plan.plan_id,
            state="DENIED",
            digest_verified=False,
            reasons=(
                f"plan digest mismatch: expected {expected_digest}, observed {plan_digest(plan)}",
            ),
        )
    return PlanVerdict(
        plan_id=plan.plan_id,
        state="PLAN_READY",
        digest_verified=True,
        reasons=(),
    )


def training_plan_verdict(plan: TrainingPlan) -> PlanVerdict:
    """A valid plan is PLAN_READY only - it never executes training.

    The verdict deliberately carries no QLoRA success, no artifact, and
    no promotion decision (TRAINING PLAN EXISTS != TRAINING EXECUTED).
    """
    try:
        plan.validate()
    except MicrobrainError as exc:
        return PlanVerdict(
            plan_id=plan.plan_id,
            state="DENIED",
            digest_verified=False,
            reasons=(exc.redacted(),),
        )
    return PlanVerdict(
        plan_id=plan.plan_id,
        state="PLAN_READY",
        digest_verified=True,
        reasons=(),
    )


# ---------------------------------------------------------------------------
# QLoRA run contract honesty (declared != executed)
# ---------------------------------------------------------------------------


def qlora_run_verdict(
    run: QloraRun,
    *,
    candidate_eligible: bool,
    output_digest: str | None = None,
    start_evidence: str | None = None,
    end_evidence: str | None = None,
) -> QloraRunVerdict:
    """QLoRA run contract verdict without real execution.

    Certified only when: the candidate was eligible, the run reached a
    terminal COMPLETED status, start and end evidence exist, and a
    well-formed output digest exists. Metrics alone never certify.
    """
    reasons: list[str] = []
    if not candidate_eligible:
        reasons.append("run after invalid candidate denied")
    if run.status in _DECLARED_STATUSES:
        reasons.append(f"run declared ({run.status.value}) but not executed")
    if run.status is QloraStatus.FAILED:
        reasons.append("failed run not certified")
    if run.status is not QloraStatus.COMPLETED:
        reasons.append(f"run status {run.status.value} lacks completion evidence")
    if not start_evidence:
        reasons.append("missing start evidence")
    if not end_evidence:
        reasons.append("missing end evidence")
    if not output_digest:
        reasons.append("training output digest missing")
    elif ":" not in output_digest or len(output_digest.split(":", 1)[1]) < 32:
        reasons.append("training output digest malformed (alg:hex with >= 32 hex chars required)")

    certified = (
        not reasons
        and run.status is QloraStatus.COMPLETED
        and bool(start_evidence and end_evidence and output_digest)
    )
    return QloraRunVerdict(
        run_id=run.run_id,
        certified=certified,
        status=run.status.value,
        reasons=tuple(reasons),
    )


# ---------------------------------------------------------------------------
# Train/eval leakage checks
# ---------------------------------------------------------------------------


def _example_digest(example: Any) -> str:
    """Canonical digest of one example's serialized form."""
    canonical = json.dumps(example.to_dict(), sort_keys=True, separators=(",", ":"))
    return "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def check_no_eval_leakage(
    suite: FrozenEvalSuite | None,
    dataset: MicrobrainDataset | None,
) -> LeakageVerdict:
    """Prove the frozen eval set is disjoint from training data.

    Missing evidence fails closed: without both the suite and the
    dataset, the check cannot attest absence of leakage.
    """
    if suite is None or dataset is None:
        return LeakageVerdict(
            checked=False,
            clean=False,
            reasons=("missing leakage evidence (suite or dataset absent) fails closed",),
        )

    reasons: list[str] = []
    eval_example_ids = {evaluation.example.example_id for evaluation in suite.evals}
    train_example_ids = {example.example_id for example in dataset.examples}
    id_collisions = sorted(eval_example_ids & train_example_ids)
    if id_collisions:
        reasons.append(f"eval item reused as training example: {id_collisions}")

    eval_correlation_ids = {
        evaluation.example.correlation_id
        for evaluation in suite.evals
        if evaluation.example.correlation_id
    }
    train_correlation_ids = {
        example.correlation_id for example in dataset.examples if example.correlation_id
    }
    corr_collisions = sorted(eval_correlation_ids & train_correlation_ids)
    if corr_collisions:
        reasons.append(f"same correlation_id train/eval collision: {corr_collisions}")

    eval_digests = {_example_digest(evaluation.example) for evaluation in suite.evals}
    train_digests = {_example_digest(example) for example in dataset.examples}
    digest_collisions = sorted(eval_digests & train_digests)
    if digest_collisions:
        reasons.append(
            f"same example digest train/eval collision: {len(digest_collisions)} item(s)"
        )

    return LeakageVerdict(
        checked=True,
        clean=not reasons,
        reasons=tuple(reasons),
    )


# ---------------------------------------------------------------------------
# Current-run training evidence
# ---------------------------------------------------------------------------


def build_training_evidence(
    *,
    run_id: str,
    git_commit: str,
    candidate: TrainingCandidate,
    dataset_id: str,
    dataset_digest: str,
    eval_suite: FrozenEvalSuite,
    eval_suite_digest: str,
    plan: TrainingPlan,
    eligibility: CandidateEligibilityVerdict,
    leakage: LeakageVerdict,
    qlora_status: QloraStatus,
    timestamp: str | None = None,
) -> TrainingEvidence:
    """Build a current-run training evidence record (redacted)."""
    decision = "READY_TO_TRAIN" if (eligibility.eligible and leakage.clean) else "BLOCK"
    return TrainingEvidence(
        run_id=run_id,
        git_commit=git_commit,
        candidate_id=candidate.candidate_id,
        dataset_id=dataset_id,
        dataset_digest=dataset_digest,
        eval_suite_id=eval_suite.suite_id,
        eval_suite_digest=eval_suite_digest,
        role=candidate.role.value,
        plan_digest=plan.plan_digest,
        eligibility=eligibility.eligible,
        leakage_clean=leakage.clean,
        qlora_status=qlora_status.value,
        decision=decision,
        timestamp=timestamp,
    )


def suite_digest_of(suite: FrozenEvalSuite) -> str:
    """Expose the canonical eval suite digest for evidence binding."""
    return suite_digest(suite)
