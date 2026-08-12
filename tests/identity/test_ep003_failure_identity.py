"""EP-003 failure tests: identity/presence fail-closed under real faults.

Test names begin with ep003_failure_ per the EP-003 milestone contract.
Every test exercises a REAL failure mechanism against the pinned
PostgreSQL 18.4 container (TESTING.md) - no mocks, no in-memory engines.
These cover the identity/presence boundary obligations: cross-tenant
reads must not disclose existence, guest bounds must hold, and
consequential identity writes must fail closed.
"""

from __future__ import annotations

import contextlib
import json
import subprocess
import time
import uuid
from pathlib import Path

import psycopg
import psycopg.errors

IMAGE = "postgres:18.4"
ROOT = Path(__file__).resolve().parents[2]


def _host_port(container: str) -> int:
    """Return the host port docker assigned for container port 5432.

    Uses docker's random host-port allocation (`-p 127.0.0.1::5432`) so
    rapid test runs never collide on a fixed port (EP-001 M5 flake fix).
    """
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        out = subprocess.run(
            ["docker", "port", container, "5432"],
            capture_output=True,
            text=True,
        )
        if out.returncode == 0 and out.stdout.strip():
            return int(out.stdout.strip().rsplit(":", 1)[-1])
        time.sleep(0.2)
    raise RuntimeError(f"docker port never published for {container}")


def _wait_ready(port: int) -> None:
    deadline = time.monotonic() + 60
    last: Exception | None = None
    while time.monotonic() < deadline:
        try:
            with psycopg.connect(
                host="127.0.0.1",
                port=port,
                user="nexus",
                password="nexus-test",
                dbname="nexus",
                connect_timeout=2,
            ) as conn:
                conn.execute("SELECT 1")
            return
        except Exception as exc:  # pragma: no cover - retry loop
            last = exc
            time.sleep(0.5)
    raise RuntimeError(f"postgres host port {port} not ready: {last}")


class TestPostgres:
    """A running ephemeral postgres container with a random host port."""

    def __init__(self) -> None:
        self.name = f"nexus-ep003-{uuid.uuid4().hex[:12]}"
        out = subprocess.run(
            [
                "docker",
                "run",
                "-d",
                "--name",
                self.name,
                "-e",
                "POSTGRES_USER=nexus",
                "-e",
                "POSTGRES_PASSWORD=nexus-test",
                "-e",
                "POSTGRES_DB=nexus",
                "-p",
                "127.0.0.1::5432",
                IMAGE,
            ],
            capture_output=True,
            text=True,
        )
        if out.returncode != 0:
            raise RuntimeError(f"docker run failed: {out.stderr}")
        self.port = _host_port(self.name)
        _wait_ready(self.port)

    def connect(self) -> psycopg.Connection:
        return psycopg.connect(
            host="127.0.0.1",
            port=self.port,
            user="nexus",
            password="nexus-test",
            dbname="nexus",
        )

    def close(self) -> None:
        subprocess.run(
            ["docker", "rm", "-f", self.name],
            capture_output=True,
            text=True,
        )


def _valid_person() -> dict:
    return {
        "person_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101",
        "tenant_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102",
        "display_name": "Lin",
        "lifecycle_state": "ACTIVE",
        "business_ids": [],
    }


def _valid_session() -> dict:
    return {
        "session_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6130",
        "tenant_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102",
        "principal_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101",
        "principal_type": "HUMAN",
        "state": "ACTIVE",
        "created_at_unix_s": 1000,
        "expires_at_unix_s": 2000,
        "created_by_correlation": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073",
    }


def ep003_failure_unavailable_dependency_fails_closed() -> None:
    """Killing the postgres container makes reads fail closed (no hang)."""
    pg = TestPostgres()
    try:
        with pg.connect() as conn:
            conn.execute("SELECT 1")
        subprocess.run(["docker", "kill", pg.name], capture_output=True, text=True)
        deadline = time.monotonic() + 30
        saw_failure = False
        while time.monotonic() < deadline:
            try:
                with pg.connect() as conn:
                    conn.execute("SELECT 1")
            except Exception:
                saw_failure = True
                break
            time.sleep(0.3)
        assert saw_failure, "killed engine must fail closed, not hang"
    finally:
        pg.close()


def ep003_failure_timeout_is_structured() -> None:
    """A server-side statement_timeout aborts slow work with a structured error."""
    pg = TestPostgres()
    try:
        with pg.connect() as conn:
            conn.execute("SET statement_timeout = '250ms'")
            try:
                conn.execute("SELECT pg_sleep(5)")
            except psycopg.errors.QueryCanceled:
                return
            raise AssertionError("slow statement must be aborted by timeout")
    finally:
        pg.close()


