"""Canonical Microbrain vocabulary (SPEC-025 + SPEC-009 locked terms).

Every enum is deny-unknown: parsing an unrecognized value raises
MicrobrainError with MICROBRAIN_CODE_UNKNOWN_VOCABULARY instead of
silently accepting it. A new synonym requires an ADR and schema update.
"""

from __future__ import annotations

import enum
from typing import Any, TypeVar

from .errors import (
    MICROBRAIN_CODE_UNKNOWN_VOCABULARY,
    MicrobrainError,
)

_EnumT = TypeVar("_EnumT", bound="MicrobrainEnum")


class MicrobrainEnum(enum.StrEnum):
    """Base class: canonical string values, fail-closed parsing."""

    @classmethod
    def parse(cls: type[_EnumT], raw: Any) -> _EnumT:
        if isinstance(raw, cls):
            return raw
        if not isinstance(raw, str):
            raise MicrobrainError(
                MICROBRAIN_CODE_UNKNOWN_VOCABULARY,
                f"{cls.__name__} value must be a string, got {type(raw).__name__}",
            )
        try:
            return cls(raw)
        except ValueError as exc:
            raise MicrobrainError(
                MICROBRAIN_CODE_UNKNOWN_VOCABULARY,
                f"unknown {cls.__name__} value: {raw!r}",
            ) from exc

    @classmethod
    def canonical_values(cls) -> list[str]:
        return [member.value for member in cls]


class Role(MicrobrainEnum):
    """Narrow NexusControlObject interpretation roles (SPEC-025 behavior 2).

    A candidate is bound to exactly one of these roles and cannot exceed it.
    """

    INTERPRETATION = "INTERPRETATION"
    CAPABILITY_SELECTION = "CAPABILITY_SELECTION"
    ROUTING = "ROUTING"
    RISK = "RISK"
    PRIVACY = "PRIVACY"
    AMBIGUITY = "AMBIGUITY"
    QUOTED_INSTRUCTION = "QUOTED_INSTRUCTION"
    ESCALATION = "ESCALATION"


class DataProvenance(MicrobrainEnum):
    """Training data provenance classes (SPEC-025 behavior 4)."""

    DETERMINISTIC_GENERATION = "DETERMINISTIC_GENERATION"
    TEACHER_CONSENSUS = "TEACHER_CONSENSUS"
    HARD_NEGATIVE = "HARD_NEGATIVE"
    OPTED_IN_SCRUBBED_CORRECTION = "OPTED_IN_SCRUBBED_CORRECTION"


class EvalDimension(MicrobrainEnum):
    """Evaluation dimensions (SPEC-025 behavior 5)."""

    EXACT_SCHEMA = "EXACT_SCHEMA"
    INTENT = "INTENT"
    ARGUMENTS = "ARGUMENTS"
    ROUTING = "ROUTING"
    RISK = "RISK"
    APPROVAL = "APPROVAL"
    INJECTION_RESISTANCE = "INJECTION_RESISTANCE"
    OUT_OF_DISTRIBUTION_ESCALATION = "OUT_OF_DISTRIBUTION_ESCALATION"
    LATENCY = "LATENCY"
    MEMORY = "MEMORY"
    QUANTIZATION_REGRESSION = "QUANTIZATION_REGRESSION"


class QuantizationFormat(MicrobrainEnum):
    """Quantized artifact formats (SPEC-025 locked term GGUF)."""

    GGUF = "GGUF"


class ShadowDecision(MicrobrainEnum):
    """Shadow comparison verdict per input (SPEC-009 shadow)."""

    MATCH = "MATCH"
    DIFFER = "DIFFER"
    DEFER = "DEFER"


class PromotionGate(MicrobrainEnum):
    """Promotion stage ladder (SPEC-025 behavior 7)."""

    SHADOW = "SHADOW"
    LOW_RISK_CANARY = "LOW_RISK_CANARY"
    GRADUAL = "GRADUAL"
    PROMOTED = "PROMOTED"


class PromotionVerdict(MicrobrainEnum):
    """Final promotion gate decision."""

    PROMOTE = "PROMOTE"
    DENY = "DENY"
    HOLD = "HOLD"


class OodVerdict(MicrobrainEnum):
    """Out-of-distribution verdict (SPEC-025 locked term OutOfDistribution)."""

    IN_DISTRIBUTION = "IN_DISTRIBUTION"
    OUT_OF_DISTRIBUTION = "OUT_OF_DISTRIBUTION"


class LicenseKind(MicrobrainEnum):
    """Separately recorded license classes (SPEC-025 behavior 8)."""

    MODEL = "MODEL"
    ADAPTER = "ADAPTER"
    DATASET = "DATASET"
    CODE = "CODE"
    EVALUATION = "EVALUATION"
    VOICE_OR_LANGUAGE = "VOICE_OR_LANGUAGE"


class CandidateStatus(MicrobrainEnum):
    """Training candidate lifecycle ladder."""

    CANDIDATE = "CANDIDATE"
    TRAINED = "TRAINED"
    EVALUATED = "EVALUATED"
    SHADOW_READY = "SHADOW_READY"
    CANARY_READY = "CANARY_READY"
    PROMOTED = "PROMOTED"
    REJECTED = "REJECTED"


class QloraStatus(MicrobrainEnum):
    """QLoRA run lifecycle."""

    PENDING = "PENDING"
    RUNNING = "RUNNING"
    COMPLETED = "COMPLETED"
    FAILED = "FAILED"


class ArtifactStatus(MicrobrainEnum):
    """Quantized artifact lifecycle."""

    BUILT = "BUILT"
    VERIFIED = "VERIFIED"
    RELEASED = "RELEASED"
    RETIRED = "RETIRED"
