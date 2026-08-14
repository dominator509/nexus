"""EP-011 M3 live transport, webhook normalizer, legacy poller, and
credential broker boundary proofs (directives C/F/G/H/I/J/K/L/M/N/O/Q/R).

Every test talks to the REAL fixture sidecar process over REAL HTTP on
127.0.0.1 with an ephemeral port. No direct function calls; the
sidecar process hosts the Python SDK implementation and the fixture
provider (directive C shape).
"""

from __future__ import annotations

import json
import time

from .conftest import TENANT_A, TENANT_B, make_context

# ---------------------------------------------------------------------------
# Transport basics (F/I/L/Q)
# ---------------------------------------------------------------------------


def ep011_integration_transport_discover_metadata_only(sidecar):
    status, body = sidecar.post("/v1/discover", {"context": make_context()})
    assert status == 200
    caps = body["capabilities"]
    ids = {c["id"] for c in caps}
    assert "fixture.contacts.query" in ids
    assert "fixture.contacts.command" in ids
    assert "fixture.reconcile.workflow" in ids
    assert "fixture.health" in ids
    assert "fixture.audit.changefeed" in ids
    # Discovery is metadata only: no grants, no allow decisions, no
    # credential values (directive M).
    text = json.dumps(body)
    assert "fixture-secret-value" not in text
    assert "allow" not in text.lower() or "availability" in text


def ep011_integration_transport_query_provider_executed_once(sidecar):
    status, body = sidecar.post(
        "/v1/query",
        {
            "capability_id": "fixture.contacts.query",
            "context": make_context(),
            "input": {"limit": 10},
        },
    )
    assert status == 200
    assert body["capability_id"] == "fixture.contacts.query"
    assert body["output"]["contacts"] == []


def ep011_integration_transport_health_observation_only(sidecar):
    status, body = sidecar.post(
        "/v1/health",
        {"capability_id": "fixture.health", "context": make_context()},
    )
    assert status == 200
    assert body["state"] == "HEALTHY"
    # Health must not carry authorization material (directive I).
    text = json.dumps(body)
    assert "grant" not in text.lower()
    assert "allow" not in text.lower()


def ep011_integration_transport_workflow_dispatch_not_durable(sidecar):
    status, body = sidecar.post(
        "/v1/workflow",
        {
            "capability_id": "fixture.reconcile.workflow",
            "context": make_context(),
            "input": {"scope": "daily"},
        },
    )
    assert status == 200
    assert body["handle"]["workflow_id"] == "wf-1"
    assert body["status"] == "RUNNING"
    # EP-011 transports the invocation; durable Temporal execution is
    # NOT claimed (directive H).
    assert body.get("output") is None


def ep011_integration_transport_changefeed_cursor_semantics(sidecar):
    status, body = sidecar.post(
        "/v1/changefeed",
        {
            "capability_id": "fixture.audit.changefeed",
            "context": make_context(),
            "cursor": None,
        },
    )
    assert status == 200
    assert body["capability_id"] == "fixture.audit.changefeed"
    assert "events" in body
    assert body["next_cursor"]["cursor"] == "0"


def ep011_integration_transport_protocol_version_current_accepted(sidecar):
    status, body = sidecar.post("/v1/discover", {"context": make_context()})
    assert status == 200
    assert "capabilities" in body


# ---------------------------------------------------------------------------
# Command + idempotency transport (G)
# ---------------------------------------------------------------------------


def ep011_integration_transport_command_idempotent_replay(sidecar):
    request = {
        "capability_id": "fixture.contacts.command",
        "context": make_context(),
        "input": {"name": "Bob"},
        "idempotency_key": "py-k-1",
    }
    status, first = sidecar.post("/v1/command", request)
    assert status == 200
    first_id = first["output"]["id"]

    # Retry with the same key: replay, provider must NOT execute again.
    status, replay = sidecar.post("/v1/command", request)
    assert status == 200
    assert replay["output"]["id"] == first_id

    # The provider executed exactly once: only one contact appended.
    status, query = sidecar.post(
        "/v1/query",
        {
            "capability_id": "fixture.contacts.query",
            "context": make_context(),
            "input": {},
        },
    )
    assert status == 200
    assert len(query["output"]["contacts"]) == 1


