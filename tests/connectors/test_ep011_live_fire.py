"""EP-011 M5 live-fire proof (LF-023).

Registry: "Wrap a real local legacy protocol fixture outside production
paths, discover capabilities, read state, issue an idempotent write,
and receive a change event."

The real wrapper is the nexus-sidecar binary (crates/nexus-sidecar) in
front of the real local legacy protocol fixture
(tests/connectors/fixture_sidecar.py). Every assertion crosses real
process boundaries over real loopback HTTP:

  test client -> nexus-sidecar process -> fixture process

No in-process mocks. The fixture is a test-zone provider (TESTING.md),
never a production path. The sidecar is the same binary proven in M4.

Proof chain (single wrapper, single fixture):
  1. DISCOVER  -> capabilities discovered through the wrapper
  2. QUERY     -> read state (contacts) through the wrapper
  3. COMMAND   -> idempotent write; replay returns the SAME result
  4. CHANGEFEED-> the write is observable as a change event
"""

from __future__ import annotations

import json
import os
import signal
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SIDECAR_BIN = REPO_ROOT / "target" / "debug" / "nexus-sidecar"
FIXTURE = REPO_ROOT / "tests" / "connectors" / "fixture_sidecar.py"

TENANT_A = "018f0f6f-9c1e-7b6e-8000-000000000003"
REQUEST_ID = "018f0f6f-9c1e-7b6e-8000-000000000001"
CORRELATION_ID = "018f0f6f-9c1e-7b6e-8000-000000000002"
CAPABILITIES = (
    "fixture.contacts.query:QUERY,fixture.contacts.command:COMMAND,fixture.audit.changefeed:QUERY"
)


