"""EP-011 M3 pytest fixtures: spawn the REAL fixture sidecar process.

The sidecar (``tests/connectors/fixture_sidecar.py``) is a real HTTP
server process on 127.0.0.1 with an ephemeral port. Every test that
uses ``sidecar`` talks to it over real HTTP - no in-process mocks, no
direct function calls (directive C).

Teardown is strict: the process is terminated in the fixture finalizer
and the port is verified closed (directive S).
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
SIDECAR = REPO_ROOT / "tests" / "connectors" / "fixture_sidecar.py"

# Make the repo-local python packages importable under `uv run
# --frozen pytest` (the project wheel only installs
# python/nexus_contracts, so the SDK package needs an explicit path).
if str(PYTHON_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))

TENANT_A = "018f0f6f-9c1e-7b6e-8000-000000000003"
TENANT_B = "018f0f6f-9c1e-7b6e-8000-000000000099"
REQUEST_ID = "018f0f6f-9c1e-7b6e-8000-000000000001"
CORRELATION_ID = "018f0f6f-9c1e-7b6e-8000-000000000002"


def make_context(tenant: str = TENANT_A, correlation: str = CORRELATION_ID) -> dict:
    return {
        "request_id": REQUEST_ID,
        "correlation_id": correlation,
        "origin_system": "pytest",
        "external_actor_id": "user:alice",
        "external_actor_type": "HUMAN",
        "tenant_id": tenant,
    }


class SidecarClient:
    """Minimal real HTTP client for the fixture sidecar."""

    def __init__(self, port: int) -> None:
        self.base = f"http://127.0.0.1:{port}"

    def post(
        self,
        path: str,
        body: dict,
        version: str = "1",
        timeout: float = 10.0,
    ) -> tuple[int, dict]:
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
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return resp.status, json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            raw = exc.read().decode("utf-8")
            try:
                return exc.code, json.loads(raw)
            except json.JSONDecodeError:
                return exc.code, {"raw": raw}
        except urllib.error.URLError as exc:
            raise AssertionError(f"sidecar unreachable: {exc.reason}") from exc

    def raw_post(self, path: str, body: bytes, version: str = "1") -> tuple[int, bytes]:
        req = urllib.request.Request(
            self.base + path,
            data=body,
            method="POST",
            headers={
                "Content-Type": "application/json",
                "X-Nexus-Protocol-Version": version,
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=10.0) as resp:
                return resp.status, resp.read()
        except urllib.error.HTTPError as exc:
            return exc.code, exc.read()
        except urllib.error.URLError as exc:
            raise AssertionError(f"sidecar unreachable: {exc.reason}") from exc


def _wait_port_closed(port: int, timeout: float = 5.0) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.settimeout(0.25)
            if sock.connect_ex(("127.0.0.1", port)) != 0:
                return True
        time.sleep(0.1)
    return False


@pytest.fixture
def sidecar() -> SidecarClient:
    env = dict(os.environ)
    proc = subprocess.Popen(
        [sys.executable, str(SIDECAR)],
        cwd=str(REPO_ROOT),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    port = 0
    try:
        line = proc.stdout.readline()  # type: ignore[union-attr]
        if not line.startswith("PORT "):
            stderr = proc.stderr.read() if proc.stderr else ""  # type: ignore[union-attr]
            proc.kill()
            raise AssertionError(f"sidecar did not print PORT: {stderr}")
        port = int(line.strip().split()[1])
        yield SidecarClient(port)
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)
        assert proc.returncode == 0, "sidecar did not shut down cleanly"
        if port:
            assert _wait_port_closed(port), "sidecar port not released (orphan!)"


@pytest.fixture
def sidecar_with_source(tmp_path: Path) -> tuple[SidecarClient, Path, Path]:
    """Sidecar with a REAL local legacy JSONL source + checkpoint."""
    source = tmp_path / "legacy-source.jsonl"
    checkpoint = tmp_path / "legacy-checkpoint.ckpt"
    source.write_text("")
    env = dict(
        os.environ,
        NEXUS_FIXTURE_SOURCE=str(source),
        NEXUS_FIXTURE_CHECKPOINT=str(checkpoint),
    )
    proc = subprocess.Popen(
        [sys.executable, str(SIDECAR)],
        cwd=str(REPO_ROOT),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    port = 0
    try:
        line = proc.stdout.readline()  # type: ignore[union-attr]
        assert line.startswith("PORT "), "sidecar did not print PORT"
        port = int(line.strip().split()[1])
        yield SidecarClient(port), source, checkpoint
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)
        if port:
            assert _wait_port_closed(port), "sidecar port not released (orphan!)"
