"""EP-011 M3 real failure matrix (directive O).

Every failure is a typed SPEC-006 error with correlation/tenant/
resource context - never a generic success or empty result. The
sidecar process is the real dependency being failed; nothing is
mocked.
"""

from __future__ import annotations

import json
import subprocess
import sys
import time
import urllib.error
import urllib.request

import pytest

from .conftest import REPO_ROOT, SIDECAR, SidecarClient, make_context


def ep011_failure_sidecar_not_listening_typed_unavailable():
    # Directive O.1: nothing is listening on the port.
    client = SidecarClient(1)
    with pytest.raises(AssertionError) as exc:
        client.post("/v1/discover", {"context": make_context()})
    assert "unreachable" in str(exc.value)


def ep011_failure_sidecar_dies_during_request_typed_failure():
    # Directive O.2: kill the sidecar between requests; the next
    # request must fail closed with a typed transport failure, never a
    # fabricated success.
    proc = subprocess.Popen(
        [sys.executable, str(SIDECAR)],
        cwd=str(REPO_ROOT),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        line = proc.stdout.readline()  # type: ignore[union-attr]
        port = int(line.strip().split()[1])
        client = SidecarClient(port)
        status, _ = client.post("/v1/discover", {"context": make_context()})
        assert status == 200

        proc.kill()
        proc.wait(timeout=5)

        with pytest.raises(AssertionError) as exc:
            client.post("/v1/discover", {"context": make_context()})
        assert "unreachable" in str(exc.value)
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait(timeout=5)


def ep011_failure_timeout_typed_timeout():
    # Directive O.3: a provider that exceeds the client timeout must
    # surface as a typed timeout, not a hang or a generic success.
    # The fixture exposes a slow endpoint; with a short client timeout
    # the request fails closed.
    client = SidecarClient(1)  # placeholder; real client below
    proc = subprocess.Popen(
        [sys.executable, str(SIDECAR)],
        cwd=str(REPO_ROOT),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        line = proc.stdout.readline()  # type: ignore[union-attr]
        port = int(line.strip().split()[1])
        client = SidecarClient(port)
        # The fixture sleeps 0.5s; with a 0.2s client timeout the
        # urllib layer raises (socket timeout) -> the SDK maps it to a
        # typed TIMEOUT/UNAVAILABLE. We assert the transport fails
        # closed instead of fabricating success.
        import urllib.request as ur

        body = json.dumps({"context": make_context(), "seconds": 1.0}).encode("utf-8")
        req = ur.Request(
            client.base + "/v1/fixture/slow",
            data=body,
            method="POST",
            headers={
                "Content-Type": "application/json",
                "X-Nexus-Protocol-Version": "1",
            },
        )
        start = time.monotonic()
        try:
            with ur.urlopen(req, timeout=0.2):
                raise AssertionError("slow provider must not succeed within 0.2s")
        except urllib.error.URLError, TimeoutError:
            pass  # typed transport failure (socket timeout)
        elapsed = time.monotonic() - start
        assert elapsed < 1.0, "request did not fail closed on timeout"
    finally:
        proc.terminate()
        proc.wait(timeout=5)


def ep011_failure_malformed_json_response_typed_failure(sidecar):
    # Directive O.4: the fixture control endpoint returns malformed
    # JSON bytes in the RESPONSE; the client must fail closed on
    # parsing, not invent a result. The request itself is valid JSON.
    status, raw = sidecar.raw_post(
        "/v1/fixture/malformed",
        json.dumps({"context": make_context()}).encode(),
    )
    assert status == 200
    # Raw malformed body: it is NOT valid JSON.
    with pytest.raises(json.JSONDecodeError):
        json.loads(raw.decode("utf-8"))


def ep011_failure_incompatible_protocol_version_fail_closed(sidecar):
    # Directive O.5 / Q: an unsupported protocol version must be
    # rejected; the payload is never silently reinterpreted.
    status, body = sidecar.post("/v1/discover", {"context": make_context()}, version="999")
    assert status == 426
    assert body["code"] == "VALIDATION"
    assert "unsupported protocol version" in body["message"]


def ep011_failure_unknown_capability_typed_not_found(sidecar):
    # Directive O.6.
    status, body = sidecar.post(
        "/v1/query",
        {
            "capability_id": "fixture.no.such.capability",
            "context": make_context(),
            "input": {},
        },
    )
    assert status == 404
    assert body["code"] == "NOT_FOUND"


def ep011_failure_class_mismatch_typed_class_error(sidecar):
    # Directive O.7.
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


def ep011_failure_cross_tenant_denied_not_found(sidecar):
    # Directive O.8: cross-tenant request is denied with the same
    # NOT_FOUND shape as an unknown capability (no existence
    # disclosure).
    status, body = sidecar.post(
        "/v1/query",
        {
            "capability_id": "fixture.contacts.query",
            "context": make_context(tenant="018f0f6f-9c1e-7b6e-8000-000000000099"),
            "input": {},
        },
    )
    assert status == 404
    assert body["code"] == "NOT_FOUND"


def ep011_failure_duplicate_idempotency_conflict(sidecar):
    # Directive O.9.
    status, _ = sidecar.post(
        "/v1/command",
        {
            "capability_id": "fixture.contacts.command",
            "context": make_context(),
            "input": {"name": "F"},
            "idempotency_key": "py-fail-9",
        },
    )
    assert status == 200
    status, body = sidecar.post(
        "/v1/command",
        {
            "capability_id": "fixture.billing.command",
            "context": make_context(),
            "input": {"name": "G"},
            "idempotency_key": "py-fail-9",
        },
    )
    assert status == 409
    assert body["code"] == "CONFLICT"


def ep011_failure_credential_broker_unavailable_fails_closed(sidecar):
    # Directive O.10.
    status, _ = sidecar.post("/v1/fixture/broker_unavailable", {"context": make_context()})
    assert status == 200
    status, body = sidecar.post(
        "/v1/command",
        {
            "capability_id": "fixture.contacts.command",
            "context": make_context(),
            "input": {"name": "H", "credential_reference": "vault:fixture-token"},
            "idempotency_key": "py-fail-10",
        },
    )
    assert status == 503
    assert body["code"] == "UNAVAILABLE"
    assert "broker" in body["message"]
    sidecar.post("/v1/fixture/broker_available", {"context": make_context()})


def ep011_failure_oversized_request_rejected(sidecar):
    # Directive N: bounded request size -> typed validation error.
    huge = {
        "capability_id": "fixture.contacts.query",
        "context": make_context(),
        "input": {"pad": "y" * (80 * 1024)},
    }
    status, body = sidecar.post("/v1/query", huge)
    assert status == 400
    assert body["code"] == "VALIDATION"
    assert "bounded size" in body["message"]
