"""Microbrain contract package.

Provider-neutral contracts for the separate Microbrain training factory
(SPEC-025, SPEC-009): dataset, frozen evals, teacher consensus, QLoRA
pipeline, GGUF export, shadow comparison, and promotion.

This package is the M1 contract boundary for EP-041. It is stdlib-only
(no provider SDK, no ML framework, no network client) so the vocabulary
and models stay deterministic and transport-neutral. Behavior lives in
later milestones; this crate locks construction, validation, versioned
serialization, and vocabulary rejection.
"""

from __future__ import annotations

from .dataset_policy import (
    DEFAULT_PROHIBITED_LICENSE_REFS,
    DatasetPolicy,
    DatasetVerdict,
    ManifestVerification,
    load_manifest,
    sha256_manifest,
    verify_manifest_file,
)
from .errors import (
    MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD,
    MICROBRAIN_CODE_FROZEN_SPLIT_VIOLATION,
    MICROBRAIN_CODE_INVALID_INPUT,
    MICROBRAIN_CODE_MISSING_REQUIRED,
    MICROBRAIN_CODE_PRIVACY_VIOLATION,
    MICROBRAIN_CODE_ROLE_EXCEEDED,
    MICROBRAIN_CODE_UNKNOWN_VOCABULARY,
    MICROBRAIN_CODE_UNLICENSED,
    MICROBRAIN_CODE_UNSUPPORTED_VERSION,
    MicrobrainError,
    redact_text,
)
from .eval_policy import (
    DimensionResult,
    EvalBindingVerdict,
    EvalEvidence,
    EvalScoreSummary,
    EvalSuiteVerdict,
    SuiteBinding,
    SuiteScoreSummary,
    bind_suite_to_dataset,
    build_eval_evidence,
    check_eval_before_training,
    load_suite_binding,
    score_eval,
    score_suite,
    suite_digest,
    validate_suite,
    verify_suite_digest,
)
from .models import (
    FrozenEval,
    FrozenEvalSuite,
    LicenseRecord,
    MicrobrainDataset,
    PromotionDecision,
    QloraRun,
    QuantizedArtifact,
    ShadowComparator,
    ShadowComparison,
    TeacherConsensus,
    TrainingCandidate,
    TrainingExample,
)
from .vocabulary import (
    ArtifactStatus,
    CandidateStatus,
    DataProvenance,
    EvalDimension,
    LicenseKind,
    OodVerdict,
    PromotionGate,
    PromotionVerdict,
    QloraStatus,
    QuantizationFormat,
    Role,
    ShadowDecision,
)

__all__ = [
    "ArtifactStatus",
    "CandidateStatus",
    "DEFAULT_PROHIBITED_LICENSE_REFS",
    "DataProvenance",
    "DatasetPolicy",
    "DatasetVerdict",
    "DimensionResult",
    "EvalBindingVerdict",
    "EvalEvidence",
    "EvalScoreSummary",
    "EvalSuiteVerdict",
    "SuiteBinding",
    "SuiteScoreSummary",
    "bind_suite_to_dataset",
    "build_eval_evidence",
    "check_eval_before_training",
    "load_suite_binding",
    "score_eval",
    "score_suite",
    "suite_digest",
    "validate_suite",
    "verify_suite_digest",
    "EvalDimension",
    "FrozenEval",
    "FrozenEvalSuite",
    "LicenseKind",
    "LicenseRecord",
    "MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD",
    "MICROBRAIN_CODE_FROZEN_SPLIT_VIOLATION",
    "MICROBRAIN_CODE_INVALID_INPUT",
    "MICROBRAIN_CODE_MISSING_REQUIRED",
    "MICROBRAIN_CODE_PRIVACY_VIOLATION",
    "MICROBRAIN_CODE_ROLE_EXCEEDED",
    "MICROBRAIN_CODE_UNKNOWN_VOCABULARY",
    "MICROBRAIN_CODE_UNLICENSED",
    "MICROBRAIN_CODE_UNSUPPORTED_VERSION",
    "ManifestVerification",
    "MicrobrainDataset",
    "MicrobrainError",
    "OodVerdict",
    "PromotionDecision",
    "PromotionGate",
    "PromotionVerdict",
    "QloraRun",
    "QloraStatus",
    "QuantizationFormat",
    "QuantizedArtifact",
    "Role",
    "ShadowComparison",
    "ShadowComparator",
    "ShadowDecision",
    "TeacherConsensus",
    "TrainingCandidate",
    "TrainingExample",
    "load_manifest",
    "redact_text",
    "sha256_manifest",
    "verify_manifest_file",
]
