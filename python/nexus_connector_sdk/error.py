"""SDK typed error (SPEC-006) for the Python binding.

Mirrors the Rust ``SdkError`` exactly: canonical failure class plus
correlation/actor/tenant/resource context. Failures fail closed: an
error is never converted into a success, and an error never contains
a secret or raw provider payload.
"""

from __future__ import annotations

from typing import Any

# Canonical SDK failure class (SPEC-006). The string values are the
# canonical wire values shared by the Rust, TypeScript, and Python
# bindings (SdkErrorCode in the Rust crate and error.ts in the
# TypeScript package use the identical set).
SdkErrorCode = {
    "VALIDATION",
    "AUTHENTICATION",
    "AUTHORIZATION",
    "POLICY",
    "UNAVAILABLE",
    "TIMEOUT",
    "CONFLICT",
    "NOT_FOUND",
    "RATE_LIMIT",
    "EXTERNAL_PROVIDER",
    "VERIFICATION",
    "COMPENSATION",
    "INTERNAL",
}

# Canonical code constants for ergonomic use.
VALIDATION = "VALIDATION"
AUTHENTICATION = "AUTHENTICATION"
AUTHORIZATION = "AUTHORIZATION"
POLICY = "POLICY"
UNAVAILABLE = "UNAVAILABLE"
TIMEOUT = "TIMEOUT"
CONFLICT = "CONFLICT"
NOT_FOUND = "NOT_FOUND"
RATE_LIMIT = "RATE_LIMIT"
EXTERNAL_PROVIDER = "EXTERNAL_PROVIDER"
VERIFICATION = "VERIFICATION"
COMPENSATION = "COMPENSATION"
INTERNAL = "INTERNAL"


class SdkError(Exception):
    """Typed SDK failure with SPEC-006 context.

    Serializes to the canonical wire envelope:
    ``{"code", "message", "correlation_id", "actor", "tenant",
    "resource"}`` with optional context fields omitted when absent -
    identical to the Rust ``SdkError`` serde output.
    """

    def __init__(
        self,
        code: str,
        message: str,
        correlation_id: str | None = None,
        actor: str | None = None,
        tenant: str | None = None,
        resource: str | None = None,
    ) -> None:
        if code not in SdkErrorCode:
            raise ValueError(f"unknown SdkErrorCode: {code}")
        super().__init__(message)
        self.code = code
        self.message = message
        self.correlation_id = correlation_id
        self.actor = actor
        self.tenant = tenant
        self.resource = resource

    def to_dict(self) -> dict[str, Any]:
        """Canonical wire envelope (snake_case, sparse context)."""
        payload: dict[str, Any] = {
            "code": self.code,
            "message": self.message,
        }
        # Option fields serialize as JSON null (Rust serde parity).
        payload["correlation_id"] = self.correlation_id
        payload["actor"] = self.actor
        payload["tenant"] = self.tenant
        payload["resource"] = self.resource
        return payload

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> SdkError:
        return cls(
            code=str(data.get("code", INTERNAL)),
            message=str(data.get("message", "unknown SDK error")),
            correlation_id=data.get("correlation_id"),
            actor=data.get("actor"),
            tenant=data.get("tenant"),
            resource=data.get("resource"),
        )

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


def validation(
    message: str,
    correlation_id: str | None = None,
    actor: str | None = None,
    tenant: str | None = None,
    resource: str | None = None,
) -> SdkError:
    return SdkError(VALIDATION, message, correlation_id, actor, tenant, resource)


def unavailable(
    message: str,
    correlation_id: str | None = None,
    actor: str | None = None,
    tenant: str | None = None,
    resource: str | None = None,
) -> SdkError:
    return SdkError(UNAVAILABLE, message, correlation_id, actor, tenant, resource)


def not_found(
    message: str,
    correlation_id: str | None = None,
    actor: str | None = None,
    tenant: str | None = None,
    resource: str | None = None,
) -> SdkError:
    return SdkError(NOT_FOUND, message, correlation_id, actor, tenant, resource)


def conflict(
    message: str,
    correlation_id: str | None = None,
    actor: str | None = None,
    tenant: str | None = None,
    resource: str | None = None,
) -> SdkError:
    return SdkError(CONFLICT, message, correlation_id, actor, tenant, resource)
