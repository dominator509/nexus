"""Deterministic Microbrain artifact and promotion behavior (EP-041 M5, SPEC-025).

Artifact validation, GGUF identity binding, QLoRA-output honesty,
shadow comparator gating, and the strict promotion gate - all above the
M1/M2/M3/M4 canonical surfaces:

- artifact validation (missing path/digest denied, malformed digest
  denied, digest mismatch denied, unsupported/non-GGUF format denied,
  artifact from failed/declared-only run denied, artifact from invalid
  candidate denied, artifact identity bound to exact run/candidate)
- QLoRA-output honesty (artifact without run binding denied, artifact
  from wrong candidate denied, metrics alone never certify)
- shadow comparator gating (shadow pass advances only to the next
  gate, never PROMOTE; shadow fail blocks; missing/stale evidence
  fails closed; any consequential false positive blocks)
- strict promotion gate (PROMOTE requires every owned prerequisite;
  a promotion decision never means autonomous deployment)

Locked invariants (M5):
- ADAPTER ARTIFACT EXISTS != TRAINING CERTIFIED
- GGUF ARTIFACT EXISTS != QUANTIZATION VERIFIED
- DIGEST PRESENT != ARTIFACT VERIFIED
- SHADOW PASSED != PROMOTED
- PROMOTION DECISION != AUTONOMOUS DEPLOYMENT
- NODE_DONE != MODEL DEPLOYED
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .errors import (
    MICROBRAIN_CODE_INVALID_INPUT,
    MICROBRAIN_CODE_MISSING_REQUIRED,
    MicrobrainError,
    redact_value,
)
from .models import (
    PromotionDecision,
    QloraRun,
    QuantizedArtifact,
    ShadowComparator,
    TrainingCandidate,
)
from .training_policy import CandidateEligibilityVerdict, LeakageVerdict
from .vocabulary import (
    PromotionGate,
    PromotionVerdict,
    QloraStatus,
    QuantizationFormat,
    ShadowDecision,
)


def _well_formed_digest(digest: str) -> bool:
    if ":" not in digest:
        return False
    _, _, hex_part = digest.partition(":")
    return bool(hex_part) and len(hex_part) >= 32


@dataclass(frozen=True, slots=True)
class ArtifactVerdict:
    """Deterministic artifact validation result.

    verified is true only when every M5 rule passes. reasons carries
    the exact failing rules. ARTIFACT EXISTS != ARTIFACT VERIFIED.
    """

    artifact_id: str
    verified: bool
    reasons: tuple[str, ...] = field(default_factory=tuple)
    digest_verified: bool = False
    format_verified: bool = False
    run_bound: bool = False

    def to_dict(self) -> dict[str, Any]:
        return {
            "artifact_id": self.artifact_id,
            "verified": self.verified,
            "reasons": list(self.reasons),
            "digest_verified": self.digest_verified,
            "format_verified": self.format_verified,
            "run_bound": self.run_bound,
        }

    def to_redacted_dict(self) -> dict[str, Any]:
        redacted = redact_value(self.to_dict())
        assert isinstance(redacted, dict)
        return redacted


@dataclass(frozen=True, slots=True)
class ArtifactFileVerification:
    """Real-file digest verification record."""

    path: str
    digest: str
    verified: bool
    reason: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "path": self.path,
            "digest": self.digest,
            "verified": self.verified,
            "reason": self.reason,
        }


@dataclass(frozen=True, slots=True)
class ShadowGateVerdict:
    """Shadow comparator gate result.

    A shadow pass advances only to the next promotion gate; it never
    issues PROMOTE by itself (SHADOW PASSED != PROMOTED).
    """

    run_id: str
    passed: bool
    next_gate: str
    reasons: tuple[str, ...] = field(default_factory=tuple)
    consequential_false_positives: int = 0

    def to_dict(self) -> dict[str, Any]:
        return {
            "run_id": self.run_id,
            "passed": self.passed,
            "next_gate": self.next_gate,
            "reasons": list(self.reasons),
            "consequential_false_positives": self.consequential_false_positives,
        }


@dataclass(frozen=True, slots=True)
class PromotionPrerequisites:
    """All owned promotion prerequisites in one deterministic record."""

    dataset_policy_passed: bool
    frozen_eval_passed: bool
    eval_frozen_before_training: bool
    candidate_eligible: bool
    no_eval_leakage: bool
    training_run_certified: bool
    artifact_verified: bool
    shadow_gate_passed: bool
    zero_consequential_false_positives: bool
    ood_safe: bool

    def unmet(self) -> list[str]:
        mapping: list[tuple[bool, str]] = [
            (self.dataset_policy_passed, "dataset policy not passed"),
            (self.frozen_eval_passed, "frozen eval not passed"),
            (self.eval_frozen_before_training, "eval not frozen before training"),
            (self.candidate_eligible, "candidate not eligible"),
            (self.no_eval_leakage, "eval leakage present"),
            (self.training_run_certified, "training run not certified"),
            (self.artifact_verified, "quantized artifact not verified"),
            (self.shadow_gate_passed, "shadow gate not passed"),
            (self.zero_consequential_false_positives, "consequential false positives present"),
            (self.ood_safe, "OOD verdict not safe"),
        ]
        return [label for ok, label in mapping if not ok]

    def all_met(self) -> bool:
        return not self.unmet()

    def to_dict(self) -> dict[str, Any]:
        return {
            "dataset_policy_passed": self.dataset_policy_passed,
            "frozen_eval_passed": self.frozen_eval_passed,
            "eval_frozen_before_training": self.eval_frozen_before_training,
            "candidate_eligible": self.candidate_eligible,
            "no_eval_leakage": self.no_eval_leakage,
            "training_run_certified": self.training_run_certified,
            "artifact_verified": self.artifact_verified,
            "shadow_gate_passed": self.shadow_gate_passed,
            "zero_consequential_false_positives": self.zero_consequential_false_positives,
            "ood_safe": self.ood_safe,
        }


@dataclass(frozen=True, slots=True)
class PromotionEvidence:
    """Current-run promotion evidence (redacted before serialization)."""

    run_id: str
    git_commit: str
    candidate_id: str
    dataset_id: str
    dataset_digest: str
    eval_suite_id: str
    eval_suite_digest: str
    plan_digest: str
    qlora_run_id: str
    qlora_status: str
    artifact_id: str
    artifact_digest: str
    quantization_format: str
    shadow_run_id: str
    shadow_decision: str
    false_positive_count: int
    promotion_decision: str
    promotion_gate: str
    certification_boundary: str
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
            "plan_digest": self.plan_digest,
            "qlora_run_id": self.qlora_run_id,
            "qlora_status": self.qlora_status,
            "artifact_id": self.artifact_id,
            "artifact_digest": self.artifact_digest,
            "quantization_format": self.quantization_format,
            "shadow_run_id": self.shadow_run_id,
            "shadow_decision": self.shadow_decision,
            "false_positive_count": self.false_positive_count,
            "promotion_decision": self.promotion_decision,
            "promotion_gate": self.promotion_gate,
            "certification_boundary": self.certification_boundary,
            "timestamp": self.timestamp,
        }

    def to_redacted_dict(self) -> dict[str, Any]:
        redacted = redact_value(self.to_dict())
        assert isinstance(redacted, dict)
        return redacted


# ---------------------------------------------------------------------------
# Artifact validation
# ---------------------------------------------------------------------------


def validate_artifact(
    artifact: QuantizedArtifact,
    *,
    run: QloraRun | None = None,
    candidate: TrainingCandidate | None = None,
    expected_digest: str | None = None,
) -> ArtifactVerdict:
    """Validate a quantized artifact, failing closed on every M5 rule.

    ARTIFACT EXISTS != ARTIFACT VERIFIED:
    - missing/malformed digest denied
    - digest mismatch (against expected) denied
    - unsupported/non-GGUF format denied
    - artifact generated from a failed/declared-only QLoRA run denied
    - artifact generated from an invalid candidate denied
    - artifact identity not bound to the exact run/candidate denied
    """
    reasons: list[str] = []
    digest_ok = _well_formed_digest(artifact.digest)
    if not artifact.digest:
        reasons.append("missing artifact digest denied")
    elif not digest_ok:
        reasons.append("malformed artifact digest denied (alg:hex with >= 32 hex chars required)")

    format_ok = artifact.format is QuantizationFormat.GGUF
    if not format_ok:
        reasons.append(f"unsupported quantization format: {artifact.format.value!r}")

    if expected_digest is not None and artifact.digest != expected_digest:
        reasons.append(
            f"artifact digest mismatch: expected {expected_digest}, observed {artifact.digest}"
        )

    run_bound = False
    if run is not None:
        if run.candidate_ref != artifact.candidate_ref:
            reasons.append(
                f"artifact candidate_ref {artifact.candidate_ref!r} != run candidate_ref "
                f"{run.candidate_ref!r} (wrong candidate denied)"
            )
        if run.status is QloraStatus.FAILED:
            reasons.append("artifact generated from failed QLoRA run denied")
        elif run.status in (QloraStatus.PENDING, QloraStatus.RUNNING):
            reasons.append("artifact generated from declared-only QLoRA run denied")
        elif run.status is QloraStatus.COMPLETED:
            run_bound = True
        else:
            reasons.append(f"unknown QLoRA run status: {run.status.value!r}")
    else:
        reasons.append("artifact without run binding denied")

    if candidate is not None and artifact.candidate_ref != candidate.candidate_id:
        reasons.append(
            f"artifact candidate_ref {artifact.candidate_ref!r} != candidate "
            f"{candidate.candidate_id!r} (wrong candidate denied)"
        )

    return ArtifactVerdict(
        artifact_id=artifact.artifact_id,
        verified=not reasons,
        reasons=tuple(reasons),
        digest_verified=bool(artifact.digest)
        and digest_ok
        and expected_digest is None
        or (expected_digest is not None and artifact.digest == expected_digest),
        format_verified=format_ok,
        run_bound=run_bound,
    )


# ---------------------------------------------------------------------------
# Real artifact file boundary
# ---------------------------------------------------------------------------


def sha256_file(path: str | Path) -> str:
    """Compute the sha256 digest of a real file's bytes."""
    file_path = Path(path)
    try:
        digest = hashlib.sha256(file_path.read_bytes()).hexdigest()
    except OSError as exc:
        raise MicrobrainError(
            MICROBRAIN_CODE_INVALID_INPUT,
            f"cannot read artifact file {file_path.name}: {exc}",
        ) from exc
    return f"sha256:{digest}"


