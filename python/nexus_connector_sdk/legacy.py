"""Legacy poller (SPEC-022 behavior 5) - Python binding.

The ``LegacyPoller`` port wraps legacy sources that only expose
polling (REST, SOAP, SQL, CLI, files, email, browser as last resort)
and normalizes their outputs into versioned, correlated events with
stable cursors. Polling is stateful: the cursor is the only
continuity contract; a poller never claims exactly-once delivery.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from .error import UNAVAILABLE, SdkError
from .vocabulary import LegacyTransport, WebhookEvent
from .wire import InvocationContext


@dataclass
class PolledBatch:
    """One normalized poll batch."""

    capability_id: str
    events: list[WebhookEvent] = field(default_factory=list)
    next_cursor: str = ""

    def to_dict(self) -> dict[str, Any]:
        return {
            "capability_id": self.capability_id,
            "events": [e.to_dict() for e in self.events],
            "next_cursor": self.next_cursor,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> PolledBatch:
        return cls(
            capability_id=str(data["capability_id"]),
            events=[WebhookEvent.from_dict(e) for e in data.get("events", [])],
            next_cursor=str(data.get("next_cursor", "")),
        )


class LegacyPoller:
    """Port for polling a legacy source.

    Implementations wrap exactly one legacy transport and normalize
    provider outputs into versioned, correlated events. The cursor is
    the only continuity contract; pollers never claim exactly-once
    delivery (EP-005 owns durable event transport). A port without an
    implementation fails closed with a typed UNAVAILABLE error; it
    never fabricates a success batch.
    """

    def __init__(self, transport: str = LegacyTransport.Rest) -> None:
        self._transport = transport

    def transport(self) -> str:
        return self._transport

    def poll(
        self,
        capability_id: str,
        cursor: str | None,
        context: InvocationContext,
    ) -> PolledBatch:
        raise SdkError(
            UNAVAILABLE,
            "legacy poller port has no provider bound",
            correlation_id=context.correlation_id,
            actor=context.external_actor_id,
            tenant=context.tenant_id,
            resource=capability_id,
        )


__all__ = ["LegacyPoller", "PolledBatch"]
