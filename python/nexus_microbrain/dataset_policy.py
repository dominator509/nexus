"""Deterministic Microbrain dataset policy (EP-041 M2, SPEC-025).

Dataset policy sits ABOVE the M1 contract: M1 proves a dataset can be
constructed and serialized; this module decides whether a dataset is
usable training data. Every rule fails closed with typed SPEC-006-style
codes. The policy is pure (no I/O); manifest loading and digest
verification are thin boundary helpers that return M1 models or typed
errors.

Locked invariants (M2):
- DATASET EXISTS != DATASET USABLE
- DATASET LICENSED != DATASET PRIVACY SAFE
- MISSING LICENSE -> DENIED
- PROHIBITED LICENSE -> DENIED
- UNKNOWN LICENSE -> DENIED
- UNFILTERED/HARD-NEGATIVE INCONSISTENCY -> DENIED
"""

from __future__ import annotations

import hashlib
import json
from collections import Counter
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .errors import (
    MICROBRAIN_CODE_INVALID_INPUT,
    MICROBRAIN_CODE_MISSING_REQUIRED,
    MicrobrainError,
    redact_value,
)
from .models import MicrobrainDataset
from .vocabulary import (
    DataProvenance,
    OodVerdict,
    Role,
)

# Canonical prohibited license references for training data. CC-BY-NC is
# commercial-restricted and never usable for the Microbrain artifact;
# the set is a policy input, not a contract vocabulary change.
DEFAULT_PROHIBITED_LICENSE_REFS: frozenset[str] = frozenset({"cc-by-nc-4.0"})
UNKNOWN_LICENSE_PREFIXES: tuple[str, ...] = ("unknown", "unlicensed")

# Provenance classes that carry licensing obligations on every example.
_LICENSE_REQUIRED_PROVENANCE: frozenset[DataProvenance] = frozenset(
    {
        DataProvenance.TEACHER_CONSENSUS,
        DataProvenance.OPTED_IN_SCRUBBED_CORRECTION,
    }
)


@dataclass(frozen=True, slots=True)
class DatasetVerdict:
    """Deterministic dataset policy result.

    usable is true only when every policy rule passes. reasons carries
    the exact failing rules. licensed and privacy_safe are governance
    facts reported separately (LICENSED != PRIVACY SAFE).
    """

    dataset_id: str
    usable: bool
    reasons: tuple[str, ...] = field(default_factory=tuple)
    licensed: bool = False
    privacy_safe: bool = False
    example_count: int = 0
    hard_negative_count: int = 0
    out_of_distribution_count: int = 0
    provenance_counts: dict[str, int] = field(default_factory=dict)
    role_counts: dict[str, int] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return {
            "dataset_id": self.dataset_id,
            "usable": self.usable,
            "reasons": list(self.reasons),
            "licensed": self.licensed,
            "privacy_safe": self.privacy_safe,
            "example_count": self.example_count,
            "hard_negative_count": self.hard_negative_count,
            "out_of_distribution_count": self.out_of_distribution_count,
            "provenance_counts": dict(self.provenance_counts),
            "role_counts": dict(self.role_counts),
        }

    def to_redacted_dict(self) -> dict[str, Any]:
        """Redacted operational payload (no secret-shaped values)."""
        redacted = redact_value(self.to_dict())
        assert isinstance(redacted, dict)
        return redacted