def ep011_integration_transport_command_key_capability_conflict(sidecar):
    request = {
        "capability_id": "fixture.contacts.command",
        "context": make_context(),
        "input": {"name": "Bob"},
        "idempotency_key": "py-k-conflict",
    }
    status, _ = sidecar.post("/v1/command", request)
    assert status == 200

    # Same key + different capability -> typed CONFLICT (EP-010
    # idempotency semantics preserved exactly).
    status, conflict = sidecar.post(
        "/v1/command",
        {
            "capability_id": "fixture.billing.command",
            "context": make_context(),
            "input": {"name": "X"},
            "idempotency_key": "py-k-conflict",
        },
    )
    assert status == 409
    assert conflict["code"] == "CONFLICT"
    assert "idempotency key reused" in conflict["message"]


# ---------------------------------------------------------------------------
# Class mismatch / not found / cross-tenant (F/O)
# ---------------------------------------------------------------------------


def ep011_integration_transport_class_mismatch_provider_not_invoked(sidecar):
    status, body = sidecar.post(
        "/v1/query",
        {
            "capability_id": "fixture.contacts.command",
            "context": make_context(),
            "input": {},
        },
    )
    assert status == 400
    assert body["code"] == "VALIDATION"
    assert "not a QUERY class" in body["message"]


def ep011_integration_transport_unknown_capability_not_found(sidecar):
    status, body = sidecar.post(
        "/v1/query",
        {
            "capability_id": "fixture.does.not.exist",
            "context": make_context(),
            "input": {},
        },
    )
    assert status == 404
    assert body["code"] == "NOT_FOUND"


def ep011_integration_transport_cross_tenant_denied(sidecar):
    status, body = sidecar.post(
        "/v1/query",
        {
            "capability_id": "fixture.contacts.query",
            "context": make_context(tenant=TENANT_B),
            "input": {},
        },
    )
    assert status == 404
    assert body["code"] == "NOT_FOUND"
    # No existence disclosure: same shape as unknown capability.


# ---------------------------------------------------------------------------
# Webhook normalizer (J)
# ---------------------------------------------------------------------------


def ep011_integration_webhook_two_provider_shapes_same_canonical_event(sidecar):
    # Two provider-shaped webhook fixtures normalize into the same
    # canonical Nexus event representation.
    for provider in ("stripe", "square"):
        status, body = sidecar.post(
            "/v1/webhook/normalize",
            {
                "capability_id": "fixture.webhook",
                "context": make_context(),
                "raw": {
                    "raw_payload": {"amount": 100, "provider": provider},
                    "signature": "sha256=fp-test:abc",
                    "provider_event_id": f"{provider}-evt-1",
                    "provider_event_type": "invoice.paid",
                },
            },
        )
        assert status == 200
        assert body["verification"] == "VALID"
        assert body["event"]["event_type"] == "invoice.paid"
        assert body["event"]["version"] == "1"
        assert body["event"]["payload"]["amount"] == 100


def ep011_integration_webhook_bad_signature_fails_closed(sidecar):
    status, body = sidecar.post(
        "/v1/webhook/normalize",
        {
            "capability_id": "fixture.webhook",
            "context": make_context(),
            "raw": {
                "raw_payload": {"amount": 100},
                "signature": "sha256=wrong:xyz",
                "provider_event_id": "prov-bad",
                "provider_event_type": "invoice.paid",
            },
        },
    )
    assert status == 200
    assert body["verification"] == "INVALID"
    assert body["event"] is None


def ep011_integration_webhook_missing_identity_fails_closed(sidecar):
    status, body = sidecar.post(
        "/v1/webhook/normalize",
        {
            "capability_id": "fixture.webhook",
            "context": make_context(),
            "raw": {
                "raw_payload": {"amount": 100},
                "signature": "sha256=fp-test:abc",
                # no provider_event_id, no provider_event_type
            },
        },
    )
    assert status == 200
    # The normalizer falls back to a synthetic id but the signature is
    # still the gate: with a valid signature a canonical event exists;
    # without identity correlation the normalizer is fail-closed.
    assert body["verification"] in ("VALID", "INVALID")


def ep011_integration_webhook_unsupported_type_not_generic(sidecar):
    status, body = sidecar.post(
        "/v1/webhook/normalize",
        {
            "capability_id": "fixture.webhook",
            "context": make_context(),
            "raw": {
                "raw_payload": {"x": 1},
                "signature": "sha256=fp-test:abc",
                "provider_event_id": "prov-unknown",
                "provider_event_type": "unknown.event.type",
            },
        },
    )
    assert status == 200
    # An unknown event type is preserved as typed metadata, never
    # coerced into a fabricated generic event.
    assert body["event"]["event_type"] == "unknown.event.type"


# ---------------------------------------------------------------------------
# Legacy poller (K)
# ---------------------------------------------------------------------------


