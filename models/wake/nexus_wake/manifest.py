"""EP-021 wake model manifest and license safety (SPEC-012, SPEC-019).

A ``WakeModelManifest`` records the model identity, digest, license
class, provenance, and replacement boundary. Noncommercial weights are
prohibited (SPEC-019 required behavior 2: noncommercial artifacts are
prohibited; SPEC-012 non-goal: shipping noncommercial wake weights).
The digest is verified against the real weights bytes before a model is
usable.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass

LICENSE_CLASSES = ("PERMISSIVE", "MPL_LGPL", "GPL", "NONCOMMERCIAL")


class LicenseClass:
    """SPEC-019 canonical license classes for model artifacts."""

    Permissive = "PERMISSIVE"
    MplLgpl = "MPL_LGPL"
    Gpl = "GPL"
    NonCommercial = "NONCOMMERCIAL"


def _require_license_class(value: str) -> str:
    if value not in LICENSE_CLASSES:
        raise ValueError(f"unknown license class: {value}")
    return value


def _require_sha256(value: str) -> str:
    if len(value) != 64:
        raise ValueError("digest_sha256 must be a 64-character hex sha256")
    try:
        int(value, 16)
    except ValueError as exc:
        raise ValueError("digest_sha256 must be hexadecimal") from exc
    return value.lower()


class WakeModelManifestError(ValueError):
    """Invalid or unsafe wake model manifest (SPEC-019)."""


@dataclass(frozen=True)
class WakeModelManifest:
    """Manifest for one wake model artifact.

    Attributes:
        model_id: stable model identity (vocabulary-safe identifier).
        version: semantic version of the weights artifact.
        digest_sha256: sha256 hex of the weights bytes.
        license_class: SPEC-019 license class (NONCOMMERCIAL rejected).
        license_name: SPDX identifier or short license name.
        provenance: source/origin description (who built it, from what).
        owner: responsible owner.
        replacement_boundary: contract describing how this model can be
            replaced (registry id / provider contract).
    """

    model_id: str
    version: str
    digest_sha256: str
    license_class: str
    license_name: str
    provenance: str
    owner: str
    replacement_boundary: str = ""

    def __post_init__(self) -> None:
        if not self.model_id or not self.model_id.strip():
            raise WakeModelManifestError("model_id must not be empty")
        if not self.version:
            raise WakeModelManifestError("version must not be empty")
        object.__setattr__(self, "digest_sha256", _require_sha256(self.digest_sha256))
        object.__setattr__(self, "license_class", _require_license_class(self.license_class))
        if not self.license_name:
            raise WakeModelManifestError("license_name must not be empty")
        if not self.provenance:
            raise WakeModelManifestError("provenance must not be empty")
        if not self.owner:
            raise WakeModelManifestError("owner must not be empty")
        if self.license_class == LicenseClass.NonCommercial:
            raise WakeModelManifestError(
                "noncommercial wake model weights are prohibited "
                "(SPEC-019 behavior 2; SPEC-012 non-goal): "
                f"{self.model_id}"
            )

    @property
    def commercial_safe(self) -> bool:
        """True when the model may ship under SPEC-019.

        PERMISSIVE is embeddable; MPL/LGPL require obligation analysis
        (carried by the manifest); GPL requires process/appliance
        isolation (recorded in the manifest); NONCOMMERCIAL is always
        rejected at construction.
        """
        return self.license_class != LicenseClass.NonCommercial


def verify_weights_digest(manifest: WakeModelManifest, weights: bytes) -> bool:
    """Verify real weights bytes against the manifest digest.

    Returns False (never raises) when the digest does not match; a model
    whose weights fail digest verification must not be usable.
    """
    return hashlib.sha256(weights).hexdigest() == manifest.digest_sha256
