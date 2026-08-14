"""Canonical wire shapes for the Python connector SDK.

These dataclasses mirror the Rust canonical types exactly (field
names and wire values are identical to ``nexus-capabilities`` and the
``nexus-connector-sdk`` crate). They serialize to the canonical
snake_case JSON consumed by the generated TypeScript/Python bindings
and the fixture sidecar transport.

The Python binding is a thin, dependency-free client surface: these
types only carry data and validate vocabulary; they never grant
authority.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from .vocabulary import WebhookEvent

# ---------------------------------------------------------------------------
# Invocation context (canonical; mirrors nexus-capabilities::context)
# ---------------------------------------------------------------------------


@dataclass
class InvocationContext:
    """Authenticated tenant/principal context for one invocation.

    Mirrors the Rust ``InvocationContext`` field-for-field. All
    identifiers are canonical Nexus IDs (UUIDv7 strings).
    """

    request_id: str
    correlation_id: str
    causation_id: str | None = None
    origin_system: str = "python-sdk"
    external_actor_id: str = "user:test"
    external_actor_type: str = "HUMAN"
    tenant_id: str = "018f0f6f-9c1e-7b6e-8000-000000000003"
    channel: str | None = None
    device_id: str | None = None
    objective_id: str | None = None
    task_id: str | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "request_id": self.request_id,
            "correlation_id": self.correlation_id,
            "origin_system": self.origin_system,
            "external_actor_id": self.external_actor_id,
            "external_actor_type": self.external_actor_type,
            "tenant_id": self.tenant_id,
        }
        # Option fields serialize as JSON null (Rust serde parity:
        # canonical wire output must be identical across bindings).
        data["causation_id"] = self.causation_id
        data["channel"] = self.channel
        data["device_id"] = self.device_id
        data["objective_id"] = self.objective_id
        data["task_id"] = self.task_id
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> InvocationContext:
        return cls(
            request_id=str(data["request_id"]),
            correlation_id=str(data["correlation_id"]),
            causation_id=data.get("causation_id"),
            origin_system=str(data.get("origin_system", "python-sdk")),
            external_actor_id=str(data.get("external_actor_id", "user:test")),
            external_actor_type=str(data.get("external_actor_type", "HUMAN")),
            tenant_id=str(data.get("tenant_id", "018f0f6f-9c1e-7b6e-8000-000000000003")),
            channel=data.get("channel"),
            device_id=data.get("device_id"),
            objective_id=data.get("objective_id"),
            task_id=data.get("task_id"),
        )


# ---------------------------------------------------------------------------
# Capability descriptor / manifest (canonical; mirrors nexus-capabilities)
# ---------------------------------------------------------------------------


@dataclass
class CapabilityDescriptor:
    """Stable capability advertisement (metadata only, never
    authorization). Mirrors ``CapabilityDescriptor``."""

    id: str
    version: str
    class_: str
    description: str
    input_schema: str
    output_schema: str
    required_scopes: list[str] = field(default_factory=list)
    risk: str = "R1"
    approval: str = "NONE"
    reversal: str = "NONE"
    idempotency: str = "NOT_APPLICABLE"
    availability: str = "AVAILABLE"
    locality: str | None = None
    data_classes: list[str] = field(default_factory=list)
    event_types: list[str] = field(default_factory=list)
    provider_id: str | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "id": self.id,
            "version": self.version,
            "class": self.class_,
            "description": self.description,
            "input_schema": self.input_schema,
            "output_schema": self.output_schema,
            "required_scopes": list(self.required_scopes),
            "risk": self.risk,
            "approval": self.approval,
            "reversal": self.reversal,
            "idempotency": self.idempotency,
            "availability": self.availability,
            "data_classes": list(self.data_classes),
            "event_types": list(self.event_types),
        }
        # Option fields serialize as JSON null (Rust serde parity).
        data["locality"] = self.locality
        data["provider_id"] = self.provider_id
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> CapabilityDescriptor:
        return cls(
            id=str(data["id"]),
            version=str(data["version"]),
            class_=str(data["class"]),
            description=str(data.get("description", "")),
            input_schema=str(data.get("input_schema", "")),
            output_schema=str(data.get("output_schema", "")),
            required_scopes=list(data.get("required_scopes", [])),
            risk=str(data.get("risk", "R1")),
            approval=str(data.get("approval", "NONE")),
            reversal=str(data.get("reversal", "NONE")),
            idempotency=str(data.get("idempotency", "NOT_APPLICABLE")),
            availability=str(data.get("availability", "AVAILABLE")),
            locality=data.get("locality"),
            data_classes=list(data.get("data_classes", [])),
            event_types=list(data.get("event_types", [])),
            provider_id=data.get("provider_id"),
        )


@dataclass
class ConnectorManifest:
    """Connector manifest (SPEC-022). Secrets are declared by name
    only; values never appear."""

    id: str
    version: str
    tier: str
    license: str
    runtime: str
    health: str
    capabilities: list[CapabilityDescriptor] = field(default_factory=list)
    events: list[str] = field(default_factory=list)
    secrets: list[str] = field(default_factory=list)
    network_origins: list[str] = field(default_factory=list)
    data_classes: list[str] = field(default_factory=list)
    certification: str | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "id": self.id,
            "version": self.version,
            "tier": self.tier,
            "license": self.license,
            "runtime": self.runtime,
            "health": self.health,
            "capabilities": [c.to_dict() for c in self.capabilities],
            "events": list(self.events),
            "secrets": list(self.secrets),
            "network_origins": list(self.network_origins),
            "data_classes": list(self.data_classes),
        }
        # Option field serializes as JSON null (Rust serde parity).
        data["certification"] = self.certification
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ConnectorManifest:
        return cls(
            id=str(data["id"]),
            version=str(data["version"]),
            tier=str(data["tier"]),
            license=str(data["license"]),
            runtime=str(data["runtime"]),
            health=str(data["health"]),
            capabilities=[CapabilityDescriptor.from_dict(c) for c in data.get("capabilities", [])],
            events=list(data.get("events", [])),
            secrets=list(data.get("secrets", [])),
            network_origins=list(data.get("network_origins", [])),
            data_classes=list(data.get("data_classes", [])),
            certification=data.get("certification"),
        )


# ---------------------------------------------------------------------------
# Query / command / workflow / health / changefeed wire shapes
# ---------------------------------------------------------------------------


@dataclass
class QueryRequest:
    capability_id: str
    context: InvocationContext
    input: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return {
            "capability_id": self.capability_id,
            "context": self.context.to_dict(),
            "input": self.input,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> QueryRequest:
        return cls(
            capability_id=str(data["capability_id"]),
            context=InvocationContext.from_dict(data["context"]),
            input=dict(data.get("input", {})),
        )


@dataclass
class QueryResult:
    capability_id: str
    output: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return {"capability_id": self.capability_id, "output": self.output}

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> QueryResult:
        return cls(
            capability_id=str(data["capability_id"]),
            output=dict(data.get("output", {})),
        )


@dataclass
class CommandRequest:
    capability_id: str
    context: InvocationContext
    input: dict[str, Any] = field(default_factory=dict)
    idempotency_key: str | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "capability_id": self.capability_id,
            "context": self.context.to_dict(),
            "input": self.input,
        }
        # Option field serializes as JSON null (Rust serde parity).
        data["idempotency_key"] = self.idempotency_key
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> CommandRequest:
        return cls(
            capability_id=str(data["capability_id"]),
            context=InvocationContext.from_dict(data["context"]),
            input=dict(data.get("input", {})),
            idempotency_key=data.get("idempotency_key"),
        )


@dataclass
class CommandResult:
    capability_id: str
    output: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return {"capability_id": self.capability_id, "output": self.output}

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> CommandResult:
        return cls(
            capability_id=str(data["capability_id"]),
            output=dict(data.get("output", {})),
        )


@dataclass
class WorkflowRequest:
    capability_id: str
    context: InvocationContext
    input: dict[str, Any] = field(default_factory=dict)
    idempotency_key: str | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "capability_id": self.capability_id,
            "context": self.context.to_dict(),
            "input": self.input,
        }
        # Option field serializes as JSON null (Rust serde parity).
        data["idempotency_key"] = self.idempotency_key
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> WorkflowRequest:
        return cls(
            capability_id=str(data["capability_id"]),
            context=InvocationContext.from_dict(data["context"]),
            input=dict(data.get("input", {})),
            idempotency_key=data.get("idempotency_key"),
        )


@dataclass
class WorkflowHandle:
    capability_id: str
    workflow_id: str

    def to_dict(self) -> dict[str, Any]:
        return {
            "capability_id": self.capability_id,
            "workflow_id": self.workflow_id,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> WorkflowHandle:
        return cls(
            capability_id=str(data["capability_id"]),
            workflow_id=str(data["workflow_id"]),
        )


@dataclass
class WorkflowResult:
    handle: WorkflowHandle
    status: str
    output: dict[str, Any] | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "handle": self.handle.to_dict(),
            "status": self.status,
        }
        # Option field serializes as JSON null (Rust serde parity).
        data["output"] = self.output
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> WorkflowResult:
        return cls(
            handle=WorkflowHandle.from_dict(data["handle"]),
            status=str(data["status"]),
            output=data.get("output"),
        )


@dataclass
class HealthReport:
    target_id: str
    state: str
    detail: str | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {"target_id": self.target_id, "state": self.state}
        # Option field serializes as JSON null (Rust serde parity).
        data["detail"] = self.detail
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> HealthReport:
        return cls(
            target_id=str(data["target_id"]),
            state=str(data["state"]),
            detail=data.get("detail"),
        )


@dataclass
class ChangeCursor:
    capability_id: str
    cursor: str

    def to_dict(self) -> dict[str, Any]:
        return {"capability_id": self.capability_id, "cursor": self.cursor}

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ChangeCursor:
        return cls(
            capability_id=str(data["capability_id"]),
            cursor=str(data["cursor"]),
        )


@dataclass
class ChangeEvent:
    event_id: str
    event_type: str
    payload: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return {
            "event_id": self.event_id,
            "event_type": self.event_type,
            "payload": self.payload,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ChangeEvent:
        return cls(
            event_id=str(data["event_id"]),
            event_type=str(data["event_type"]),
            payload=dict(data.get("payload", {})),
        )


@dataclass
class ChangeBatch:
    capability_id: str
    events: list[ChangeEvent] = field(default_factory=list)
    next_cursor: ChangeCursor | None = None

    def to_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "capability_id": self.capability_id,
            "events": [e.to_dict() for e in self.events],
        }
        # Option field serializes as JSON null (Rust serde parity).
        data["next_cursor"] = self.next_cursor.to_dict() if self.next_cursor is not None else None
        return data

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ChangeBatch:
        return cls(
            capability_id=str(data["capability_id"]),
            events=[ChangeEvent.from_dict(e) for e in data.get("events", [])],
            next_cursor=(
                ChangeCursor.from_dict(data["next_cursor"])
                if data.get("next_cursor") is not None
                else None
            ),
        )


__all__ = [
    "CapabilityDescriptor",
    "ChangeBatch",
    "ChangeCursor",
    "ChangeEvent",
    "CommandRequest",
    "CommandResult",
    "ConnectorManifest",
    "HealthReport",
    "InvocationContext",
    "QueryRequest",
    "QueryResult",
    "WebhookEvent",
    "WorkflowHandle",
    "WorkflowRequest",
    "WorkflowResult",
]
