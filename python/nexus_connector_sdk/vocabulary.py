"""EP-011 SDK vocabulary (ADR-016) for the Python binding.

Vocabulary-locked classes for the connector SDK contract and the
sandboxed legacy Connector Sidecar (SPEC-022). Unknown classes are
rejected at parse time; wire values are canonical SCREAMING_SNAKE
strings so the Python binding matches the Rust and TypeScript
surfaces exactly.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

# ---------------------------------------------------------------------------
# Vocabulary enums (canonical wire strings; parse rejects unknowns)
# ---------------------------------------------------------------------------

SDK_LANGUAGES = ("RUST", "TYPESCRIPT", "PYTHON")
SIDECAR_TRANSPORTS = (
    "REST",
    "SOAP",
    "GRAPHQL",
    "SQL",
    "ODBC",
    "JDBC",
    "CLI",
    "FILES",
    "EMAIL",
    "WEBHOOK",
    "BROWSER",
    "DESKTOP",
)
LEGACY_TRANSPORTS = ("REST", "SOAP", "SQL", "CLI", "FILES", "EMAIL", "BROWSER")
WEBHOOK_DELIVERY_STATES = ("PENDING", "DELIVERED", "FAILED", "REPLAY")
WEBHOOK_VERIFICATIONS = ("VALID", "INVALID", "REPLAY")


def _require_member(value: str, members: tuple[str, ...], name: str) -> str:
    if value not in members:
        raise ValueError(f"unknown {name} value: {value}")
    return value


class SdkLanguage:
    """SDK language surface (SPEC-022 behavior 4)."""

    Rust = "RUST"
    TypeScript = "TYPESCRIPT"
    Python = "PYTHON"

    @staticmethod
    def parse(value: str) -> str:
        return _require_member(value, SDK_LANGUAGES, "SdkLanguage")


class SidecarTransport:
    """Transport the sandboxed Connector Sidecar wraps (SPEC-022)."""

    Rest = "REST"
    Soap = "SOAP"
    Graphql = "GRAPHQL"
    Sql = "SQL"
    Odbc = "ODBC"
    Jdbc = "JDBC"
    Cli = "CLI"
    Files = "FILES"
    Email = "EMAIL"
    Webhook = "WEBHOOK"
    Browser = "BROWSER"
    Desktop = "DESKTOP"

    @staticmethod
    def parse(value: str) -> str:
        return _require_member(value, SIDECAR_TRANSPORTS, "SidecarTransport")


class LegacyTransport:
    """Legacy source family wrapped by the LegacyPoller (SPEC-022)."""

    Rest = "REST"
    Soap = "SOAP"
    Sql = "SQL"
    Cli = "CLI"
    Files = "FILES"
    Email = "EMAIL"
    Browser = "BROWSER"

    @staticmethod
    def parse(value: str) -> str:
        return _require_member(value, LEGACY_TRANSPORTS, "LegacyTransport")


class WebhookDeliveryState:
    """Webhook delivery state (SPEC-022 behavior 2)."""

    Pending = "PENDING"
    Delivered = "DELIVERED"
    Failed = "FAILED"
    Replay = "REPLAY"

    @staticmethod
    def parse(value: str) -> str:
        return _require_member(value, WEBHOOK_DELIVERY_STATES, "WebhookDeliveryState")


class WebhookVerification:
    """Webhook verification result (mirrors the Rust enum)."""

    Valid = "VALID"
    Invalid = "INVALID"
    Replay = "REPLAY"

    @staticmethod
    def parse(value: str) -> str:
        return _require_member(value, WEBHOOK_VERIFICATIONS, "WebhookVerification")


# ---------------------------------------------------------------------------
# Canonical webhook event (SPEC-022): versioned, correlated, signed.
# ---------------------------------------------------------------------------


@dataclass
class WebhookEvent:
    """Canonical webhook event.

    The payload is a schema reference or normalized JSON value, never a
    raw provider blob. Field names match the Rust ``WebhookEvent``
    exactly.
    """

    event_id: str
    event_type: str
    version: str
    correlation_id: str
    payload: dict[str, Any]

    def to_dict(self) -> dict[str, Any]:
        return {
            "event_id": self.event_id,
            "event_type": self.event_type,
            "version": self.version,
            "correlation_id": self.correlation_id,
            "payload": self.payload,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> WebhookEvent:
        return cls(
            event_id=str(data["event_id"]),
            event_type=str(data["event_type"]),
            version=str(data["version"]),
            correlation_id=str(data["correlation_id"]),
            payload=dict(data.get("payload", {})),
        )


@dataclass
class WebhookSignature:
    """Webhook signature envelope (SPEC-022): scheme and key
    fingerprint; raw secrets never appear."""

    scheme: str
    key_fingerprint: str
    value_hex: str

    def to_dict(self) -> dict[str, Any]:
        return {
            "scheme": self.scheme,
            "key_fingerprint": self.key_fingerprint,
            "value_hex": self.value_hex,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> WebhookSignature:
        return cls(
            scheme=str(data["scheme"]),
            key_fingerprint=str(data["key_fingerprint"]),
            value_hex=str(data["value_hex"]),
        )


__all__ = [
    "LEGACY_TRANSPORTS",
    "LegacyTransport",
    "SDK_LANGUAGES",
    "SIDECAR_TRANSPORTS",
    "SdkLanguage",
    "SidecarTransport",
    "WEBHOOK_DELIVERY_STATES",
    "WEBHOOK_VERIFICATIONS",
    "WebhookDeliveryState",
    "WebhookEvent",
    "WebhookSignature",
    "WebhookVerification",
]
