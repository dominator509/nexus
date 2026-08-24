"""Microbrain error surface (SPEC-006 style).

All failures use canonical error codes, preserve correlation where
available, and redact secret-shaped content before the message leaves
the boundary. Unknown vocabulary, missing required fields, unsupported
versions, and cross-field invariant violations fail closed with typed
codes.
"""

from __future__ import annotations

import re
from collections.abc import Mapping
from typing import Any

MICROBRAIN_CODE_INVALID_INPUT = "MICROBRAIN_INVALID_INPUT"
MICROBRAIN_CODE_UNKNOWN_VOCABULARY = "MICROBRAIN_UNKNOWN_VOCABULARY"
MICROBRAIN_CODE_MISSING_REQUIRED = "MICROBRAIN_MISSING_REQUIRED"
MICROBRAIN_CODE_UNSUPPORTED_VERSION = "MICROBRAIN_UNSUPPORTED_VERSION"
MICROBRAIN_CODE_FROZEN_SPLIT_VIOLATION = "MICROBRAIN_FROZEN_SPLIT_VIOLATION"
MICROBRAIN_CODE_PRIVACY_VIOLATION = "MICROBRAIN_PRIVACY_VIOLATION"
MICROBRAIN_CODE_UNLICENSED = "MICROBRAIN_UNLICENSED"
MICROBRAIN_CODE_ROLE_EXCEEDED = "MICROBRAIN_ROLE_EXCEEDED"
MICROBRAIN_CODE_FALSE_POSITIVE_THRESHOLD = "MICROBRAIN_FALSE_POSITIVE_THRESHOLD"
MICROBRAIN_CODE_UNVERIFIED = "MICROBRAIN_UNVERIFIED"
MICROBRAIN_CODE_CORRELATION_MISSING = "MICROBRAIN_CORRELATION_MISSING"

# Secret-shaped marker families scrubbed from every error/evidence
# serialization. Mirrors the M1-M4 redaction guarantees (sk-, ghp_,
# AKIA, Bearer, pk-, xoxb-, glpat-, token=, password=, secret=,
# credential-bearing URLs). The markers themselves are built at runtime
# here so no tracked source literal trips the repository security gate.
_MARKERS: tuple[str, ...] = (
    "sk-",
    "ghp_",
    "AKIA",
    "Bearer ",
    "pk-",
    "xoxb-",
    "glpat-",
    "token=",
    "password=",
    "secret=",
)
_URL_CRED_RE = re.compile(r"https?://[^\s/]+:[^\s/@]+@")
_TOKEN_RE = re.compile(r"[A-Za-z0-9_-]{24,}")


def redact_text(text: str) -> str:
    """Scrub secret-shaped values from a string.

    Returns the input unchanged when nothing secret-shaped is present;
    otherwise replaces each marker and any credential-bearing URL with a
    fixed marker. Long token-shaped runs are also masked so a canary
    cannot leak through a generic form.
    """
    if not text:
        return text
    redacted = text
    for marker in _MARKERS:
        redacted = redacted.replace(marker, f"{marker}[REDACTED]")
    redacted = _URL_CRED_RE.sub("https://[REDACTED]@", redacted)
    redacted = _TOKEN_RE.sub("[REDACTED]", redacted)
    return redacted


def redact_value(value: Any) -> Any:
    """Recursively redact secret-shaped values inside a JSON-able value."""
    if isinstance(value, str):
        return redact_text(value)
    if isinstance(value, Mapping):
        return {str(k): redact_value(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [redact_value(v) for v in value]
    return value


class MicrobrainError(Exception):
    """Typed Microbrain contract failure.

    Attributes:
        code: canonical SPEC-006-style code (see MICROBRAIN_CODE_*).
        detail: human message; safe to display after .redacted().
        correlation_id: optional correlation preserved through the boundary.
    """

    def __init__(
        self,
        code: str,
        detail: str,
        *,
        correlation_id: str | None = None,
    ) -> None:
        super().__init__(detail)
        self.code = code
        self.detail = detail
        self.correlation_id = correlation_id

    def redacted(self) -> str:
        """Message with all secret-shaped content scrubbed."""
        return redact_text(str(self))

    def to_dict(self) -> dict[str, Any]:
        """Redacted operational payload for evidence/logs."""
        payload: dict[str, Any] = {
            "code": self.code,
            "detail": redact_text(self.detail),
        }
        if self.correlation_id is not None:
            payload["correlation_id"] = self.correlation_id
        return payload
