"""AUD-064 remediation proofs (RX-019 M6).

AUD-064: EP-041 closes a QLoRA/GGUF training node without executing
training or quantization - the promotion path issued PROMOTE from
caller-asserted prerequisites with no DECLARED promotion/evaluation
evidence binding, and the M5 evidence proof even recorded a PENDING
(declared-only) run beside a PROMOTE verdict.

Fix (enforced in python/nexus_microbrain/artifact_policy.py):
- declared_promotion_evidence_gate binds a PROMOTE decision to the
  declared PromotionEvidence record: matching candidate, COMPLETED
  (executed) QLoRA run, well-formed bound digests, matching shadow run,
  zero false positives.
- promotion_gate_decision fails closed: PROMOTE requires the declared
  evidence; missing/inconsistent evidence => DENY.

Real model training/quantization execution remains LOGGED as GAP-004
(Dominic directive 2026-09-02): this gate makes promotion impossible
without executed-training evidence, but producing real GGUF artifacts
requires a GPU/toolchain this host does not have.
"""

from __future__ import annotations

import json
from pathlib import Path

from nexus_microbrain import (
    PromotionEvidence,
    PromotionGate,
    PromotionPrerequisites,
    PromotionVerdict,
    QloraStatus,
    ShadowComparator,
    ShadowComparison,
    ShadowDecision,
    ShadowGateVerdict,
    TrainingCandidate,
    declared_promotion_evidence_gate,
    promotion_decision_never_deploys,
    promotion_gate_decision,
    sha256_file,
    shadow_gate_verdict,
)

REPO_ROOT = Path(__file__).resolve().parents[2]
CANDIDATE_JSON = (
    REPO_ROOT / "microbrain" / "training" / "plans" / "nexus-candidate-v1.candidate.json"
)
ARTIFACT_JSON = (
    REPO_ROOT / "microbrain" / "artifacts" / "fixtures" / "nexus-artifact-v1.artifact.json"
)
MANIFEST = (
    REPO_ROOT
    / "microbrain"
    / "datasets"
    / "manifests"
    / "nexus-synthetic-role-ops-v1.manifest.json"
)


def _candidate() -> TrainingCandidate:
    return TrainingCandidate.from_dict(json.loads(CANDIDATE_JSON.read_text(encoding="utf-8")))


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
        run_id="shadow-aud064-1",
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


def _evidence(
    *,
    qlora_status: str = QloraStatus.COMPLETED.value,
    candidate_id: str = "nexus-candidate-v1",
    artifact_digest: str | None = None,
    shadow_run_id: str = "shadow-aud064-1",
    false_positive_count: int = 0,
    certification_boundary: str = "aud064 declared evidence; real training certified separately",
    dataset_digest: str | None = None,
    eval_suite_digest: str | None = None,
) -> PromotionEvidence:
    artifact = json.loads(ARTIFACT_JSON.read_text(encoding="utf-8"))
    return PromotionEvidence(
        run_id="run-aud064-1",
        git_commit="aud064-git",
        candidate_id=candidate_id,
        dataset_id="nexus-synthetic-role-ops-v1",
        dataset_digest=dataset_digest or sha256_file(MANIFEST),
        eval_suite_id="nexus-frozen-suite-v1",
        eval_suite_digest=eval_suite_digest or "sha256:" + "c" * 64,
        plan_digest="sha256:" + "d" * 64,
        qlora_run_id="run-aud064-1",
        qlora_status=qlora_status,
        artifact_id=artifact["artifact_id"],
        artifact_digest=artifact_digest or artifact["digest"],
        quantization_format=artifact["format"],
        shadow_run_id=shadow_run_id,
        shadow_decision="LOW_RISK_CANARY",
        false_positive_count=false_positive_count,
        promotion_decision="",
        promotion_gate="",
        certification_boundary=certification_boundary,
    )


# ---------------------------------------------------------------------------
# AUD-064 hostile proofs: promotion fails closed without declared evidence
# ---------------------------------------------------------------------------


