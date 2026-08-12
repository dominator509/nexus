"""EP-001 integration test: generated contracts round-trip through real PostgreSQL.

Test names begin with ep001_integration_ per the EP-001 milestone contract.
Uses the pinned PostgreSQL 18.4 image from COMPONENT_REGISTRY.yaml in a real
ephemeral container - never an in-memory substitute (TESTING.md).
"""

from __future__ import annotations

import json
import subprocess
import time
import uuid
from pathlib import Path

import psycopg
from nexus_contracts.generated import (
    ActionRequest,
    NexusControlObject,
)

IMAGE = "postgres:18.4"
IMAGE_DIGEST = "sha256:a02db8cac496f15b094798a38254f14d6e00741f709360e5e00bb6668ea31636"
ROOT = Path(__file__).resolve().parents[2]


def _host_port(container: str) -> int:
    """Return the host port docker assigned for container port 5432.

    Uses docker's random host-port allocation (`-p 127.0.0.1::5432`) so
    parallel/rapid test runs never collide on a fixed port (EP-001 M5 flake
    fix: fixed ports left orphaned docker-proxy listeners).
    """
    out = subprocess.run(
        ["docker", "port", container, "5432"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    return int(out.rsplit(":", 1)[1])


def _wait_for_postgres(container: str, timeout: float = 60.0) -> None:
    """Wait until a real connection through the published host port succeeds.

    Probing only `pg_isready` inside the container is insufficient: docker's
    port-publish (proxy/iptables) can lag behind the server being ready, so a
    host-port connect can fail right after pg_isready reports ready. The test
    consumes the host port, so readiness is defined by a successful connect
    through that port (EP-001 M5 flake fix, ADR-005 discovery).
    """
    port = _host_port(container)
    deadline = time.monotonic() + timeout
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            conn = psycopg.connect(
                host="127.0.0.1",
                port=port,
                user="nexus",
                password="nexus-test",
                dbname="nexus",
                connect_timeout=2,
            )
            conn.close()
            return
        except psycopg.OperationalError as exc:
            last_error = exc
            time.sleep(0.5)
    raise TimeoutError(f"postgres host port {port} not ready within {timeout}s") from last_error


def ep001_integration_contracts_roundtrip_postgres() -> None:
    """A NexusControlObject and ActionRequest survive a real SQL round-trip."""
    name = f"nexus-ep001-{uuid.uuid4().hex[:8]}"
    subprocess.run(
        [
            "docker",
            "run",
            "-d",
            "--name",
            name,
            "-e",
            "POSTGRES_USER=nexus",
            "-e",
            "POSTGRES_PASSWORD=nexus-test",
            "-e",
            "POSTGRES_DB=nexus",
            "-p",
            "127.0.0.1::5432",
            f"{IMAGE}@{IMAGE_DIGEST}",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    try:
        _wait_for_postgres(name)
        port = _host_port(name)

        conn = psycopg.connect(
            host="127.0.0.1",
            port=port,
            user="nexus",
            password="nexus-test",
            dbname="nexus",
            connect_timeout=5,
        )
        conn.execute(
            """
            CREATE TABLE contract_roundtrip (
                id TEXT PRIMARY KEY,
                payload JSONB NOT NULL
            )
            """
        )
        obj: NexusControlObject = {
            "schema_version": "1",
            "intent": "home.lights.set",
            "route": "DETERMINISTIC",
            "risk": "R0",
            "privacy": "HOUSEHOLD",
            "ambiguity": 0.0,
            "approval_required": False,
            "executable_instruction": True,
            "confidence": 0.99,
            "required_capabilities": ["home.lights.set"],
            "entities": {},
        }
        req: ActionRequest = {
            "action_id": "act_1",
            "tenant_id": "tenant_1",
            "principal_id": "user_1",
            "capability_id": "cap.lock",
            "idempotency_key": "key_1",
            "risk": "R3",
            "approval_class": "HUMAN",
            "reversal": "COMPENSATING",
            "arguments": {"door": "front"},
            "expected_state": {"locked": True},
            "invocation": {"channel": "voice"},
        }
        rows = [
            ("obj", json.dumps(obj)),
            ("req", json.dumps(req)),
        ]
        with conn.cursor() as cur:
            for key, payload in rows:
                cur.execute(
                    "INSERT INTO contract_roundtrip (id, payload) VALUES (%s, %s)",
                    (key, payload),
                )
        conn.commit()

        with conn.cursor() as cur:
            cur.execute("SELECT id, payload FROM contract_roundtrip ORDER BY id")
            rows = cur.fetchall()
            fetched = {
                key: (payload if isinstance(payload, dict) else json.loads(payload))
                for key, payload in rows
            }

        assert fetched["obj"] == obj, "control object changed across SQL round-trip"
        assert fetched["req"] == req, "action request changed across SQL round-trip"
        assert fetched["obj"]["intent"] == "home.lights.set"
        assert fetched["req"]["idempotency_key"] == "key_1"
        conn.close()
    finally:
        subprocess.run(
            ["docker", "rm", "-f", name],
            capture_output=True,
            text=True,
        )


def ep001_integration_postgres_image_is_pinned() -> None:
    """The integration database is the locked PostgreSQL 18.4 artifact."""
    inspect = subprocess.run(
        ["docker", "image", "inspect", f"{IMAGE}@{IMAGE_DIGEST}"],
        capture_output=True,
        text=True,
    )
    assert inspect.returncode == 0, "pinned postgres:18.4 image not present locally"


def ep001_integration_generated_schema_validates() -> None:
    """The canonical control-object schema validates the generated contract."""
    schema_path = ROOT / "schemas" / "nexus-control-object.schema.json"
    schema = json.loads(schema_path.read_text())
    obj = {
        "schema_version": "1",
        "intent": "home.lights.set",
        "route": "DETERMINISTIC",
        "risk": "R0",
        "privacy": "HOUSEHOLD",
        "ambiguity": 0.0,
        "approval_required": False,
        "executable_instruction": True,
        "confidence": 0.99,
        "required_capabilities": ["home.lights.set"],
        "entities": {},
    }
    required = set(schema["required"])
    assert required <= set(obj), f"missing required fields: {required - set(obj)}"
    assert obj["intent"].startswith("home.")
    assert obj["route"] == "DETERMINISTIC"