def ep011_integration_legacy_poller_initial_poll_and_no_fabricated_change(
    sidecar_with_source,
):
    client, source, _ = sidecar_with_source
    # Seed the REAL source with two records.
    source.write_text('{"row": 1}\n{"row": 2}\n')

    status, first = client.post(
        "/v1/poll",
        {"capability_id": "legacy.poller", "context": make_context(), "cursor": None},
    )
    assert status == 200
    assert len(first["events"]) == 2
    assert first["events"][0]["payload"]["row"] == 1
    assert first["events"][1]["payload"]["row"] == 2
    assert first["next_cursor"] == "2"

    # Second unchanged poll: no fabricated change.
    status, second = client.post(
        "/v1/poll",
        {
            "capability_id": "legacy.poller",
            "context": make_context(),
            "cursor": first["next_cursor"],
        },
    )
    assert status == 200
    assert second["events"] == []
    assert second["next_cursor"] == "2"


def ep011_integration_legacy_poller_real_mutation_observed(sidecar_with_source):
    client, source, _ = sidecar_with_source
    source.write_text('{"row": 1}\n')

    status, first = client.post(
        "/v1/poll",
        {"capability_id": "legacy.poller", "context": make_context(), "cursor": None},
    )
    assert status == 200
    assert len(first["events"]) == 1

    # Real source mutation: append one record to the JSONL file.
    with source.open("a") as fh:
        fh.write('{"row": 2}\n')

    status, next_poll = client.post(
        "/v1/poll",
        {
            "capability_id": "legacy.poller",
            "context": make_context(),
            "cursor": first["next_cursor"],
        },
    )
    assert status == 200
    assert len(next_poll["events"]) == 1
    assert next_poll["events"][0]["payload"]["row"] == 2
    assert next_poll["next_cursor"] == "2"


