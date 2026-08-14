"""Webhook normalizer (SPEC-022 behavior 2) - Python binding.

The ``WebhookNormalizer`` port converts raw webhook deliveries into
canonical, versioned, correlated ``WebhookEvent`` records and verifies
signatures. Replay detection is part of the contract; a delivery that
fails signature or replay checks is rejected and never becomes an
event. Mirrors the Rust ``WebhookNormalizer`` exactly.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from .vocabulary import WebhookEvent, WebhookVerification
from .wire import InvocationContext


@dataclass
class RawWebhook:
    """Raw webhook delivery before normalization."""

    raw_payload: dict[str, Any] = field(default_factory=dict)
    signature: str | None = None
    provider_event_id: str | None = None
    provider_event_type: str | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {"raw_payload": self.raw_payload}
        # Option fields serialize as JSON null (Rust serde parity).
        data["signature"] = self.signature
        data["provider_event_id"] = self.provider_event_id
        data["provider_event_type"] = self.provider_event_type
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> RawWebhook:
        return cls(
            raw_payload=dict(data.get("raw_payload", {})),
            signature=data.get("signature"),
            provider_event_id=data.get("provider_event_id"),
            provider_event_type=data.get("provider_event_type"),
        )


@dataclass
class NormalizedWebhook:
    """Normalized webhook outcome."""

    event: WebhookEvent | None = None
    verification: str = WebhookVerification.Invalid

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {"verification": self.verification}
        # Option field serializes as JSON null (Rust serde parity).
        data["event"] = self.event.to_dict() if self.event is not None else None
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> NormalizedWebhook:
        return cls(
            event=(
                WebhookEvent.from_dict(data["event"]) if data.get("event") is not None else None
            ),
            verification=str(data.get("verification", WebhookVerification.Invalid)),
        )


class WebhookNormalizer:
    """Port for normalizing and verifying webhook deliveries.

    Implementations convert a raw delivery into a canonical event,
    verify the signature and replay state, and reject deliveries that
    fail those checks. This base class carries the shared replay
    guard; subclasses supply the signature check.
    """

    def __init__(self, expected_fingerprint: str) -> None:
        self._expected_fingerprint = expected_fingerprint
        self._seen_event_ids: set[str] = set()

    def normalize(
        self,
        raw: RawWebhook,
        capability_id: str,
        context: InvocationContext,
    ) -> NormalizedWebhook:
        """Normalize a raw delivery into a canonical event, verifying
        the signature and replay state."""
        signature = raw.signature or ""
        if self._expected_fingerprint not in signature:
            return NormalizedWebhook(event=None, verification=WebhookVerification.Invalid)
        event_id = raw.provider_event_id or f"wh-{context.request_id}"
        if event_id in self._seen_event_ids:
            return NormalizedWebhook(event=None, verification=WebhookVerification.Replay)
        self._seen_event_ids.add(event_id)
        event = WebhookEvent(
            event_id=event_id,
            event_type=raw.provider_event_type or "webhook.received",
            version="1",
            correlation_id=context.correlation_id,
            payload=raw.raw_payload,
        )
        return NormalizedWebhook(event=event, verification=WebhookVerification.Valid)


class AcceptingWebhookNormalizer(WebhookNormalizer):
    """Deterministic normalizer that accepts a configured signature.

    Test/verification zone: used by the conformance corpus to prove
    the normalize contract; production webhook providers implement
    their own ``WebhookNormalizer`` with real signature verification.
    """

    pass


__all__ = [
    "AcceptingWebhookNormalizer",
    "NormalizedWebhook",
    "RawWebhook",
    "WebhookNormalizer",
]