def aud064_promote_without_declared_evidence_denies() -> None:
    decision = promotion_gate_decision(
        decision_id="aud064-noevidence",
        candidate=_candidate(),
        prerequisites=_real_prerequisites(),
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
        evidence=None,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "declared promotion/evaluation evidence required" in decision.reason


def aud064_promote_with_declared_only_run_denies() -> None:
    """The exact M5 defect: PENDING (declared-only) run must never PROMOTE."""
    from nexus_microbrain import PromotionDecision

    candidate = _candidate()
    shadow = _shadow_pass()
    evidence = _evidence(qlora_status=QloraStatus.PENDING.value)
    decision = promotion_gate_decision(
        decision_id="aud064-pending",
        candidate=candidate,
        prerequisites=_real_prerequisites(),
        shadow=shadow,
        eval_ref="nexus-frozen-suite-v1",
        evidence=evidence,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "training run not COMPLETED" in decision.reason
    # The standalone gate also reports unsupported for a prospective
    # PROMOTE bound to declared-only (PENDING) evidence.
    prospective = PromotionDecision(
        decision_id="aud064-pending-prospective",
        verdict=PromotionVerdict.PROMOTE,
        gate=PromotionGate.GRADUAL,
        candidate_ref=candidate.candidate_id,
        eval_ref="nexus-frozen-suite-v1",
        shadow_ref=shadow.run_id,
        zero_consequential_false_positives=True,
        reason="prospective",
    )
    verdict = declared_promotion_evidence_gate(
        decision=prospective,
        evidence=evidence,
        candidate=candidate,
        shadow=shadow,
    )
    assert not verdict.supported
    assert any("training run not COMPLETED" in r for r in verdict.reasons)


def aud064_promote_with_malformed_artifact_digest_denies() -> None:
    evidence = _evidence(artifact_digest="not-a-digest")
    decision = promotion_gate_decision(
        decision_id="aud064-baddigest",
        candidate=_candidate(),
        prerequisites=_real_prerequisites(),
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
        evidence=evidence,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "artifact digest malformed" in decision.reason


def aud064_promote_with_candidate_mismatch_denies() -> None:
    evidence = _evidence(candidate_id="some-other-candidate")
    decision = promotion_gate_decision(
        decision_id="aud064-badcand",
        candidate=_candidate(),
        prerequisites=_real_prerequisites(),
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
        evidence=evidence,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "candidate" in decision.reason


def aud064_promote_with_shadow_mismatch_denies() -> None:
    evidence = _evidence(shadow_run_id="shadow-other")
    decision = promotion_gate_decision(
        decision_id="aud064-badshadow",
        candidate=_candidate(),
        prerequisites=_real_prerequisites(),
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
        evidence=evidence,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "shadow" in decision.reason


def aud064_promote_with_false_positives_denies() -> None:
    evidence = _evidence(false_positive_count=1)
    decision = promotion_gate_decision(
        decision_id="aud064-fp",
        candidate=_candidate(),
        prerequisites=_real_prerequisites(),
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
        evidence=evidence,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "false positives" in decision.reason


def aud064_promote_with_empty_certification_boundary_denies() -> None:
    evidence = _evidence(certification_boundary="   ")
    decision = promotion_gate_decision(
        decision_id="aud064-noboundary",
        candidate=_candidate(),
        prerequisites=_real_prerequisites(),
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
        evidence=evidence,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "certification boundary" in decision.reason


def aud064_declared_evidence_missing_dataset_digest_denies() -> None:
    evidence = _evidence(dataset_digest="short")
    decision = promotion_gate_decision(
        decision_id="aud064-baddataset",
        candidate=_candidate(),
        prerequisites=_real_prerequisites(),
        shadow=_shadow_pass(),
        eval_ref="nexus-frozen-suite-v1",
        evidence=evidence,
    )
    assert decision.verdict is PromotionVerdict.DENY
    assert "dataset binding" in decision.reason


# ---------------------------------------------------------------------------
# AUD-064 positive proofs
# ---------------------------------------------------------------------------


def aud064_complete_declared_evidence_promotes() -> None:
    candidate = _candidate()
    shadow = _shadow_pass()
    evidence = _evidence()
    decision = promotion_gate_decision(
        decision_id="aud064-ok",
        candidate=candidate,
        prerequisites=_real_prerequisites(),
        shadow=shadow,
        eval_ref="nexus-frozen-suite-v1",
        evidence=evidence,
    )
    assert decision.verdict is PromotionVerdict.PROMOTE
    assert decision.gate is PromotionGate.GRADUAL
    # A decision is still never autonomous deployment.
    assert promotion_decision_never_deploys(decision)
    verdict = declared_promotion_evidence_gate(
        decision=decision,
        evidence=evidence,
        candidate=candidate,
        shadow=shadow,
    )
    assert verdict.supported
    assert verdict.reasons == ()


def aud064_declared_evidence_gate_redacted_no_secret() -> None:
    candidate = _candidate()
    shadow = _shadow_pass()
    decision = promotion_gate_decision(
        decision_id="aud064-redact",
        candidate=candidate,
        prerequisites=_real_prerequisites(),
        shadow=shadow,
        eval_ref="nexus-frozen-suite-v1",
        evidence=_evidence(),
    )
    verdict = declared_promotion_evidence_gate(
        decision=decision,
        evidence=_evidence(),
        candidate=candidate,
        shadow=shadow,
    )
    payload = json.dumps(verdict.to_redacted_dict())
    assert "ghp_" not in payload


def aud064_non_promote_decision_needs_no_evidence() -> None:
    candidate = _candidate()
    shadow = _shadow_pass()
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
        decision_id="aud064-denyonly",
        candidate=candidate,
        prerequisites=prereqs,
        shadow=shadow,
        eval_ref="nexus-frozen-suite-v1",
        evidence=None,
    )
    assert decision.verdict is PromotionVerdict.DENY
    verdict = declared_promotion_evidence_gate(
        decision=decision,
        evidence=None,
        candidate=candidate,
        shadow=shadow,
    )
    assert verdict.supported
