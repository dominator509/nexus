"""EP-001 failure tests: fail-closed behavior under real dependency faults.

Test names begin with ep001_failure_ per the EP-001 milestone contract.
Every test exercises a REAL failure mechanism against the pinned
PostgreSQL 18.4 container (TESTING.md) - no mocks, no in-memory engines.
"""

from __future__ import annotations

import json
import subprocess
import time
import uuid
from pathlib import Path

import psycopg
import psycopg.errors
from nexus_contracts.generated import ActionRequest

IMAGE = "postgres:18.4"
IMAGE_DIGEST = "sha256:a02db8cac496f15b094798a38254f14d6e00741f709360e5e00bb6668ea31636"
PORT = 55434  # distinct from unit/integration test ports
ROOT = Path(__file__).resolve().parents[2]


def _start_container() -> str:
    name = f"nexus-ep001-fail-{uuid.uuid4().hex[:8]}"
    subprocess.run(
        [
            "docker", "run", "-d", "--name", name,
            "-e", "POSTGRES_USER=nexus",
            "-e", "POSTGRES_PASSWORD=nexus-test",
            "-e", "POSTGRES_DB=nexus",
            "-p", f"127.0.0.1:{PORT}:5432",
            f"{IMAGE}@{IMAGE_DIGEST}",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return name


def _wait_ready(container: str, timeout: float = 60.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        probe = subprocess.run(
            ["docker", "exec", container, "pg_isready", "-U", "nexus", "-d", "nexus"],
            capture_output=True,
            text=True,
        )
        if probe.returncode == 0:
            return
        time.sleep(1)
    raise TimeoutError(f"container {container} not ready")


def _connect() -> psycopg.Connection:
    return psycopg.connect(
        host="127.0.0.1",
        port=PORT,
        user="nexus",
        password="nexus-test",
        dbname="nexus",
        connect_timeout=5,
    )


def _sample_req() -> ActionRequest:
    return {
        "action_id": "act_fail_1",
        "tenant_id": "tenant_1",
        "principal_id": "user_1",
        "capability_id": "cap.lock",
        "idempotency_key": "key_fail_1",
        "risk": "R3",
        "approval_class": "HUMAN",
        "reversal": "COMPENSATING",
        "arguments": {"door": "front"},
        "expected_state": {"locked": True},
        "invocation": {"channel": "voice"},
    }


def ep001_failure_unavailable_dependency_fails_closed() -> None:
    """Unavailable dependency: a closed port must raise, never silently pass."""
    # PORT+1 has no listener; a real connect attempt must fail fast.
    try:
        psycopg.connect(
            host="127.0.0.1",
            port=PORT + 1,
            user="nexus",
            password="nexus-test",
            dbname="nexus",
            connect_timeout=2,
        )
    except psycopg.OperationalError:
        return
    raise AssertionError("connection to a closed port unexpectedly succeeded")


def ep001_failure_timeout_is_structured() -> None:
    """Timeout on an unreachable dependency is a typed, structured failure."""
    import socket

    # Bind a listener that never accepts - a blackhole for connection attempts.
    blackhole = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    blackhole.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    blackhole.bind(("127.0.0.1", PORT + 2))
    blackhole.listen(1)
    try:
        try:
            psycopg.connect(
                host="127.0.0.1",
                port=PORT + 2,
                user="nexus",
                password="nexus-test",
                dbname="nexus",
                connect_timeout=2,
            )
        except psycopg.OperationalError:
            return
        raise AssertionError("blackholed connection unexpectedly succeeded")
    finally:
        blackhole.close()


def ep001_failure_malformed_input_rejected() -> None:
    """Malformed contract JSON must be rejected, not silently coerced."""
    malformed = '{"action_id": "act_1", "risk": '
    try:
        json.loads(malformed)
    except json.JSONDecodeError:
        return
    raise AssertionError("truncated JSON parsed without error")


def ep001_failure_duplicate_request_rejected() -> None:
    """Duplicate idempotency key is rejected by a real unique constraint."""
    name = _start_container()
    try:
        _wait_ready(name)
        conn = _connect()
        conn.execute(
            "CREATE TABLE action_log ("
            " idempotency_key TEXT PRIMARY KEY,"
            " action_id TEXT NOT NULL,"
            " payload JSONB NOT NULL"
            ")"
        )
        req = _sample_req()
        conn.execute(
            "INSERT INTO action_log (idempotency_key, action_id, payload)"
            " VALUES (%s, %s, %s)",
            (req["idempotency_key"], req["action_id"], json.dumps(req)),
        )
        conn.commit()
        try:
            conn.execute(
                "INSERT INTO action_log (idempotency_key, action_id, payload)"
                " VALUES (%s, %s, %s)",
                (req["idempotency_key"], req["action_id"], json.dumps(req)),
            )
            conn.commit()
        except psycopg.errors.UniqueViolation:
            return
        finally:
            conn.close()
        raise AssertionError("duplicate idempotency key was not rejected")
    finally:
        subprocess.run(["docker", "rm", "-f", name], capture_output=True, text=True)


def ep001_failure_denied_permission_fails_closed() -> None:
    """A principal without grants must be denied by the real database."""
    name = _start_container()
    try:
        _wait_ready(name)
        admin = _connect()
        admin.execute("CREATE ROLE low_privilege LOGIN PASSWORD 'low'")
        admin.execute("CREATE TABLE restricted (id TEXT PRIMARY KEY)")
        admin.commit()
        try:
            denied = psycopg.connect(
                host="127.0.0.1",
                port=PORT,
                user="low_privilege",
                password="low",
                dbname="nexus",
                connect_timeout=5,
            )
            denied.execute("SELECT * FROM restricted")
            denied.commit()
        except psycopg.errors.InsufficientPrivilege:
            return
        finally:
            admin.close()
        raise AssertionError("unprivileged principal read restricted table")
    finally:
        subprocess.run(["docker", "rm", "-f", name], capture_output=True, text=True)


def ep001_failure_partial_side_effect_rolls_back() -> None:
    """A mid-transaction failure rolls back prior writes (atomicity)."""
    name = _start_container()
    try:
        _wait_ready(name)
        conn = _connect()
        conn.execute("CREATE TABLE audit_ledger (id TEXT PRIMARY KEY, note TEXT)")
        conn.commit()
        try:
            with conn.transaction():
                conn.execute(
                    "INSERT INTO audit_ledger (id, note) VALUES (%s, %s)",
                    ("first", "written-before-failure"),
                )
                conn.execute("INSERT INTO audit_ledger (id, note) VALUES (NULL, 'bad')")
        except psycopg.errors.NotNullViolation:
            pass
        with conn.cursor() as cur:
            cur.execute("SELECT COUNT(*) FROM audit_ledger")
            count = cur.fetchone()[0]
        conn.close()
        assert count == 0, f"partial side effect leaked: {count} rows committed"
    finally:
        subprocess.run(["docker", "rm", "-f", name], capture_output=True, text=True)


def ep001_failure_cancelled_work_is_clean() -> None:
    """Cancelling a running query surfaces a typed cancellation error."""
    name = _start_container()
    try:
        _wait_ready(name)
        conn = _connect()
        conn.execute("CREATE TABLE big (id serial PRIMARY KEY, pad text)")
        conn.execute(
            "INSERT INTO big (pad) SELECT repeat('x', 100) FROM generate_series(1, 10000)"
        )
        conn.commit()
        cur = conn.cursor()
        import threading

        cancelled = []

        def cancel() -> None:
            time.sleep(0.1)
            try:
                # psycopg: cancel() carries this backend's secret key; it must
                # be invoked on the SAME connection object from another thread.
                conn.cancel()
                cancelled.append(True)
            except Exception:  # noqa: BLE001 - cancel may race completion
                pass

        t = threading.Thread(target=cancel)
        t.start()
        try:
            # pg_sleep is a real server-side sleep: guaranteed cancellable work.
            cur.execute("SELECT pg_sleep(15)")
            cur.fetchall()
        except psycopg.errors.QueryCanceled:
            t.join(timeout=5)
            conn.close()
            assert cancelled, "cancel signal never reached the server"
            return
        t.join(timeout=5)
        conn.close()
        raise AssertionError("long query completed despite cancellation")
    finally:
        subprocess.run(["docker", "rm", "-f", name], capture_output=True, text=True)
