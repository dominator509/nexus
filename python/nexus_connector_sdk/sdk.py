"""Connector SDK surface (SPEC-022 behavior 4) - Python binding.

``ConnectorSdk`` is the shared contract corpus that the Rust,
TypeScript, and Python SDKs implement: typed capability discovery,
query, command (idempotent), health, and change-feed access through
the EP-010 capability ports. ``SdkLanguage`` marks which language
surface a binding exposes; the contract version is the shared corpus
version, so a Rust binding and a Python binding can be proven to
speak the same contract.

The SDK never grants authority: it discovers and invokes through
capability ports, and authorization to invoke remains EP-008's
boundary. Discovery results are metadata only.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from .error import CONFLICT, NOT_FOUND, VALIDATION, SdkError
from .vocabulary import SdkLanguage
from .wire import (
    CapabilityDescriptor,
    ChangeBatch,
    ChangeCursor,
    CommandRequest,
    CommandResult,
    HealthReport,
    InvocationContext,
    QueryRequest,
    QueryResult,
    WorkflowRequest,
    WorkflowResult,
)

CONTRACT_VERSION = "1.0.0"

# Capability class canonical wire strings (mirror nexus_domain).
CLASS_QUERY = "QUERY"
CLASS_COMMAND = "COMMAND"
CLASS_WORKFLOW = "WORKFLOW"
CLASS_STREAM = "STREAM"
CLASS_ADMINISTRATIVE = "ADMINISTRATIVE"


@dataclass
class IdempotencyRecord:
    """Idempotency record (SPEC-006): key bound to capability."""

    key: str
    capability_id: str
    result: dict[str, Any]


class IdempotencyTracker:
    """Deterministic idempotency tracker mirroring the Rust
    ``IdempotencyTracker``: a key is bound to the capability it was
    first used with; reusing a key for a different capability is a
    conflict."""

    def __init__(self) -> None:
        self._records: dict[str, IdempotencyRecord] = {}

    def record(self, record: IdempotencyRecord) -> SdkError | None:
        existing = self._records.get(record.key)
        if existing is not None and existing.capability_id != record.capability_id:
            return SdkError(
                CONFLICT,
                "idempotency key reused for a different capability",
                resource=record.capability_id,
            )
        self._records[record.key] = record
        return None

    def get(self, key: str) -> IdempotencyRecord | None:
        return self._records.get(key)

    def __len__(self) -> int:
        return len(self._records)


class ConnectorSdk:
    """The shared connector SDK contract (SPEC-022 behavior 4).

    Every language binding implements this surface; the same
    conformance corpus must pass against each implementation.
    """

    language: str = SdkLanguage.Python
    contract_version: str = CONTRACT_VERSION

    def __init__(self, tracker: IdempotencyTracker | None = None) -> None:
        self._tracker = tracker if tracker is not None else IdempotencyTracker()
        self._descriptors: dict[str, CapabilityDescriptor] = {}
        self._query_ports: dict[str, Any] = {}
        self._command_ports: dict[str, Any] = {}
        self._health_ports: dict[str, Any] = {}
        self._feed_ports: dict[str, Any] = {}
        self._workflow_ports: dict[str, Any] = {}

    # -- registration -----------------------------------------------------

    def register_descriptor(self, descriptor: CapabilityDescriptor) -> None:
        self._descriptors[descriptor.id] = descriptor

    def register_query(self, capability_id: str, port: Any) -> None:
        self._query_ports[capability_id] = port

    def register_command(self, capability_id: str, port: Any) -> None:
        self._command_ports[capability_id] = port

    def register_workflow(self, capability_id: str, port: Any) -> None:
        self._workflow_ports[capability_id] = port

    def register_health(self, capability_id: str, port: Any) -> None:
        self._health_ports[capability_id] = port

    def register_change_feed(self, capability_id: str, port: Any) -> None:
        self._feed_ports[capability_id] = port

    # -- surface ----------------------------------------------------------

    def discover(self, _context: InvocationContext) -> list[CapabilityDescriptor]:
        """Discover advertised capabilities (metadata only)."""
        return [d for d in self._descriptors.values() if d.availability == "AVAILABLE"]

    def query(self, request: QueryRequest) -> QueryResult:
        descriptor = self._descriptors.get(request.capability_id)
        if descriptor is None:
            raise SdkError(
                NOT_FOUND,
                "capability not found",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        if descriptor.class_ != CLASS_QUERY:
            raise SdkError(
                VALIDATION,
                "capability is not a QUERY class",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        port = self._query_ports.get(request.capability_id)
        if port is None:
            raise SdkError(
                NOT_FOUND,
                "query port not registered",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        return port.query(request)

    def command(self, request: CommandRequest) -> CommandResult:
        descriptor = self._descriptors.get(request.capability_id)
        if descriptor is None:
            raise SdkError(
                NOT_FOUND,
                "capability not found",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        if descriptor.class_ != CLASS_COMMAND:
            raise SdkError(
                VALIDATION,
                "capability is not a COMMAND class",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        port = self._command_ports.get(request.capability_id)
        if port is None:
            raise SdkError(
                NOT_FOUND,
                "command port not registered",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        key = request.idempotency_key
        if key is not None:
            existing = self._tracker.get(key)
            if existing is not None:
                if existing.capability_id != request.capability_id:
                    raise SdkError(
                        CONFLICT,
                        "idempotency key reused for a different capability",
                        request.context.correlation_id,
                        request.context.external_actor_id,
                        request.context.tenant_id,
                        request.capability_id,
                    )
                return CommandResult(
                    capability_id=existing.capability_id,
                    output=dict(existing.result),
                )
        result = port.command(request)
        if key is not None:
            self._tracker.record(
                IdempotencyRecord(
                    key=key,
                    capability_id=request.capability_id,
                    result=dict(result.to_dict().get("output", {})),
                )
            )
        return result

    def workflow(self, request: WorkflowRequest) -> WorkflowResult:
        """Start a workflow-capability invocation.

        This transports the invocation; it does NOT claim durable
        Temporal execution (EP-006 owns Temporal semantics).
        """
        descriptor = self._descriptors.get(request.capability_id)
        if descriptor is None:
            raise SdkError(
                NOT_FOUND,
                "capability not found",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        if descriptor.class_ != CLASS_WORKFLOW:
            raise SdkError(
                VALIDATION,
                "capability is not a WORKFLOW class",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        port = self._workflow_ports.get(request.capability_id)
        if port is None:
            raise SdkError(
                NOT_FOUND,
                "workflow port not registered",
                request.context.correlation_id,
                request.context.external_actor_id,
                request.context.tenant_id,
                request.capability_id,
            )
        return port.workflow(request)

    def health(self, capability_id: str, context: InvocationContext) -> HealthReport:
        port = self._health_ports.get(capability_id)
        if port is None:
            raise SdkError(
                NOT_FOUND,
                "health port not registered",
                context.correlation_id,
                context.external_actor_id,
                context.tenant_id,
                capability_id,
            )
        return port.health(context)

    def changefeed(
        self,
        capability_id: str,
        cursor: ChangeCursor | None,
        context: InvocationContext,
    ) -> ChangeBatch:
        port = self._feed_ports.get(capability_id)
        if port is None:
            raise SdkError(
                NOT_FOUND,
                "changefeed port not registered",
                context.correlation_id,
                context.external_actor_id,
                context.tenant_id,
                capability_id,
            )
        return port.changes_since(capability_id, cursor, context)


class PythonConnectorSdk(ConnectorSdk):
    """Python connector SDK binding (SPEC-022 behavior 4)."""

    language: str = SdkLanguage.Python
    contract_version: str = CONTRACT_VERSION


__all__ = [
    "CLASS_ADMINISTRATIVE",
    "CLASS_COMMAND",
    "CLASS_QUERY",
    "CLASS_STREAM",
    "CLASS_WORKFLOW",
    "CONTRACT_VERSION",
    "ConnectorSdk",
    "IdempotencyRecord",
    "IdempotencyTracker",
    "PythonConnectorSdk",
]
