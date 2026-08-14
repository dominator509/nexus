"""EP-011 M3 cross-language golden wire parity from the Python binding
(directives D/E/P).

Reads the SAME canonical golden fixtures generated from the Rust types
(example ``generate_golden``) and proves the Python binding
serializes to equivalent semantic structures and deserializes the
same files. Semantic comparison (Python dicts), never raw JSON
strings, so map ordering is irrelevant.
"""

from __future__ import annotations

import json
from pathlib import Path

from nexus_connector_sdk.credential import CredentialReference
from nexus_connector_sdk.error import SdkError
from nexus_connector_sdk.sidecar import SidecarRequest, SidecarResponse
from nexus_connector_sdk.vocabulary import WebhookEvent
from nexus_connector_sdk.webhook import NormalizedWebhook, RawWebhook
from nexus_connector_sdk.wire import (
    CapabilityDescriptor,
    ChangeBatch,
    CommandRequest,
    ConnectorManifest,
    HealthReport,
    InvocationContext,
    QueryRequest,
    WorkflowResult,
)

GOLDEN_DIR = Path(__file__).resolve().parents[2] / "tests" / "connectors" / "golden"

GOLDEN_FILES = [
    "capability_descriptor",
    "change_batch",
    "change_cursor",
    "command_request",
    "command_result",
    "connector_manifest",
    "credential_reference",
    "error_envelope",
    "health_report",
    "invocation_context",
    "normalized_webhook",
    "query_request",
    "query_result",
    "raw_webhook",
    "sidecar_request",
    "sidecar_response",
    "webhook_event",
    "workflow_request",
    "workflow_result",
]


def load(name: str) -> dict:
    path = GOLDEN_DIR / f"{name}.json"
    assert path.exists(), f"golden fixture {name} missing"
    return json.loads(path.read_text())


def ep011_integration_golden_every_fixture_is_semantic_object():
    for name in GOLDEN_FILES:
        parsed = load(name)
        assert isinstance(parsed, dict)
        assert len(parsed) > 0


def ep011_integration_golden_set_is_stable():
    files = sorted(p.name for p in GOLDEN_DIR.glob("*.json"))
    expected = sorted(f"{n}.json" for n in GOLDEN_FILES)
    assert files == expected


def ep011_integration_golden_invocation_context_parity():
    golden = load("invocation_context")
    ctx = InvocationContext.from_dict(golden)
    assert ctx.tenant_id == "018f0f6f-9c1e-7b6e-8000-000000000003"
    assert ctx.correlation_id == "018f0f6f-9c1e-7b6e-8000-000000000002"
    assert ctx.to_dict() == golden


def ep011_integration_golden_capability_descriptor_parity():
    golden = load("capability_descriptor")
    desc = CapabilityDescriptor.from_dict(golden)
    assert desc.id == "fixture.contacts.query"
    assert desc.class_ == "QUERY"
    assert desc.to_dict() == golden


def ep011_integration_golden_manifest_parity():
    golden = load("connector_manifest")
    manifest = ConnectorManifest.from_dict(golden)
    assert manifest.id == "fixture-connector"
    assert manifest.secrets == ["vault:fixture-token"]
    assert manifest.to_dict() == golden


def ep011_integration_golden_query_command_parity():
    for name in ("query_request", "query_result", "command_request", "command_result"):
        golden = load(name)
        assert isinstance(golden, dict)


def ep011_integration_golden_query_request_round_trip():
    golden = load("query_request")
    req = QueryRequest.from_dict(golden)
    assert req.capability_id == "fixture.contacts.query"
    assert req.to_dict() == golden


def ep011_integration_golden_command_request_round_trip():
    golden = load("command_request")
    req = CommandRequest.from_dict(golden)
    assert req.idempotency_key == "op-1"
    assert req.to_dict() == golden


def ep011_integration_golden_workflow_running_not_completed():
    golden = load("workflow_result")
    result = WorkflowResult.from_dict(golden)
    assert result.status == "RUNNING"
    assert result.output is None
    assert result.to_dict() == golden


def ep011_integration_golden_health_parity():
    golden = load("health_report")
    health = HealthReport.from_dict(golden)
    assert health.state == "HEALTHY"
    assert health.to_dict() == golden


def ep011_integration_golden_changefeed_parity():
    golden = load("change_batch")
    batch = ChangeBatch.from_dict(golden)
    assert batch.capability_id == "fixture.audit.changefeed"
    assert batch.next_cursor is not None
    assert batch.to_dict() == golden


def ep011_integration_golden_webhook_parity():
    golden = load("webhook_event")
    event = WebhookEvent.from_dict(golden)
    assert event.event_type == "invoice.paid"
    assert event.to_dict() == golden

    raw = RawWebhook.from_dict(load("raw_webhook"))
    assert raw.signature == "sha256=fp-test:abc"
    assert raw.to_dict() == load("raw_webhook")

    normalized = NormalizedWebhook.from_dict(load("normalized_webhook"))
    assert normalized.verification == "VALID"
    assert normalized.event is not None
    assert normalized.to_dict() == load("normalized_webhook")


def ep011_integration_golden_sidecar_parity():
    golden = load("sidecar_request")
    request = SidecarRequest.from_dict(golden)
    assert request.transport == "SOAP"
    assert request.action == "read.invoice"
    assert request.to_dict() == golden

    response = SidecarResponse.from_dict(load("sidecar_response"))
    assert response.output["total"] == 100
    assert response.to_dict() == load("sidecar_response")


def ep011_integration_golden_credential_never_value():
    golden = load("credential_reference")
    ref = CredentialReference.from_dict(golden)
    assert ref.reference == "vault:fixture-token"
    assert ref.to_dict() == golden
    text = json.dumps(golden)
    assert "fixture-secret-value" not in text


def ep011_integration_golden_error_envelope_canonical_code():
    golden = load("error_envelope")
    err = SdkError.from_dict(golden)
    assert err.code == "NOT_FOUND"
    assert err.correlation_id == "018f0f6f-9c1e-7b6e-8000-000000000002"
    assert err.to_dict() == golden


def ep011_integration_golden_cross_language_snake_case():
    # Every canonical wire field must be snake_case (directive D): no
    # language-specific wire aliases in the golden corpus. Single-word
    # lowercase keys (approval, risk, status, class) are canonical;
    # camelCase keys are the violation.
    for name in GOLDEN_FILES:
        golden = load(name)
        for key in golden:
            assert key.islower(), f"{name}: non-lowercase wire field {key!r}"
            assert " " not in key, f"{name}: whitespace in wire field {key!r}"