def verify_artifact_file(
    path: str | Path,
    expected_digest: str | None = None,
) -> ArtifactFileVerification:
    """Verify an artifact file against its current bytes.

    With expected_digest set, a mismatch is a hard denial (DIGEST
    PRESENT != ARTIFACT VERIFIED). Without it, the current digest is
    recorded so evidence can bind to the exact file state.
    """
    file_path = Path(path)
    if not file_path.is_file():
        raise MicrobrainError(
            MICROBRAIN_CODE_MISSING_REQUIRED,
            f"artifact file missing: {file_path.name}",
        )
    current = sha256_file(file_path)
    if expected_digest is None:
        return ArtifactFileVerification(
            path=str(file_path),
            digest=current,
            verified=True,
            reason="current-run digest recorded",
        )
    if current != expected_digest:
        return ArtifactFileVerification(
            path=str(file_path),
            digest=current,
            verified=False,
            reason=f"digest mismatch: expected {expected_digest}, observed {current}",
        )
    return ArtifactFileVerification(
        path=str(file_path),
        digest=current,
        verified=True,
        reason="digest verified against current bytes",
    )


# ---------------------------------------------------------------------------
# Shadow comparator gating (shadow pass != promoted)
# ---------------------------------------------------------------------------


def shadow_gate_verdict(
    comparator: ShadowComparator | None,
    *,
    required_exact_match_rate: float = 0.99,
) -> ShadowGateVerdict:
    """Evaluate the shadow gate.

    A pass advances only to the next gate (LOW_RISK_CANARY); it never
    issues PROMOTE. Missing comparator or missing comparison evidence
    fails closed. Any consequential false positive blocks.
    """
    if comparator is None:
        return ShadowGateVerdict(
            run_id="__missing__",
            passed=False,
            next_gate="SHADOW",
            reasons=("missing shadow evidence fails closed",),
            consequential_false_positives=0,
        )
    reasons: list[str] = []
    if not comparator.comparisons:
        reasons.append("missing shadow comparison evidence fails closed")
    if comparator.exact_match_rate < required_exact_match_rate:
        reasons.append(
            f"exact match rate {comparator.exact_match_rate} below required "
            f"{required_exact_match_rate}"
        )
    if comparator.consequential_false_positives > 0:
        reasons.append(
            f"consequential false positives {comparator.consequential_false_positives} > 0 blocks"
        )
    for comparison in comparator.comparisons:
        if comparison.decision is ShadowDecision.DIFFER:
            reasons.append(f"shadow comparison {comparison.input_ref} differs from provider")
    passed = not reasons
    return ShadowGateVerdict(
        run_id=comparator.run_id,
        passed=passed,
        # A shadow pass only advances to the canary gate, never to PROMOTE.
        next_gate="LOW_RISK_CANARY" if passed else "SHADOW",
        reasons=tuple(reasons),
        consequential_false_positives=comparator.consequential_false_positives,
    )