class DatasetPolicy:
    """Deterministic dataset policy engine.

    Args:
        prohibited_license_refs: license references that always deny.
        unknown_license_prefixes: license references starting with any of
            these prefixes are treated as unknown and deny.
    """

    def __init__(
        self,
        prohibited_license_refs: frozenset[str] = DEFAULT_PROHIBITED_LICENSE_REFS,
        unknown_license_prefixes: tuple[str, ...] = UNKNOWN_LICENSE_PREFIXES,
    ) -> None:
        self._prohibited = prohibited_license_refs
        self._unknown_prefixes = unknown_license_prefixes

    # -- policy rules ------------------------------------------------------

    def _rule_non_empty(self, dataset: MicrobrainDataset) -> str | None:
        if not dataset.examples:
            return "dataset has no examples (DATASET EXISTS != USABLE)"
        return None

    def _rule_licensed(self, dataset: MicrobrainDataset) -> str | None:
        for example in dataset.examples:
            ref = (example.license_ref or "").strip().lower()
            if not ref:
                return (
                    f"example {example.example_id} has no license_ref (MISSING LICENSE -> DENIED)"
                )
        return None

    def _rule_license_not_prohibited(self, dataset: MicrobrainDataset) -> str | None:
        for example in dataset.examples:
            ref = (example.license_ref or "").strip().lower()
            if ref in self._prohibited:
                return (
                    f"example {example.example_id} uses prohibited license "
                    f"{ref!r} (PROHIBITED LICENSE -> DENIED)"
                )
        return None

    def _rule_license_known(self, dataset: MicrobrainDataset) -> str | None:
        for example in dataset.examples:
            ref = (example.license_ref or "").strip().lower()
            if any(ref.startswith(prefix) for prefix in self._unknown_prefixes):
                return (
                    f"example {example.example_id} uses unknown license "
                    f"{ref!r} (UNKNOWN LICENSE -> DENIED)"
                )
        return None

    def _rule_hard_negative_consistency(self, dataset: MicrobrainDataset) -> str | None:
        for example in dataset.examples:
            if example.hard_negative and example.provenance is not DataProvenance.HARD_NEGATIVE:
                return (
                    f"example {example.example_id} is flagged hard_negative "
                    "but provenance is not HARD_NEGATIVE"
                )
        return None

    def _rule_provenance_known(self, dataset: MicrobrainDataset) -> str | None:
        for example in dataset.examples:
            if not isinstance(example.provenance, DataProvenance):
                return (
                    f"example {example.example_id} has unknown provenance "
                    "(UNKNOWN PROVENANCE -> DENIED)"
                )
        return None

    def _rule_role_known(self, dataset: MicrobrainDataset) -> str | None:
        for example in dataset.examples:
            if not isinstance(example.role, Role):
                return f"example {example.example_id} has unknown role (ROLE MISMATCH -> DENIED)"
        return None

    def _rule_teacher_licensed(self, dataset: MicrobrainDataset) -> str | None:
        for example in dataset.examples:
            if example.provenance in _LICENSE_REQUIRED_PROVENANCE:
                ref = (example.license_ref or "").strip()
                if not ref:
                    return (
                        f"teacher/opted-in example {example.example_id} "
                        "has no license_ref (UNFILTERED TEACHER DATA -> DENIED)"
                    )
        return None

    # -- evaluation ---------------------------------------------------------

    def evaluate(self, dataset: MicrobrainDataset) -> DatasetVerdict:
        """Evaluate a dataset against every rule, deterministically."""
        checks = (
            self._rule_non_empty,
            self._rule_licensed,
            self._rule_license_not_prohibited,
            self._rule_license_known,
            self._rule_hard_negative_consistency,
            self._rule_provenance_known,
            self._rule_role_known,
            self._rule_teacher_licensed,
        )
        reasons: list[str] = []
        for check in checks:
            reason = check(dataset)
            if reason is not None:
                reasons.append(reason)

        all_licensed = all((e.license_ref or "").strip() for e in dataset.examples)
        return DatasetVerdict(
            dataset_id=dataset.dataset_id,
            usable=not reasons,
            reasons=tuple(reasons),
            licensed=all_licensed and bool(dataset.examples),
            privacy_safe=all_licensed and bool(dataset.examples),
            example_count=len(dataset.examples),
            hard_negative_count=sum(1 for e in dataset.examples if e.hard_negative),
            out_of_distribution_count=sum(
                1 for e in dataset.examples if e.ood_verdict is OodVerdict.OUT_OF_DISTRIBUTION
            ),
            provenance_counts=dict(Counter(e.provenance.value for e in dataset.examples)),
            role_counts=dict(Counter(e.role.value for e in dataset.examples)),
        )


# ---------------------------------------------------------------------------
# Boundary helpers: real manifest I/O, fail-closed
# ---------------------------------------------------------------------------


def load_manifest(path: str | Path) -> MicrobrainDataset:
    """Load a real dataset manifest JSON file through the M1 contract.

    Fails closed with typed errors on missing file, malformed JSON, or
    any contract violation (unsupported schema version, unknown
    vocabulary, missing required field).
    """
    manifest_path = Path(path)
    try:
        raw = manifest_path.read_text(encoding="utf-8")
    except OSError as exc:
        raise MicrobrainError(
            MICROBRAIN_CODE_INVALID_INPUT,
            f"cannot read manifest {manifest_path.name}: {exc}",
        ) from exc
    try:
        data: Mapping[str, Any] = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise MicrobrainError(
            MICROBRAIN_CODE_INVALID_INPUT,
            f"manifest {manifest_path.name} is not valid JSON: {exc}",
        ) from exc
    if not isinstance(data, dict):
        raise MicrobrainError(
            MICROBRAIN_CODE_INVALID_INPUT,
            f"manifest {manifest_path.name} must be a JSON object",
        )
    return MicrobrainDataset.from_dict(data)


@dataclass(frozen=True, slots=True)
class ManifestVerification:
    """Digest verification record for a real manifest file."""

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


def sha256_manifest(path: str | Path) -> str:
    """Compute the sha256 digest of a real manifest file's bytes."""
    manifest_path = Path(path)
    try:
        digest = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    except OSError as exc:
        raise MicrobrainError(
            MICROBRAIN_CODE_INVALID_INPUT,
            f"cannot read manifest {manifest_path.name}: {exc}",
        ) from exc
    return f"sha256:{digest}"


def verify_manifest_file(
    path: str | Path,
    expected_digest: str | None = None,
) -> ManifestVerification:
    """Verify a manifest file against its current bytes.

    With expected_digest set, a mismatch is a hard denial (DIGEST
    MISMATCH -> DENIED). Without it, the current digest is recorded so
    evidence can bind to the exact file state.
    """
    manifest_path = Path(path)
    if not manifest_path.is_file():
        raise MicrobrainError(
            MICROBRAIN_CODE_MISSING_REQUIRED,
            f"manifest file missing: {manifest_path.name}",
        )
    current = sha256_manifest(manifest_path)
    if expected_digest is None:
        return ManifestVerification(
            path=str(manifest_path),
            digest=current,
            verified=True,
            reason="current-run digest recorded",
        )
    if current != expected_digest:
        return ManifestVerification(
            path=str(manifest_path),
            digest=current,
            verified=False,
            reason=(f"digest mismatch: expected {expected_digest}, observed {current}"),
        )
    return ManifestVerification(
        path=str(manifest_path),
        digest=current,
        verified=True,
        reason="digest verified against current bytes",
    )
