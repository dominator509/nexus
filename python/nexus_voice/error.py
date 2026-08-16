"""EP-021 typed errors (SPEC-006 codes; redaction preserved).

Voice core failures use the canonical SPEC-006 error taxonomy. Every
error preserves correlation and redacts sensitive content (raw audio
is never placed in error payloads). No raw audio, prompts, or private
content is ever emitted in error surfaces.
"""

from __future__ import annotations

from typing import Any

# SPEC-006 canonical error codes used by the voice core.
VOICE_ERROR_CODES = (
    "VALIDATION",
    "AUTHENTICATION",
    "AUTHORIZATION",
    "POLICY",
    "UNAVAILABLE",
    "TIMEOUT",
    "CONFLICT",
    "RATE_LIMIT",
    "EXTERNAL_PROVIDER",
    "VERIFICATION",
    "COMPENSATION",
    "INTERNAL_INVARIANT",
)


class VoiceErrorCode:
    """Canonical SPEC-006 error codes for the voice core."""

    Validation = "VALIDATION"
    Authentication = "AUTHENTICATION"
    Authorization = "AUTHORIZATION"
    Policy = "POLICY"
    Unavailable = "UNAVAILABLE"
    Timeout = "TIMEOUT"
    Conflict = "CONFLICT"
    RateLimit = "RATE_LIMIT"
    ExternalProvider = "EXTERNAL_PROVIDER"
    Verification = "VERIFICATION"
    Compensation = "COMPENSATION"
    InternalInvariant = "INTERNAL_INVARIANT"


def _require_code(code: str) -> str:
    if code not in VOICE_ERROR_CODES:
        raise ValueError(f"unknown voice error code: {code}")
    return code


class VoiceError(Exception):
    """Typed voice core error.

    Carries the canonical SPEC-006 code and an optional correlation id.
    The message is the only human-readable surface; it must never
    contain raw audio, prompts, or secrets.
    """

    def __init__(
        self,
        code: str,
        message: str,
        *,
        correlation_id: str | None = None,
        detail: Any = None,
    ) -> None:
        super().__init__(message)
        self.code = _require_code(code)
        self.message = message
        self.correlation_id = correlation_id
        self.detail = detail  # never raw audio or secrets; metadata only

    def as_dict(self) -> dict[str, str]:
        """Structured redacted surface (never includes raw audio)."""
        payload: dict[str, str] = {"code": self.code, "message": self.message}
        if self.correlation_id is not None:
            payload["correlation_id"] = self.correlation_id
        return payload

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"