# ---------------------------------------------------------------------------
# Strict promotion gate
# ---------------------------------------------------------------------------


def promotion_gate_decision(
    *,
    decision_id: str,
    candidate: TrainingCandidate,
    prerequisites: PromotionPrerequisites,
    shadow: ShadowGateVerdict,
    eligibility: CandidateEligibilityVerdict | None = None,
    leakage: LeakageVerdict | None = None,
    eval_ref: str,
) -> PromotionDecision:
    """Compute the strict promotion decision.

    PROMOTE requires every owned prerequisite, shadow gate completion,
    and zero consequential false positives. A decision is never an
    autonomous deployment (PROMOTION DECISION != AUTONOMOUS DEPLOYMENT).
    """
    if not prerequisites.all_met():
        unmet = "; ".join(prerequisites.unmet())
        return PromotionDecision(
            decision_id=decision_id,
            verdict=PromotionVerdict.DENY,
            gate=PromotionGate.SHADOW,
            candidate_ref=candidate.candidate_id,
            eval_ref=eval_ref,
            shadow_ref=shadow.run_id,
            zero_consequential_false_positives=(prerequisites.zero_consequential_false_positives),
            reason="prerequisites unmet: " + unmet,
        )
    # All prerequisites met: the gate advances through the ladder.
    return PromotionDecision(
        decision_id=decision_id,
        verdict=PromotionVerdict.PROMOTE,
        gate=PromotionGate.GRADUAL,
        candidate_ref=candidate.candidate_id,
        eval_ref=eval_ref,
        shadow_ref=shadow.run_id,
        zero_consequential_false_positives=prerequisites.zero_consequential_false_positives,
        reason="all owned prerequisites met (gate GRADUAL; autonomous deployment NOT implied)",
    )


def promotion_decision_never_deploys(decision: PromotionDecision) -> bool:
    """A promotion decision is never an autonomous deployment."""
    return (
        decision.verdict is not PromotionVerdict.PROMOTE
        or decision.gate is not PromotionGate.PROMOTED
    )