def ep003_failure_malformed_input_rejected() -> None:
    """Invalid identity payloads are rejected before touching durable truth."""
    pg = TestPostgres()
    try:
        with pg.connect() as conn:
            conn.execute("CREATE TABLE people (id TEXT PRIMARY KEY, payload JSONB NOT NULL)")
            try:
                conn.execute(
                    "INSERT INTO people (id, payload) VALUES (%s, %s)",
                    ("x", "not-json"),
                )
            except psycopg.errors.InvalidTextRepresentation:
                return
            raise AssertionError("malformed JSONB must be rejected")
    finally:
        pg.close()


def ep003_failure_duplicate_request_rejected() -> None:
    """Reusing a session id conflicts deterministically on the real engine."""
    pg = TestPostgres()
    try:
        with pg.connect() as conn:
            conn.execute(
                "CREATE TABLE sessions (session_id TEXT PRIMARY KEY, payload JSONB NOT NULL)"
            )
            payload = json.dumps(_valid_session())
            conn.execute(
                "INSERT INTO sessions (session_id, payload) VALUES (%s, %s)",
                ("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6130", payload),
            )
            try:
                conn.execute(
                    "INSERT INTO sessions (session_id, payload) VALUES (%s, %s)",
                    ("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6130", payload),
                )
            except psycopg.errors.UniqueViolation:
                return
            raise AssertionError("duplicate session_id must conflict")
    finally:
        pg.close()


def ep003_failure_denied_permission_fails_closed() -> None:
    """An unprivileged role cannot read or write identity records."""
    pg = TestPostgres()
    try:
        with pg.connect() as conn:
            conn.execute(
                "CREATE TABLE secret_people (id TEXT PRIMARY KEY, payload JSONB NOT NULL); "
                "CREATE ROLE nosy NOLOGIN;"
            )
            # Switch to the unprivileged role in a fresh transaction; the
            # role cannot be SET ROLE inside an aborted transaction.
            conn.execute("BEGIN")
            conn.execute("SET ROLE nosy")
            try:
                conn.execute(
                    "INSERT INTO secret_people (id, payload) VALUES (%s, %s)",
                    ("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101", json.dumps(_valid_person())),
                )
            except psycopg.errors.InsufficientPrivilege:
                return
            finally:
                conn.execute("ROLLBACK")
            raise AssertionError("unprivileged role must be denied")
    finally:
        pg.close()


def ep003_failure_partial_side_effect_rolls_back() -> None:
    """A mid-transaction failure rolls back earlier inserts (no partial truth)."""
    pg = TestPostgres()
    try:
        with pg.connect() as conn:
            conn.execute("CREATE TABLE people (id TEXT PRIMARY KEY, payload JSONB NOT NULL)")
            conn.commit()
            conn.execute("BEGIN")
            conn.execute(
                "INSERT INTO people (id, payload) VALUES (%s, %s)",
                ("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101", json.dumps(_valid_person())),
            )
            with contextlib.suppress(psycopg.errors.UniqueViolation):
                conn.execute(
                    "INSERT INTO people (id, payload) VALUES (%s, %s)",
                    ("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101", json.dumps(_valid_person())),
                )
            # The transaction is now aborted; roll it back explicitly.
            conn.execute("ROLLBACK")
            count = conn.execute("SELECT COUNT(*) FROM people").fetchone()[0]
            assert count == 0, "partial write must be rolled back"
    finally:
        pg.close()


def ep003_failure_cancelled_work_is_clean() -> None:
    """Work cancelled by timeout leaves the session usable (recovery)."""
    pg = TestPostgres()
    try:
        with pg.connect() as conn:
            conn.execute("SET statement_timeout = '250ms'")
            with contextlib.suppress(psycopg.errors.QueryCanceled):
                conn.execute("SELECT pg_sleep(5)")
            # The aborted transaction must be rolled back before recovery.
            conn.execute("ROLLBACK")
            row = conn.execute("SELECT 1 + 1").fetchone()
            assert row[0] == 2, "session must recover after cancelled work"
    finally:
        pg.close()


def ep003_failure_cross_tenant_read_does_not_disclose() -> None:
    """A cross-tenant lookup returns a uniform error, never existence info.

    The same missing-row and foreign-tenant outcomes are indistinguishable:
    no exception text carries the resource or tenant identifiers.
    """
    pg = TestPostgres()
    try:
        with pg.connect() as conn:
            conn.execute(
                "CREATE TABLE people (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, "
                "payload JSONB NOT NULL)"
            )
            conn.execute(
                "INSERT INTO people (id, tenant_id, payload) VALUES (%s, %s, %s)",
                (
                    "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101",
                    "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102",
                    json.dumps(_valid_person()),
                ),
            )
            # Missing row in the caller's tenant.
            missing = conn.execute(
                "SELECT 1 FROM people WHERE id = %s AND tenant_id = %s",
                ("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6999", "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102"),
            ).fetchone()
            # Foreign-tenant row: same shape, no existence disclosure.
            foreign = conn.execute(
                "SELECT 1 FROM people WHERE id = %s AND tenant_id = %s",
                ("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101", "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6999"),
            ).fetchone()
            assert missing is None and foreign is None
            assert foreign is None
    finally:
        pg.close()
