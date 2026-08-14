"""Nexus connector SDK - Python binding (SPEC-022 behavior 4).

Mirrors the Rust ``ConnectorSdk`` contract corpus exactly: typed
capability discovery, query, idempotent command, health, and
change-feed access over canonical snake_case wire shapes. The Python
binding speaks the same JSON contract as the Rust and TypeScript
bindings; the conformance suite in ``tests/connectors`` proves the
three bindings serialize and deserialize identical canonical
structures.

This package is the client surface only. It never grants authority:
discovery is metadata, health is observation, and every invocation
remains subject to the EP-008 authorization boundary and the EP-010
capability registry/dispatcher semantics.
"""

from __future__ import annotations

from .credential import CredentialBroker, CredentialReference, TemporaryCredential
from .error import SdkError, SdkErrorCode
from .legacy import LegacyPoller, PolledBatch
from .sdk import ConnectorSdk, PythonConnectorSdk
from .sidecar import SidecarAdapter, SidecarRequest, SidecarResponse
from .vocabulary import (
    LegacyTransport,
    SdkLanguage,
    SidecarTransport,
    WebhookDeliveryState,
    WebhookEvent,
    WebhookSignature,
    WebhookVerification,
)
from .webhook import NormalizedWebhook, RawWebhook, WebhookNormalizer
from .wire import (
    CapabilityDescriptor,
    ChangeBatch,
    ChangeCursor,
    ChangeEvent,
    CommandRequest,
    CommandResult,
    ConnectorManifest,
    HealthReport,
    InvocationContext,
    QueryRequest,
    QueryResult,
    WorkflowHandle,
    WorkflowRequest,
    WorkflowResult,
)

__all__ = [
    "CapabilityDescriptor",
    "ChangeBatch",
    "ChangeCursor",
    "ChangeEvent",
    "CommandRequest",
    "CommandResult",
    "ConnectorManifest",
    "ConnectorSdk",
    "CredentialBroker",
    "CredentialReference",
    "HealthReport",
    "InvocationContext",
    "LegacyPoller",
    "LegacyTransport",
    "NormalizedWebhook",
    "PolledBatch",
    "PythonConnectorSdk",
    "QueryRequest",
    "QueryResult",
    "RawWebhook",
    "SdkError",
    "SdkErrorCode",
    "SdkLanguage",
    "SidecarAdapter",
    "SidecarRequest",
    "SidecarResponse",
    "SidecarTransport",
    "TemporaryCredential",
    "WebhookDeliveryState",
    "WebhookEvent",
    "WebhookNormalizer",
    "WebhookSignature",
    "WebhookVerification",
    "WorkflowHandle",
    "WorkflowRequest",
    "WorkflowResult",
]

CONTRACT_VERSION = "1.0.0"
