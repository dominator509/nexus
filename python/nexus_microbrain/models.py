"""Microbrain public contracts (EP-041 node contract).

Every interface is provider-neutral, versioned, and fail-closed:
- schema_version must be the supported version or construction fails
- required fields are enforced with typed codes
- vocabulary values are parsed through deny-unknown enums
- cross-field invariants encode the SPEC-025 acceptance obligations
  (frozen split, teacher licensing/privacy, narrow role, zero
  consequential false positives) at the contract boundary.

The models are stdlib-only dataclasses; to_dict/from_dict provide the
versioned serialization contract. Provider adapters may add internal
types but cannot alter these canonical shapes.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from collections.abc import Mapping
from dataclasses import dataclass, field
from typing import Any, ClassVar, Self

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

SUPPORTED_SCHEMA_VERSION = "1"


def _require(data: Mapping[str, Any], key: str) -> Any:
    if key not in data or data[key] is None:
        raise MicrobrainError(
            MICROBRAIN_CODE_MISSING_REQUIRED,
            f"missing required field: {key}",
        )
    return data[key]


def _parse_enum(
    enum_cls: type[Any],
    data: Mapping[str, Any],
    key: str,
) -> Any:
    return enum_cls.parse(_require(data, key))


@dataclass(frozen=True, slots=True)
class MicrobrainModel(ABC):
    """Base contract: versioned serialization with fail-closed parsing.

    Abstract by design: the base cannot be instantiated and every
    concrete contract must implement to_dict.
    """

    schema_version: ClassVar[str] = SUPPORTED_SCHEMA_VERSION

    def __post_init__(self) -> None:
        self._validate()

    @abstractmethod
    def to_dict(self) -> dict[str, Any]:
        """Serialize to the canonical versioned dictionary shape."""

    @classmethod
    def _check_version(cls, data: Mapping[str, Any]) -> None:
        version = data.get("schema_version", SUPPORTED_SCHEMA_VERSION)
        if version != cls.schema_version:
            raise MicrobrainError(
                MICROBRAIN_CODE_UNSUPPORTED_VERSION,
                f"unsupported schema_version: {version!r}",
            )

    def _validate(self) -> None:
        return None


@dataclass(frozen=True, slots=True)
class LicenseRecord(MicrobrainModel):
    """Separately recorded license class (SPEC-025 behavior 8)."""

    license_ref: str
    kind: LicenseKind
    status: str = "RECORDED"

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "license_ref": self.license_ref,
            "kind": self.kind.value,
            "status": self.status,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            license_ref=str(_require(data, "license_ref")),
            kind=LicenseKind.parse(_require(data, "kind")),
            status=str(data.get("status", "RECORDED")),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.license_ref.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_UNLICENSED,
                "license_ref must not be empty",
            )
        if not self.status.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "status must not be empty",
            )


@dataclass(frozen=True, slots=True)
class TrainingExample(MicrobrainModel):
    """One dataset item (SPEC-025 locked term TrainingExample)."""

    example_id: str
    role: Role
    input_text: str
    control_object: dict[str, Any]
    provenance: DataProvenance
    hard_negative: bool
    ood_verdict: OodVerdict
    license_ref: str | None = None
    correlation_id: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "example_id": self.example_id,
            "role": self.role.value,
            "input_text": self.input_text,
            "control_object": self.control_object,
            "provenance": self.provenance.value,
            "hard_negative": self.hard_negative,
            "ood_verdict": self.ood_verdict.value,
            "license_ref": self.license_ref,
            "correlation_id": self.correlation_id,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            example_id=str(_require(data, "example_id")),
            role=Role.parse(_require(data, "role")),
            input_text=str(_require(data, "input_text")),
            control_object=dict(_require(data, "control_object")),
            provenance=DataProvenance.parse(_require(data, "provenance")),
            hard_negative=bool(_require(data, "hard_negative")),
            ood_verdict=OodVerdict.parse(_require(data, "ood_verdict")),
            license_ref=data.get("license_ref"),
            correlation_id=data.get("correlation_id"),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.example_id.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "example_id must not be empty",
            )
        if not self.input_text.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "input_text must not be empty",
            )
        if self.provenance is DataProvenance.HARD_NEGATIVE and not self.hard_negative:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "HARD_NEGATIVE provenance requires hard_negative=true",
            )
        if (
            self.provenance
            in (DataProvenance.TEACHER_CONSENSUS, DataProvenance.OPTED_IN_SCRUBBED_CORRECTION)
            and self.license_ref is None
        ):
            raise MicrobrainError(
                MICROBRAIN_CODE_UNLICENSED,
                "teacher/opted-in examples require a license_ref",
            )


@dataclass(frozen=True, slots=True)
class MicrobrainDataset(MicrobrainModel):
    """Microbrain dataset contract (SPEC-025).

    dataset_id, name, and lineage are required; examples are validated
    individually and may be empty only when the dataset is still being
    assembled. Lineage is the contract-level traceability hook required
    by the dataset-lineage test.
    """

    dataset_id: str
    name: str
    lineage: str
    examples: tuple[TrainingExample, ...] = field(default_factory=tuple)
    created_at: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "dataset_id": self.dataset_id,
            "name": self.name,
            "lineage": self.lineage,
            "examples": [example.to_dict() for example in self.examples],
            "created_at": self.created_at,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        examples = tuple(TrainingExample.from_dict(item) for item in _require(data, "examples"))
        obj = cls(
            dataset_id=str(_require(data, "dataset_id")),
            name=str(_require(data, "name")),
            lineage=str(_require(data, "lineage")),
            examples=examples,
            created_at=data.get("created_at"),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.dataset_id.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "dataset_id must not be empty",
            )
        if not self.name.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "name must not be empty",
            )
        if not self.lineage.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "lineage must not be empty",
            )


@dataclass(frozen=True, slots=True)
class FrozenEval(MicrobrainModel):
    """One frozen evaluation item (SPEC-025 locked term FrozenEval).

    The frozen hidden test set is created before training and never used
    for gradient updates or prompt iteration; the contract encodes that
    obligation with created_before_training and a frozen marker.
    """

    eval_id: str
    kind: str
    example: TrainingExample
    dimensions: tuple[EvalDimension, ...]
    created_before_training: bool
    frozen_at: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "eval_id": self.eval_id,
            "kind": self.kind,
            "example": self.example.to_dict(),
            "dimensions": [dimension.value for dimension in self.dimensions],
            "created_before_training": self.created_before_training,
            "frozen_at": self.frozen_at,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            eval_id=str(_require(data, "eval_id")),
            kind=str(_require(data, "kind")),
            example=TrainingExample.from_dict(_require(data, "example")),
            dimensions=tuple(EvalDimension.parse(item) for item in _require(data, "dimensions")),
            created_before_training=bool(_require(data, "created_before_training")),
            frozen_at=data.get("frozen_at"),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.eval_id.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "eval_id must not be empty",
            )
        if self.kind != "FROZEN":
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                f"kind must be FROZEN, got {self.kind!r}",
            )
        if not self.created_before_training:
            raise MicrobrainError(
                MICROBRAIN_CODE_FROZEN_SPLIT_VIOLATION,
                "frozen eval must predate training (created_before_training=true)",
            )
        if not self.dimensions:
            raise MicrobrainError(
                MICROBRAIN_CODE_MISSING_REQUIRED,
                "frozen eval requires at least one dimension",
            )


@dataclass(frozen=True, slots=True)
class FrozenEvalSuite(MicrobrainModel):
    """Frozen hidden test suite (SPEC-025 behavior 3)."""

    suite_id: str
    evals: tuple[FrozenEval, ...] = field(default_factory=tuple)
    created_at: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "suite_id": self.suite_id,
            "evals": [evaluation.to_dict() for evaluation in self.evals],
            "created_at": self.created_at,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            suite_id=str(_require(data, "suite_id")),
            evals=tuple(FrozenEval.from_dict(item) for item in _require(data, "evals")),
            created_at=data.get("created_at"),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.suite_id.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "suite_id must not be empty",
            )
        if not self.evals:
            raise MicrobrainError(
                MICROBRAIN_CODE_MISSING_REQUIRED,
                "frozen eval suite must not be empty",
            )
        for evaluation in self.evals:
            if not evaluation.created_before_training:
                raise MicrobrainError(
                    MICROBRAIN_CODE_FROZEN_SPLIT_VIOLATION,
                    f"eval {evaluation.eval_id} does not predate training",
                )


@dataclass(frozen=True, slots=True)
class TeacherConsensus(MicrobrainModel):
    """Frontier teacher consensus record (SPEC-025 locked term).

    Acceptance obligation: teacher data is filtered, licensed, and
    privacy safe. The contract refuses a consensus record that is not
    filtered, not privacy safe, or carries no license records.
    """

    consensus_id: str
    teachers: tuple[str, ...]
    consensus_text: str
    agreement_ratio: float
    filtered: bool
    privacy_safe: bool
    licenses: tuple[LicenseRecord, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "consensus_id": self.consensus_id,
            "teachers": list(self.teachers),
            "consensus_text": self.consensus_text,
            "agreement_ratio": self.agreement_ratio,
            "filtered": self.filtered,
            "privacy_safe": self.privacy_safe,
            "licenses": [license.to_dict() for license in self.licenses],
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            consensus_id=str(_require(data, "consensus_id")),
            teachers=tuple(str(item) for item in _require(data, "teachers")),
            consensus_text=str(_require(data, "consensus_text")),
            agreement_ratio=float(_require(data, "agreement_ratio")),
            filtered=bool(_require(data, "filtered")),
            privacy_safe=bool(_require(data, "privacy_safe")),
            licenses=tuple(LicenseRecord.from_dict(item) for item in _require(data, "licenses")),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.consensus_id.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "consensus_id must not be empty",
            )
        if not self.teachers:
            raise MicrobrainError(
                MICROBRAIN_CODE_MISSING_REQUIRED,
                "teacher consensus requires at least one teacher",
            )
        if not self.consensus_text.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "consensus_text must not be empty",
            )
        if not 0.0 <= self.agreement_ratio <= 1.0:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "agreement_ratio must be within [0, 1]",
            )
        if not self.filtered:
            raise MicrobrainError(
                MICROBRAIN_CODE_PRIVACY_VIOLATION,
                "teacher data must be filtered",
            )
        if not self.privacy_safe:
            raise MicrobrainError(
                MICROBRAIN_CODE_PRIVACY_VIOLATION,
                "teacher data must be privacy safe",
            )
        if not self.licenses:
            raise MicrobrainError(
                MICROBRAIN_CODE_UNLICENSED,
                "teacher data requires recorded licenses",
            )


@dataclass(frozen=True, slots=True)
class TrainingCandidate(MicrobrainModel):
    """A candidate model bound to one narrow NexusControlObject role.

    Acceptance obligation: a candidate cannot exceed its narrow role.
    The contract enforces the role is exactly one canonical Role value.
    """

    candidate_id: str
    role: Role
    model_ref: str
    base_model: str
    dataset_ref: str
    status: CandidateStatus = CandidateStatus.CANDIDATE

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "candidate_id": self.candidate_id,
            "role": self.role.value,
            "model_ref": self.model_ref,
            "base_model": self.base_model,
            "dataset_ref": self.dataset_ref,
            "status": self.status.value,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            candidate_id=str(_require(data, "candidate_id")),
            role=Role.parse(_require(data, "role")),
            model_ref=str(_require(data, "model_ref")),
            base_model=str(_require(data, "base_model")),
            dataset_ref=str(_require(data, "dataset_ref")),
            status=CandidateStatus.parse(data.get("status", CandidateStatus.CANDIDATE.value)),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.candidate_id.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "candidate_id must not be empty",
            )
        if self.role is None:
            raise MicrobrainError(
                MICROBRAIN_CODE_ROLE_EXCEEDED,
                "candidate requires a narrow canonical role",
            )
        if not self.model_ref.strip() or not self.base_model.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "model_ref and base_model must not be empty",
            )


@dataclass(frozen=True, slots=True)
class QloraRun(MicrobrainModel):
    """QLoRA training run record (SPEC-025 locked term QLoRA).

    Reproducibility is contract-level: seed and config_digest are
    required fields so a run can be reproduced deterministically.
    """

    run_id: str
    candidate_ref: str
    adapter: str
    rank: int
    alpha: int
    seed: int
    config_digest: str
    dataset_ref: str
    status: QloraStatus = QloraStatus.PENDING
    correlation_id: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "run_id": self.run_id,
            "candidate_ref": self.candidate_ref,
            "adapter": self.adapter,
            "rank": self.rank,
            "alpha": self.alpha,
            "seed": self.seed,
            "config_digest": self.config_digest,
            "dataset_ref": self.dataset_ref,
            "status": self.status.value,
            "correlation_id": self.correlation_id,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            run_id=str(_require(data, "run_id")),
            candidate_ref=str(_require(data, "candidate_ref")),
            adapter=str(_require(data, "adapter")),
            rank=int(_require(data, "rank")),
            alpha=int(_require(data, "alpha")),
            seed=int(_require(data, "seed")),
            config_digest=str(_require(data, "config_digest")),
            dataset_ref=str(_require(data, "dataset_ref")),
            status=QloraStatus.parse(data.get("status", QloraStatus.PENDING.value)),
            correlation_id=data.get("correlation_id"),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.run_id.strip() or not self.candidate_ref.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "run_id and candidate_ref must not be empty",
            )
        if self.rank <= 0 or self.alpha <= 0:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "rank and alpha must be positive",
            )
        if not self.config_digest.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "config_digest must not be empty",
            )


@dataclass(frozen=True, slots=True)
class QuantizedArtifact(MicrobrainModel):
    """GGUF quantized artifact (SPEC-025 locked term GGUF).

    Identity is the digest (alg:hex, >=32 hex chars) - a name or tag is
    never the artifact identity (image-tag-vs-digest lesson carried from
    EP-039).
    """

    artifact_id: str
    candidate_ref: str
    format: QuantizationFormat
    quantization: str
    digest: str
    size_bytes: int
    license_ref: str | None = None
    status: ArtifactStatus = ArtifactStatus.BUILT

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "artifact_id": self.artifact_id,
            "candidate_ref": self.candidate_ref,
            "format": self.format.value,
            "quantization": self.quantization,
            "digest": self.digest,
            "size_bytes": self.size_bytes,
            "license_ref": self.license_ref,
            "status": self.status.value,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            artifact_id=str(_require(data, "artifact_id")),
            candidate_ref=str(_require(data, "candidate_ref")),
            format=QuantizationFormat.parse(_require(data, "format")),
            quantization=str(_require(data, "quantization")),
            digest=str(_require(data, "digest")),
            size_bytes=int(_require(data, "size_bytes")),
            license_ref=data.get("license_ref"),
            status=ArtifactStatus.parse(data.get("status", ArtifactStatus.BUILT.value)),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.artifact_id.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "artifact_id must not be empty",
            )
        if self.format is not QuantizationFormat.GGUF:
            raise MicrobrainError(
                MICROBRAIN_CODE_UNKNOWN_VOCABULARY,
                f"unsupported artifact format: {self.format.value!r}",
            )
        if ":" not in self.digest:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "digest must be alg:hex",
            )
        alg, _, hex_part = self.digest.partition(":")
        if not alg or len(hex_part) < 32:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "digest hex must be at least 32 chars",
            )
        if self.size_bytes < 0:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "size_bytes must not be negative",
            )


@dataclass(frozen=True, slots=True)
class ShadowComparison(MicrobrainModel):
    """One shadow comparison against the ReflexProvider (DeepSeek)."""

    input_ref: str
    candidate_decision: str
    provider_decision: str
    decision: ShadowDecision
    ood_verdict: OodVerdict = OodVerdict.IN_DISTRIBUTION

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "input_ref": self.input_ref,
            "candidate_decision": self.candidate_decision,
            "provider_decision": self.provider_decision,
            "decision": self.decision.value,
            "ood_verdict": self.ood_verdict.value,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            input_ref=str(_require(data, "input_ref")),
            candidate_decision=str(_require(data, "candidate_decision")),
            provider_decision=str(_require(data, "provider_decision")),
            decision=ShadowDecision.parse(_require(data, "decision")),
            ood_verdict=OodVerdict.parse(data.get("ood_verdict", OodVerdict.IN_DISTRIBUTION.value)),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.input_ref.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "input_ref must not be empty",
            )


@dataclass(frozen=True, slots=True)
class ShadowComparator(MicrobrainModel):
    """Shadow comparison run against the ReflexProvider.

    Acceptance obligation: shadow and canary thresholds include zero
    consequential false positives. The contract computes the zero flag
    from the recorded count and refuses a negative count.
    """

    run_id: str
    candidate_ref: str
    provider_ref: str
    comparisons: tuple[ShadowComparison, ...] = field(default_factory=tuple)
    exact_match_rate: float = 1.0
    consequential_false_positives: int = 0
    correlation_id: str | None = None

    def zero_consequential_false_positives(self) -> bool:
        return self.consequential_false_positives == 0

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "run_id": self.run_id,
            "candidate_ref": self.candidate_ref,
            "provider_ref": self.provider_ref,
            "comparisons": [comparison.to_dict() for comparison in self.comparisons],
            "exact_match_rate": self.exact_match_rate,
            "consequential_false_positives": self.consequential_false_positives,
            "correlation_id": self.correlation_id,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            run_id=str(_require(data, "run_id")),
            candidate_ref=str(_require(data, "candidate_ref")),
            provider_ref=str(_require(data, "provider_ref")),
            comparisons=tuple(
                ShadowComparison.from_dict(item) for item in _require(data, "comparisons")
            ),
            exact_match_rate=float(_require(data, "exact_match_rate")),
            consequential_false_positives=int(_require(data, "consequential_false_positives")),
            correlation_id=data.get("correlation_id"),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.run_id.strip() or not self.candidate_ref.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "run_id and candidate_ref must not be empty",
            )
        if not self.provider_ref.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "provider_ref must not be empty",
            )
        if not 0.0 <= self.exact_match_rate <= 1.0:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "exact_match_rate must be within [0, 1]",
            )
        if self.consequential_false_positives < 0:
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "consequential_false_positives must not be negative",
            )


@dataclass(frozen=True, slots=True)
class PromotionDecision(MicrobrainModel):
    """Promotion gate decision (SPEC-025 locked term PromotionGate).

    A consequential false positive in the protected test class is a hard
    promotion failure (SPEC-025 behavior 6): the contract refuses a
    PROMOTE verdict unless zero consequential false positives is proven.
    """

    decision_id: str
    verdict: PromotionVerdict
    gate: PromotionGate
    candidate_ref: str
    eval_ref: str
    shadow_ref: str
    zero_consequential_false_positives: bool
    reason: str = ""

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "decision_id": self.decision_id,
            "verdict": self.verdict.value,
            "gate": self.gate.value,
            "candidate_ref": self.candidate_ref,
            "eval_ref": self.eval_ref,
            "shadow_ref": self.shadow_ref,
            "zero_consequential_false_positives": self.zero_consequential_false_positives,
            "reason": self.reason,
        }

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> Self:
        cls._check_version(data)
        obj = cls(
            decision_id=str(_require(data, "decision_id")),
            verdict=PromotionVerdict.parse(_require(data, "verdict")),
            gate=PromotionGate.parse(_require(data, "gate")),
            candidate_ref=str(_require(data, "candidate_ref")),
            eval_ref=str(_require(data, "eval_ref")),
            shadow_ref=str(_require(data, "shadow_ref")),
            zero_consequential_false_positives=bool(
                _require(data, "zero_consequential_false_positives")
            ),
            reason=str(data.get("reason", "")),
        )
        obj._validate()
        return obj

    def _validate(self) -> None:
        if not self.decision_id.strip():
            raise MicrobrainError(
                MICROBRAIN_CODE_INVALID_INPUT,
                "decision_id must not be empty",
            )
        if self.verdict is PromotionVerdict.PROMOTE:
            if not self.zero_consequential_false_positives:
                raise MicrobrainError(
                    MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD,
                    "cannot promote with consequential false positives",
                )
            if self.gate is PromotionGate.SHADOW:
                raise MicrobrainError(
                    MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD,
                    "cannot promote directly from shadow",
                )
