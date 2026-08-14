"""Sandboxed legacy Connector Sidecar adapter (SPEC-022 behavior 5).

The sidecar wraps legacy sources - REST, SOAP, GraphQL, SQL read
replicas, ODBC/JDBC, CLI, files, email, webhooks, browser, or desktop
GUI as a last resort - inside a sandbox. The adapter is the port: a
transport implementation talks to a real sidecar process over a
canonical HTTP transport. The sidecar never holds direct authority;
commands stay idempotent and events stay versioned.
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from typing import Any

from .error import (
    EXTERNAL_PROVIDER,
    TIMEOUT,
    UNAVAILABLE,
    VALIDATION,
    SdkError,
)
from .vocabulary import SidecarTransport
from .wire import InvocationContext

# Canonical sidecar transport protocol version (SPEC-022 Tier 1 REST).
SIDECAR_PROTOCOL_VERSION = "1"

DEFAULT_REQUEST_TIMEOUT_SECONDS = 5.0
DEFAULT_MAX_REQUEST_BYTES = 64 * 1024


@dataclass
class SidecarRequest:
    """A normalized request to a sandboxed sidecar transport."""

    capability_id: str
    transport: str
    action: str
    input: dict[str, Any] = field(default_factory=dict)
    idempotency_key: str | None = None
    context: InvocationContext = field(
        default_factory=lambda: InvocationContext(
            request_id="018f0f6f-9c1e-7b6e-8000-000000000001",
            correlation_id="018f0f6f-9c1e-7b6e-8000-000000000002",
        )
    )

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "capability_id": self.capability_id,
            "transport": self.transport,
            "action": self.action,
            "input": self.input,
            "context": self.context.to_dict(),
        }
        # Option field serializes as JSON null (Rust serde parity).
        data["idempotency_key"] = self.idempotency_key
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> SidecarRequest:
        return cls(
            capability_id=str(data["capability_id"]),
            transport=str(data["transport"]),
            action=str(data["action"]),
            input=dict(data.get("input", {})),
            idempotency_key=data.get("idempotency_key"),
            context=InvocationContext.from_dict(data["context"]),
        )


@dataclass
class SidecarResponse:
    """Normalized sidecar response."""

    capability_id: str
    output: dict[str, Any] = field(default_factory=dict)
    cursor: str | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "capability_id": self.capability_id,
            "output": self.output,
        }
        # Option field serializes as JSON null (Rust serde parity).
        data["cursor"] = self.cursor
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> SidecarResponse:
        return cls(
            capability_id=str(data["capability_id"]),
            output=dict(data.get("output", {})),
            cursor=data.get("cursor"),
        )


class SidecarAdapter:
    """HTTP transport adapter for a sandboxed Connector Sidecar.

    Talks to a real sidecar process over HTTP on a localhost endpoint
    (bind 127.0.0.1, ephemeral port). The transport is versioned
    (``X-Nexus-Protocol-Version``); an unsupported version fails
    closed. Failures are typed and never converted into success.
    """

    def __init__(
        self,
        base_url: str,
        transport: str = SidecarTransport.Rest,
        timeout_seconds: float = DEFAULT_REQUEST_TIMEOUT_SECONDS,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._transport = transport
        self._timeout = timeout_seconds

    def transport(self) -> str:
        return self._transport

    def execute(self, request: SidecarRequest) -> SidecarResponse:
        """Execute one normalized sidecar request over real HTTP."""
        body = json.dumps(request.to_dict()).encode("utf-8")
        if len(body) > DEFAULT_MAX_REQUEST_BYTES:
            raise SdkError(
                VALIDATION,
                "sidecar request exceeds bounded size",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        headers = {
            "Content-Type": "application/json",
            "X-Nexus-Protocol-Version": SIDECAR_PROTOCOL_VERSION,
            "X-Nexus-Correlation-Id": request.context.correlation_id,
        }
        req = urllib.request.Request(
            f"{self._base_url}/v1/execute", data=body, headers=headers, method="POST"
        )
        try:
            with urllib.request.urlopen(req, timeout=self._timeout) as resp:
                payload = json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            raise self._map_http_error(exc, request) from exc
        except urllib.error.URLError as exc:
            raise SdkError(
                UNAVAILABLE,
                f"sidecar unavailable: {exc.reason}",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            ) from exc
        except TimeoutError as exc:
            raise SdkError(
                TIMEOUT,
                "sidecar request timed out",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            ) from exc
        return SidecarResponse.from_dict(payload)

    def _map_http_error(self, exc: urllib.error.HTTPError, request: SidecarRequest) -> SdkError:
        code = EXTERNAL_PROVIDER
        try:
            payload = json.loads(exc.read().decode("utf-8"))
            if isinstance(payload, dict) and "code" in payload:
                code = str(payload["code"])
                message = str(payload.get("message", "sidecar error"))
                return SdkError(
                    code,
                    message,
                    payload.get("correlation_id") or request.context.correlation_id,
                    payload.get("actor") or request.context.external_actor_id,
                    payload.get("tenant") or request.context.tenant_id,
                    payload.get("resource") or request.capability_id,
                )
        except json.JSONDecodeError, OSError:
            pass
        return SdkError(
            code,
            f"sidecar HTTP {exc.code}",
            request.context.correlation_id,
            request.context.external_actor_id,
            request.context.tenant_id,
            request.capability_id,
        )


__all__ = [
    "DEFAULT_MAX_REQUEST_BYTES",
    "DEFAULT_REQUEST_TIMEOUT_SECONDS",
    "SIDECAR_PROTOCOL_VERSION",
    "SidecarAdapter",
    "SidecarRequest",
    "SidecarResponse",
]