class HttpClient:
    """Minimal real HTTP client (loopback only)."""

    def __init__(self, base: str) -> None:
        self.base = base

    def post(self, path: str, body: dict, version: str = "1") -> tuple[int, dict]:
        data = json.dumps(body).encode("utf-8")
        req = urllib.request.Request(
            self.base + path,
            data=data,
            method="POST",
            headers={
                "Content-Type": "application/json",
                "X-Nexus-Protocol-Version": version,
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=10.0) as resp:
                return resp.status, json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            raw = exc.read().decode("utf-8")
            try:
                return exc.code, json.loads(raw)
            except json.JSONDecodeError:
                return exc.code, {"raw": raw}


def _wait_port_closed(port: int, timeout: float = 5.0) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.settimeout(0.25)
            if sock.connect_ex(("127.0.0.1", port)) != 0:
                return True
        time.sleep(0.1)
    return False


def _spawn_fixture() -> tuple[subprocess.Popen[str], int]:
    proc = subprocess.Popen(
        [sys.executable, str(FIXTURE)],
        cwd=str(REPO_ROOT),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    assert proc.stdout is not None
    line = proc.stdout.readline()
    if not line.startswith("PORT "):
        proc.kill()
        raise AssertionError(f"fixture did not print PORT: {line!r}")
    return proc, int(line.strip().split()[1])


def _spawn_wrapper(provider_base: str) -> tuple[subprocess.Popen[str], int]:
    """Spawn the real nexus-sidecar wrapper around the fixture."""
    env = dict(
        os.environ,
        NEXUS_SIDECAR_TENANT=TENANT_A,
        NEXUS_SIDECAR_CONNECTOR="fixture-connector",
        NEXUS_SIDECAR_CAPABILITIES=CAPABILITIES,
        NEXUS_PROVIDER_URL=provider_base,
    )
    proc = subprocess.Popen(
        [str(SIDECAR_BIN)],
        cwd=str(REPO_ROOT),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    assert proc.stdout is not None
    line = proc.stdout.readline()
    if not line.startswith("PORT "):
        stderr = ""
        if proc.stderr is not None:
            stderr = proc.stderr.read()
        proc.kill()
        raise AssertionError(f"wrapper did not print PORT: {line!r} stderr={stderr}")
    return proc, int(line.strip().split()[1])


def _stop(proc: subprocess.Popen[str], port: int) -> None:
    """Controlled shutdown: SIGTERM, bounded wait, port release check."""
    proc.send_signal(signal.SIGTERM)
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)
    assert proc.returncode == 0, "wrapper must shut down cleanly (exit 0)"
    assert _wait_port_closed(port), "wrapper port not released (orphan!)"


def _envelope(
    capability_id: str,
    operation: str,
    input_value: dict,
    idempotency_key: str | None = None,
) -> dict:
    body = {
        "protocol_version": "1",
        "correlation_id": CORRELATION_ID,
        "request_id": REQUEST_ID,
        "tenant_id": TENANT_A,
        "connector_id": "fixture-connector",
        "capability_id": capability_id,
        "operation": operation,
        "transport": "REST",
        "schema_version": "1.0",
        "input": input_value,
    }
    if idempotency_key is not None:
        body["idempotency_key"] = idempotency_key
    return body


def ep011_livefire_wrapper_over_legacy_fixture() -> None:
    """LF-023: wrap, discover, read, idempotent write, change event."""
    fixture_proc, fixture_port = _spawn_fixture()
    fixture = HttpClient(f"http://127.0.0.1:{fixture_port}")
    try:
        wrapper_proc, wrapper_port = _spawn_wrapper(fixture.base)
    except BaseException:
        fixture_proc.kill()
        fixture_proc.wait(timeout=5)
        raise
    wrapper = HttpClient(f"http://127.0.0.1:{wrapper_port}")
    try:
        # 1. DISCOVER capabilities through the wrapper.
        status, value = wrapper.post(
            "/v1/discover",
            _envelope("fixture.contacts.query", "DISCOVER", {}),
        )
        assert status == 200, f"discover failed: {status} {value}"
        capabilities = value["capabilities"]
        ids = [c["id"] for c in capabilities]
        assert "fixture.contacts.query" in ids
        assert "fixture.contacts.command" in ids
        assert "fixture.audit.changefeed" in ids

        # 2. QUERY: read state through the wrapper.
        status, value = wrapper.post(
            "/v1/query",
            _envelope("fixture.contacts.query", "QUERY", {}),
        )
        assert status == 200, f"query failed: {status} {value}"
        contacts = value["output"]["contacts"]
        assert isinstance(contacts, list)

        # 3. COMMAND: idempotent write through the wrapper.
        write = _envelope(
            "fixture.contacts.command",
            "COMMAND",
            {"name": "lf023-target"},
            idempotency_key="lf023-op-1",
        )
        status, value = wrapper.post("/v1/command", write)
        assert status == 200, f"command failed: {status} {value}"
        first_id = value["output"]["id"]

        # Replay with the SAME key returns the SAME result (idempotent).
        status2, value2 = wrapper.post("/v1/command", write)
        assert status2 == 200, f"command replay failed: {status2} {value2}"
        assert value2["output"]["id"] == first_id, (
            "idempotent replay must return the identical result"
        )

        # 4. CHANGEFEED: the write is observable as a change event.
        status, value = wrapper.post(
            "/v1/changefeed",
            _envelope("fixture.audit.changefeed", "CHANGEFEED", {}),
        )
        assert status == 200, f"changefeed failed: {status} {value}"
        events = value["events"]
        assert isinstance(events, list)
        assert any(
            e.get("event_type") == "fixture.contact.updated"
            and e.get("payload", {}).get("id") == first_id
            for e in events
        ), "change event for the idempotent write must be observable"

        # The wrapper never claims authorization (EP-008 boundary).
        assert "authorized" not in json.dumps(value).lower()
    finally:
        _stop(wrapper_proc, wrapper_port)
        fixture_proc.kill()
        fixture_proc.wait(timeout=5)
        assert _wait_port_closed(fixture_port), "fixture port not released (orphan!)"