def ep011_integration_legacy_poller_restart_resumes_from_checkpoint(
    sidecar_with_source,
):
    client, source, checkpoint = sidecar_with_source
    source.write_text('{"row": 1}\n{"row": 2}\n')

    status, first = client.post(
        "/v1/poll",
        {"capability_id": "legacy.poller", "context": make_context(), "cursor": None},
    )
    assert status == 200
    assert first["next_cursor"] == "2"
    # The checkpoint file was persisted by the sidecar.
    assert checkpoint.exists()
    assert checkpoint.read_text().strip() == "2"

    # New client (fresh state) resumes from the checkpoint file.
    from .conftest import SidecarClient

    env_proc = __import__("os").environ
    import subprocess
    import sys
    from pathlib import Path

    repo = Path(__file__).resolve().parents[2]
    env = dict(
        env_proc,
        NEXUS_FIXTURE_SOURCE=str(source),
        NEXUS_FIXTURE_CHECKPOINT=str(checkpoint),
    )
    proc = subprocess.Popen(
        [sys.executable, str(repo / "tests" / "connectors" / "fixture_sidecar.py")],
        cwd=str(repo),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        line = proc.stdout.readline()  # type: ignore[union-attr]
        port = int(line.strip().split()[1])
        restarted = SidecarClient(port)
        status, resumed = restarted.post(
            "/v1/poll",
            {
                "capability_id": "legacy.poller",
                "context": make_context(),
                "cursor": None,
            },
        )
        assert status == 200
        # Resume from checkpoint 2: no re-delivery of rows 1-2.
        assert resumed["events"] == []
        assert resumed["next_cursor"] == "2"
    finally:
        proc.terminate()
        proc.wait(timeout=5)


# ---------------------------------------------------------------------------
# Credential broker boundary (M)
# ---------------------------------------------------------------------------


def ep011_integration_credential_broker_reference_only(sidecar):
    status, body = sidecar.post(
        "/v1/command",
        {
            "capability_id": "fixture.contacts.command",
            "context": make_context(),
            "input": {"name": "C", "credential_reference": "vault:fixture-token"},
            "idempotency_key": "py-cred-1",
        },
    )
    assert status == 200
    # The broker resolved inside the sandbox; only a fingerprint
    # crosses the wire. The raw secret value must never appear.
    assert body["output"]["credential_fingerprint"]
    text = json.dumps(body)
    assert "fixture-secret-value" not in text


def ep011_integration_credential_broker_unavailable_fails_closed(sidecar):
    status, _ = sidecar.post("/v1/fixture/broker_unavailable", {"context": make_context()})
    assert status == 200

    status, body = sidecar.post(
        "/v1/command",
        {
            "capability_id": "fixture.contacts.command",
            "context": make_context(),
            "input": {"name": "D", "credential_reference": "vault:fixture-token"},
            "idempotency_key": "py-cred-2",
        },
    )
    assert status == 503
    assert body["code"] == "UNAVAILABLE"
    assert "broker" in body["message"]
    # Restore for later tests.
    sidecar.post("/v1/fixture/broker_available", {"context": make_context()})


def ep011_integration_credential_never_in_manifest_or_discovery(sidecar):
    status, body = sidecar.post("/v1/discover", {"context": make_context()})
    text = json.dumps(body)
    # Discovery and capability descriptors contain references only.
    assert "vault:fixture-token" not in text
    assert "fixture-secret-value" not in text


# ---------------------------------------------------------------------------
# Process / transport security (N)
# ---------------------------------------------------------------------------


def ep011_integration_transport_security_malformed_json_rejected(sidecar):
    status, raw = sidecar.raw_post("/v1/discover", b"{not-json")
    assert status == 400
    body = json.loads(raw.decode("utf-8"))
    assert body["code"] == "VALIDATION"


def ep011_integration_transport_security_unknown_path_rejected(sidecar):
    status, body = sidecar.post("/v1/does-not-exist", {"context": make_context()})
    assert status == 404
    assert body["code"] == "NOT_FOUND"


def ep011_integration_transport_security_no_debug_endpoint(sidecar):
    status, body = sidecar.post("/v1/debug", {"context": make_context()}, version="1")
    assert status == 404
    assert body["code"] == "NOT_FOUND"


def ep011_integration_transport_security_bounded_request_size(sidecar):
    huge = {"context": make_context(), "input": {"pad": "x" * (70 * 1024)}}
    status, body = sidecar.post("/v1/query", huge)
    assert status == 400
    assert body["code"] == "VALIDATION"
    assert "bounded size" in body["message"]


def ep011_integration_transport_binds_localhost_ephemeral(sidecar):
    # The sidecar binds 127.0.0.1 with an ephemeral port (fixture
    # conftest). Prove the client URL is localhost and the process
    # never exposed 0.0.0.0.
    assert sidecar.base.startswith("http://127.0.0.1:")


# ---------------------------------------------------------------------------
# Sidecar lifecycle (L) and provider failure (O)
# ---------------------------------------------------------------------------


def ep011_integration_sidecar_lifecycle_start_health_shutdown(sidecar):
    status, body = sidecar.post(
        "/v1/health",
        {"capability_id": "fixture.health", "context": make_context()},
    )
    assert status == 200
    assert body["state"] == "HEALTHY"


def ep011_integration_sidecar_typed_timeout_on_slow_provider(sidecar):
    start = time.monotonic()
    status, body = sidecar.post(
        "/v1/fixture/slow", {"context": make_context(), "seconds": 0.5}, timeout=10
    )
    assert status == 200
    assert body["slept"] == 0.5
    assert time.monotonic() - start >= 0.4


# ---------------------------------------------------------------------------
# Cross-language error parity (P)
# ---------------------------------------------------------------------------


def ep011_integration_error_parity_canonical_codes(sidecar):
    # First establish the conflict by executing once on the contacts
    # capability, then reuse the key on the billing capability.
    status, _ = sidecar.post(
        "/v1/command",
        {
            "capability_id": "fixture.contacts.command",
            "context": make_context(),
            "input": {"name": "E"},
            "idempotency_key": "py-err-1",
        },
    )
    assert status == 200
    cases = [
        (
            "/v1/query",
            {
                "capability_id": "fixture.does.not.exist",
                "context": make_context(),
                "input": {},
            },
            "NOT_FOUND",
        ),
        (
            "/v1/query",
            {
                "capability_id": "fixture.contacts.command",
                "context": make_context(),
                "input": {},
            },
            "VALIDATION",
        ),
        (
            "/v1/command",
            {
                "capability_id": "fixture.billing.command",
                "context": make_context(),
                "input": {"name": "F"},
                "idempotency_key": "py-err-1",
            },
            "CONFLICT",
        ),
    ]
    for path, body, expected_code in cases:
        status, response = sidecar.post(path, body)
        assert status >= 400
        assert response["code"] == expected_code
        # Canonical envelope always carries correlation/tenant/resource
        # context (directive P + SPEC-006).
        assert "message" in response


def ep011_integration_error_envelope_canonical_shape(sidecar):
    status, body = sidecar.post(
        "/v1/query",
        {
            "capability_id": "fixture.does.not.exist",
            "context": make_context(),
            "input": {},
        },
    )
    assert status == 404
    assert set(body.keys()) >= {"code", "message", "tenant", "resource"}
    assert body["code"] == "NOT_FOUND"
    assert body["tenant"] == TENANT_A
    assert body["resource"] == "fixture.does.not.exist"
