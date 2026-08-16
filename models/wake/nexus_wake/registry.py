"""EP-021 wake model registry (SPEC-012 behavior 1; SPEC-019).

An idempotent registry of certified wake models. A model becomes usable
only when its manifest is commercial-safe AND its weights digest
verifies. Duplicate registration with the same digest is idempotent; a
different digest for the same model id is a conflict (never silent
overwrite).
"""

from __future__ import annotations

from .manifest import WakeModelManifest, verify_weights_digest


class WakeModelNotFound(LookupError):
    """Requested model id is not registered."""


class WakeModelConflict(ValueError):
    """Registration conflicts with an existing model id (digest differs)."""


class WakeModelRegistry:
    """In-memory registry of certified wake models.

    Production models are loaded from real manifest + weights at
    startup; the registry never fabricates a model and never registers
    a model whose digest fails verification.
    """

    def __init__(self) -> None:
        self._models: dict[str, tuple[WakeModelManifest, bytes]] = {}

    def register(self, manifest: WakeModelManifest, weights: bytes) -> bool:
        """Register a model after digest verification.

        Returns True when newly registered, False when the identical
        model (same id and digest) was already present. Raises
        ``WakeModelConflict`` when the same id has different weights,
        and ``ValueError`` when the weights fail digest verification.
        """
        if not verify_weights_digest(manifest, weights):
            raise ValueError(
                f"wake model weights digest mismatch for {manifest.model_id} (SPEC-019)"
            )
        existing = self._models.get(manifest.model_id)
        if existing is not None:
            existing_manifest, _ = existing
            if existing_manifest.digest_sha256 == manifest.digest_sha256:
                return False
            raise WakeModelConflict(
                f"wake model {manifest.model_id} already registered with a different digest"
            )
        self._models[manifest.model_id] = (manifest, weights)
        return True

    def contains(self, model_id: str) -> bool:
        return model_id in self._models

    def get(self, model_id: str) -> tuple[WakeModelManifest, bytes]:
        """Return the registered (manifest, weights)."""
        try:
            return self._models[model_id]
        except KeyError as exc:
            raise WakeModelNotFound(f"wake model not registered: {model_id}") from exc

    def ids(self) -> tuple[str, ...]:
        return tuple(sorted(self._models))

    def __len__(self) -> int:
        return len(self._models)
