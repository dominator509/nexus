#!/usr/bin/env python3
"""EP-011 M3 fixture sidecar - a REAL sandboxed Connector Sidecar.

This is a real HTTP server process (stdlib ``http.server``) that
implements the canonical sidecar REST transport used by the EP-011 M3
proofs:

  Nexus-side test client
      -> real HTTP (127.0.0.1, ephemeral port)
      -> this sidecar process
      -> Python SDK implementation + fixture provider

It is test/verification tooling only (TESTING.md test zone): the
fixture provider is deterministic and the transport carries canonical
snake_case JSON with typed SPEC-006 error envelopes. It never grants
authority, never exposes secrets, and emits redacted structured
telemetry.

Transport contract (Decision Log EP-011 M3):
- every request must carry ``X-Nexus-Protocol-Version: 1``
- JSON bodies only; bounded request size (64 KiB)
- typed error envelope: {"code","message","correlation_id","actor",
  "tenant","resource"}
- endpoints: /v1/discover /v1/query /v1/command /v1/workflow
  /v1/health /v1/changefeed /v1/webhook/normalize /v1/poll
  /v1/execute (sidecar adapter surface)
- fixture control surface under /v1/fixture/ (test zone only):
  malformed, slow, crash, broker_unavailable, broker_available, mutate
- binds 127.0.0.1 only; ephemeral port; prints "PORT <n>" to stdout
"""

from __future__ import annotations

import contextlib
import hashlib
import json
import os
import signal
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

# Make the repo-local python packages importable when run as a script.
_REPO_PYTHON = str(Path(__file__).resolve().parents[2] / "python")
if _REPO_PYTHON not in sys.path:
    sys.path.insert(0, _REPO_PYTHON)

from nexus_connector_sdk.error import (  # noqa: E402
    CONFLICT,
    NOT_FOUND,
    UNAVAILABLE,
    VALIDATION,
    SdkError,
)
from nexus_connector_sdk.vocabulary import (  # noqa: E402
    SidecarTransport,
)
from nexus_connector_sdk.webhook import RawWebhook  # noqa: E402

PROTOCOL_VERSION = "1"
MAX_REQUEST_BYTES = 64 * 1024
CONNECTOR_ID = "fixture-connector"
CONNECTOR_FINGERPRINT = hashlib.sha256(CONNECTOR_ID.encode()).hexdigest()[:16]
TENANT_A = "018f0f6f-9c1e-7b6e-8000-000000000003"
TENANT_B = "018f0f6f-9c1e-7b6e-8000-000000000099"

CAPABILITIES = [
    {
        "id": "fixture.contacts.query",
        "version": "1.0.0",
        "class": "QUERY",
        "description": "query contacts from the fixture connector",
        "input_schema": "https://schemas.nexus.local/query/v1",
        "output_schema": "https://schemas.nexus.local/query-result/v1",
        "required_scopes": ["fixture.contacts:read"],
        "risk": "R1",
        "approval": "NONE",
        "reversal": "NONE",
        "idempotency": "NOT_APPLICABLE",
        "availability": "AVAILABLE",
        "locality": "ANY",
        "data_classes": ["PUBLIC"],
        "event_types": [],
        "provider_id": "fixture-provider",
    },
    {
        "id": "fixture.contacts.command",
        "version": "1.0.0",
        "class": "COMMAND",
        "description": "append a contact idempotently",
        "input_schema": "https://schemas.nexus.local/command/v1",
        "output_schema": "https://schemas.nexus.local/command-result/v1",
        "required_scopes": ["fixture.contacts:write"],
        "risk": "R2",
        "approval": "NONE",
        "reversal": "NONE",
        "idempotency": "REQUIRED",
        "availability": "AVAILABLE",
        "locality": "ANY",
        "data_classes": ["PUBLIC"],
        "event_types": [],
        "provider_id": "fixture-provider",
    },
    {
        "id": "fixture.billing.command",
        "version": "1.0.0",
        "class": "COMMAND",
        "description": "append a billing record idempotently",
        "input_schema": "https://schemas.nexus.local/command/v1",
        "output_schema": "https://schemas.nexus.local/command-result/v1",
        "required_scopes": ["fixture.billing:write"],
        "risk": "R2",
        "approval": "NONE",
        "reversal": "NONE",
        "idempotency": "REQUIRED",
        "availability": "AVAILABLE",
        "locality": "ANY",
        "data_classes": ["PUBLIC"],
        "event_types": [],
        "provider_id": "fixture-provider",
    },
    {
        "id": "fixture.reconcile.workflow",
        "version": "1.0.0",
        "class": "WORKFLOW",
        "description": "dispatch a reconcile workflow (transport only)",
        "input_schema": "https://schemas.nexus.local/workflow/v1",
        "output_schema": "https://schemas.nexus.local/workflow-result/v1",
        "required_scopes": ["fixture.reconcile:run"],
        "risk": "R2",
        "approval": "NONE",
        "reversal": "NONE",
        "idempotency": "OPTIONAL",
        "availability": "AVAILABLE",
        "locality": "ANY",
        "data_classes": ["PUBLIC"],
        "event_types": [],
        "provider_id": "fixture-provider",
    },
    {
        "id": "fixture.health",
        "version": "1.0.0",
        "class": "QUERY",
        "description": "health observation for the fixture connector",
        "input_schema": "https://schemas.nexus.local/health/v1",
        "output_schema": "https://schemas.nexus.local/health-result/v1",
        "required_scopes": [],
        "risk": "R0",
        "approval": "NONE",
        "reversal": "NONE",
        "idempotency": "NOT_APPLICABLE",
        "availability": "AVAILABLE",
        "locality": "ANY",
        "data_classes": [],
        "event_types": [],
        "provider_id": "fixture-provider",
    },
    {
        "id": "fixture.audit.changefeed",
        "version": "1.0.0",
        "class": "STREAM",
        "description": "audit change feed for the fixture connector",
        "input_schema": "https://schemas.nexus.local/changefeed/v1",
        "output_schema": "https://schemas.nexus.local/change-batch/v1",
        "required_scopes": ["fixture.audit:read"],
        "risk": "R1",
        "approval": "NONE",
        "reversal": "NONE",
        "idempotency": "NOT_APPLICABLE",
        "availability": "AVAILABLE",
        "locality": "ANY",
        "data_classes": ["PUBLIC"],
        "event_types": ["fixture.contact.updated"],
        "provider_id": "fixture-provider",
    },
]

CLASS_BY_ID = {c["id"]: c["class"] for c in CAPABILITIES}


class FixtureProvider:
    """Deterministic fixture capability provider (test zone)."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._contacts: dict[str, list[dict[str, Any]]] = {TENANT_A: []}
        self._commands: dict[str, dict[str, Any]] = {}
        self._events: list[dict[str, Any]] = []
        self._seq = 0
        self._broker_available = True
        # Broker holds one fixture credential: reference -> value. The
        # VALUE never leaves this process.
        self._broker_values = {"vault:fixture-token": "fixture-secret-value"}
        # Legacy poller source: a real JSONL file the test mutates.
        self._source_path: Path | None = None
        self._checkpoint_path: Path | None = None
        self._cursor = 0

    # -- fixture controls ---------------------------------------------------

    def set_source(self, source_path: str, checkpoint_path: str) -> None:
        self._source_path = Path(source_path)
        self._checkpoint_path = Path(checkpoint_path)
        if self._checkpoint_path.exists():
            try:
                self._cursor = int(self._checkpoint_path.read_text().strip())
            except ValueError:
                self._cursor = 0

    def set_broker(self, available: bool) -> None:
        with self._lock:
            self._broker_available = available

    def broker_available(self) -> bool:
        with self._lock:
            return self._broker_available

    def resolve_credential(self, reference: str) -> str:
        """Resolve inside the sandbox; value never crosses the wire."""
        if not self.broker_available():
            raise SdkError(UNAVAILABLE, "credential broker unavailable", resource=reference)
        value = self._broker_values.get(reference)
        if value is None:
            raise SdkError(NOT_FOUND, "credential reference not found", resource=reference)
        return value

    def broker_fingerprint(self, reference: str) -> str:
        if not self.broker_available():
            raise SdkError(UNAVAILABLE, "credential broker unavailable", resource=reference)
        value = self._broker_values.get(reference)
        if value is None:
            raise SdkError(NOT_FOUND, "credential reference not found", resource=reference)
        return hashlib.sha256(value.encode()).hexdigest()[:16]

    def record_event(self, event_type: str, payload: dict[str, Any]) -> None:
        with self._lock:
            self._seq += 1
            self._events.append(
                {
                    "event_id": f"evt-{self._seq}",
                    "event_type": event_type,
                    "payload": payload,
                }
            )

    # -- capability implementations ----------------------------------------

    def discover(self, tenant_id: str) -> list[dict[str, Any]]:
        if tenant_id != TENANT_A:
            return []
        return [c for c in CAPABILITIES if c["availability"] == "AVAILABLE"]

    def query(self, capability_id: str, tenant_id: str, _input: dict[str, Any]) -> dict[str, Any]:
        if tenant_id != TENANT_A:
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        if capability_id != "fixture.contacts.query":
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        with self._lock:
            return {"contacts": list(self._contacts[tenant_id])}

    def command(
        self,
        capability_id: str,
        tenant_id: str,
        input: dict[str, Any],
        idempotency_key: str | None,
        correlation_id: str,
    ) -> dict[str, Any]:
        if tenant_id != TENANT_A:
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        if capability_id not in ("fixture.contacts.command", "fixture.billing.command"):
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        if idempotency_key is not None:
            with self._lock:
                existing = self._commands.get(idempotency_key)
                if existing is not None:
                    if existing["capability_id"] != capability_id:
                        raise SdkError(
                            CONFLICT,
                            "idempotency key reused for a different capability",
                            correlation_id=correlation_id,
                            tenant=tenant_id,
                            resource=capability_id,
                        )
                    return dict(existing["result"])
        # A command may reference a broker credential; resolving it
        # proves the broker boundary inside the sandbox.
        reference = input.get("credential_reference")
        fingerprint = None
        if reference is not None:
            fingerprint = self.broker_fingerprint(str(reference))
        result: dict[str, Any] = {
            "id": f"c{len(self._contacts[tenant_id]) + 1}",
        }
        if capability_id == "fixture.billing.command":
            result["billing_id"] = f"b{len(self._contacts[tenant_id]) + 1}"
        if fingerprint is not None:
            result["credential_fingerprint"] = fingerprint
        with self._lock:
            self._contacts[tenant_id].append({"name": input.get("name", "?")})
            if idempotency_key is not None:
                self._commands[idempotency_key] = {
                    "capability_id": capability_id,
                    "result": result,
                }
        self.record_event("fixture.contact.updated", {"id": result["id"]})
        return result

    def workflow(self, capability_id: str, tenant_id: str) -> dict[str, Any]:
        if tenant_id != TENANT_A:
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        if capability_id != "fixture.reconcile.workflow":
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        # Transport dispatch only: returns a RUNNING handle. This is
        # NOT durable Temporal execution (EP-006 owns Temporal).
        return {
            "handle": {
                "capability_id": capability_id,
                "workflow_id": "wf-1",
            },
            "status": "RUNNING",
            "output": None,
        }

    def health(self, capability_id: str, tenant_id: str) -> dict[str, Any]:
        if tenant_id != TENANT_A:
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        return {
            "target_id": CONNECTOR_ID,
            "state": "HEALTHY",
            "detail": "ready",
        }

    def changefeed(self, capability_id: str, tenant_id: str, cursor: str | None) -> dict[str, Any]:
        if tenant_id != TENANT_A:
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        if capability_id != "fixture.audit.changefeed":
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        with self._lock:
            since = int(cursor) if cursor is not None else 0
            events = [e for e in self._events if int(e["event_id"].split("-")[1]) > since]
            next_seq = self._seq
        return {
            "capability_id": capability_id,
            "events": events,
            "next_cursor": {"capability_id": capability_id, "cursor": str(next_seq)},
        }

    def poll(self, capability_id: str, tenant_id: str, cursor: str | None) -> dict[str, Any]:
        """Legacy poller: reads a REAL local JSONL source file."""
        if tenant_id != TENANT_A:
            raise SdkError(
                NOT_FOUND, "capability not found", tenant=tenant_id, resource=capability_id
            )
        if self._source_path is None or not self._source_path.exists():
            raise SdkError(
                UNAVAILABLE, "legacy source unavailable", tenant=tenant_id, resource=capability_id
            )
        start = int(cursor) if cursor is not None else self._cursor
        lines = self._source_path.read_text().splitlines()
        events = []
        index = 0
        for index, line in enumerate(lines):
            if index < start:
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            events.append(
                {
                    "event_id": f"legacy-{index}",
                    "event_type": "legacy.record.created",
                    "version": "1",
                    "correlation_id": "00000000-0000-7000-8000-000000000000",
                    "payload": record,
                }
            )
        next_cursor = str(max(start, len(lines)))
        if self._checkpoint_path is not None:
            self._checkpoint_path.write_text(next_cursor)
        self._cursor = int(next_cursor)
        return {
            "capability_id": capability_id,
            "events": events,
            "next_cursor": next_cursor,
        }

    def webhook_normalize(self, raw: dict[str, Any], capability_id: str) -> dict[str, Any]:
        expected = "fp-test"
        signature = raw.get("signature") or ""
        if expected not in signature:
            return {"event": None, "verification": "INVALID"}
        event_id = raw.get("provider_event_id") or "wh-unknown"
        return {
            "event": {
                "event_id": event_id,
                "event_type": raw.get("provider_event_type") or "webhook.received",
                "version": "1",
                "correlation_id": "00000000-0000-7000-8000-000000000000",
                "payload": raw.get("raw_payload", {}),
            },
            "verification": "VALID",
        }


PROVIDER = FixtureProvider()


class SidecarHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    # -- helpers ------------------------------------------------------------

    def _telemetry(
        self,
        capability_id: str,
        cls: str,
        tenant_fingerprint: str,
        result_class: str,
        correlation_id: str,
        latency_ms: float,
        error_class: str | None = None,
    ) -> None:
        """Redacted structured telemetry. Never secrets, never full
        payloads, never authorization headers."""
        entry = {
            "connector_id": CONNECTOR_ID,
            "connector_id_fingerprint": CONNECTOR_FINGERPRINT,
            "capability_id": capability_id,
            "class": cls,
            "tenant_fingerprint": tenant_fingerprint,
            "transport": "REST",
            "result_class": result_class,
            "correlation_id": correlation_id,
            "latency_ms": round(latency_ms, 3),
        }
        if error_class is not None:
            entry["error_class"] = error_class
        sys.stderr.write(json.dumps(entry) + "\n")
        sys.stderr.flush()

    def _tenant_fingerprint(self, tenant_id: str) -> str:
        return hashlib.sha256(tenant_id.encode()).hexdigest()[:16]

    def _send_json(self, status: int, payload: dict[str, Any]) -> None:
        # Test-zone control: allow raw malformed bytes to escape so the
        # client-side parse failure can be proven (directive O.4).
        if "__raw_bytes__" in payload:
            body = payload["__raw_bytes__"]  # type: ignore[assignment]
        else:
            body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("X-Nexus-Protocol-Version", PROTOCOL_VERSION)
        self.end_headers()
        self.wfile.write(body)

    def _send_error_envelope(self, status: int, err: SdkError) -> None:
        self._send_json(status, err.to_dict())

    def _read_body(self) -> dict[str, Any] | None:
        length = self.headers.get("Content-Length")
        if length is None:
            return None
        try:
            size = int(length)
        except ValueError:
            return None
        if size > MAX_REQUEST_BYTES:
            raise SdkError(
                VALIDATION,
                "request body exceeds bounded size",
                resource=self.path,
            )
        raw = self.rfile.read(size)
        try:
            return json.loads(raw.decode("utf-8"))
        except (json.JSONDecodeError, UnicodeDecodeError) as parse_err:
            raise SdkError(VALIDATION, "malformed JSON body", resource=self.path) from parse_err

    def _check_protocol(self) -> bool:
        version = self.headers.get("X-Nexus-Protocol-Version")
        if version != PROTOCOL_VERSION:
            self._send_error_envelope(
                426,
                SdkError(
                    VALIDATION,
                    f"unsupported protocol version: {version!r}",
                    resource=self.path,
                ),
            )
            return False
        return True

    # -- dispatch -----------------------------------------------------------

    def _route(self, body: dict[str, Any]) -> tuple[int, dict[str, Any]]:
        path = self.path
        correlation_id = str(body.get("context", {}).get("correlation_id", "unknown"))
        tenant_id = str(body.get("context", {}).get("tenant_id", ""))
        capability_id = str(body.get("capability_id", ""))
        cls = CLASS_BY_ID.get(capability_id, "UNKNOWN")
        start = time.monotonic()

        if path == "/v1/discover":
            capabilities = PROVIDER.discover(tenant_id)
            self._telemetry(
                capability_id,
                "DISCOVER",
                self._tenant_fingerprint(tenant_id),
                "ALLOW" if capabilities else "EMPTY",
                correlation_id,
                (time.monotonic() - start) * 1000,
            )
            return 200, {"capabilities": capabilities}

        if path == "/v1/query":
            if capability_id in CLASS_BY_ID and CLASS_BY_ID[capability_id] != "QUERY":
                raise SdkError(
                    VALIDATION,
                    "capability is not a QUERY class",
                    correlation_id=correlation_id,
                    tenant=tenant_id,
                    resource=capability_id,
                )
            try:
                result = PROVIDER.query(capability_id, tenant_id, body.get("input", {}))
            except SdkError as err:
                self._telemetry(
                    capability_id,
                    cls,
                    self._tenant_fingerprint(tenant_id),
                    "DENY",
                    correlation_id,
                    (time.monotonic() - start) * 1000,
                    err.code,
                )
                raise
            self._telemetry(
                capability_id,
                cls,
                self._tenant_fingerprint(tenant_id),
                "ALLOW",
                correlation_id,
                (time.monotonic() - start) * 1000,
            )
            return 200, {"capability_id": capability_id, "output": result}

        if path == "/v1/command":
            if capability_id in CLASS_BY_ID and CLASS_BY_ID[capability_id] != "COMMAND":
                raise SdkError(
                    VALIDATION,
                    "capability is not a COMMAND class",
                    correlation_id=correlation_id,
                    tenant=tenant_id,
                    resource=capability_id,
                )
            try:
                result = PROVIDER.command(
                    capability_id,
                    tenant_id,
                    body.get("input", {}),
                    body.get("idempotency_key"),
                    correlation_id,
                )
            except SdkError as err:
                self._telemetry(
                    capability_id,
                    cls,
                    self._tenant_fingerprint(tenant_id),
                    "DENY",
                    correlation_id,
                    (time.monotonic() - start) * 1000,
                    err.code,
                )
                raise
            self._telemetry(
                capability_id,
                cls,
                self._tenant_fingerprint(tenant_id),
                "ALLOW",
                correlation_id,
                (time.monotonic() - start) * 1000,
            )
            return 200, {"capability_id": capability_id, "output": result}

        if path == "/v1/workflow":
            if capability_id in CLASS_BY_ID and CLASS_BY_ID[capability_id] != "WORKFLOW":
                raise SdkError(
                    VALIDATION,
                    "capability is not a WORKFLOW class",
                    correlation_id=correlation_id,
                    tenant=tenant_id,
                    resource=capability_id,
                )
            try:
                result = PROVIDER.workflow(capability_id, tenant_id)
            except SdkError as err:
                self._telemetry(
                    capability_id,
                    cls,
                    self._tenant_fingerprint(tenant_id),
                    "DENY",
                    correlation_id,
                    (time.monotonic() - start) * 1000,
                    err.code,
                )
                raise
            self._telemetry(
                capability_id,
                cls,
                self._tenant_fingerprint(tenant_id),
                "ALLOW",
                correlation_id,
                (time.monotonic() - start) * 1000,
            )
            return 200, result

        if path == "/v1/health":
            try:
                result = PROVIDER.health(capability_id, tenant_id)
            except SdkError as err:
                self._telemetry(
                    capability_id,
                    cls,
                    self._tenant_fingerprint(tenant_id),
                    "DENY",
                    correlation_id,
                    (time.monotonic() - start) * 1000,
                    err.code,
                )
                raise
            self._telemetry(
                capability_id,
                cls,
                self._tenant_fingerprint(tenant_id),
                "ALLOW",
                correlation_id,
                (time.monotonic() - start) * 1000,
            )
            return 200, result

        if path == "/v1/changefeed":
            try:
                cursor = body.get("cursor")
                cursor_value = cursor.get("cursor") if isinstance(cursor, dict) else cursor
                result = PROVIDER.changefeed(capability_id, tenant_id, cursor_value)
            except SdkError as err:
                self._telemetry(
                    capability_id,
                    cls,
                    self._tenant_fingerprint(tenant_id),
                    "DENY",
                    correlation_id,
                    (time.monotonic() - start) * 1000,
                    err.code,
                )
                raise
            self._telemetry(
                capability_id,
                cls,
                self._tenant_fingerprint(tenant_id),
                "ALLOW",
                correlation_id,
                (time.monotonic() - start) * 1000,
            )
            return 200, result

        if path == "/v1/poll":
            try:
                result = PROVIDER.poll(capability_id, tenant_id, body.get("cursor"))
            except SdkError as err:
                self._telemetry(
                    capability_id,
                    cls,
                    self._tenant_fingerprint(tenant_id),
                    "DENY",
                    correlation_id,
                    (time.monotonic() - start) * 1000,
                    err.code,
                )
                raise
            self._telemetry(
                capability_id,
                cls,
                self._tenant_fingerprint(tenant_id),
                "ALLOW",
                correlation_id,
                (time.monotonic() - start) * 1000,
            )
            return 200, result

        if path == "/v1/webhook/normalize":
            try:
                raw = RawWebhook.from_dict(body.get("raw", {}))
                result = PROVIDER.webhook_normalize(raw.to_dict(), capability_id)
            except SdkError as err:
                self._telemetry(
                    capability_id,
                    "WEBHOOK",
                    self._tenant_fingerprint(tenant_id),
                    "DENY",
                    correlation_id,
                    (time.monotonic() - start) * 1000,
                    err.code,
                )
                raise
            self._telemetry(
                capability_id,
                "WEBHOOK",
                self._tenant_fingerprint(tenant_id),
                "ALLOW",
                correlation_id,
                (time.monotonic() - start) * 1000,
            )
            return 200, result

        if path == "/v1/execute":
            # SidecarAdapter surface: canonical SidecarRequest ->
            # SidecarResponse. Class-checked by capability.
            transport = body.get("transport", SidecarTransport.Rest)
            action = body.get("action", "")
            if transport not in (
                "REST",
                "SOAP",
                "GRAPHQL",
                "SQL",
                "ODBC",
                "JDBC",
                "CLI",
                "FILES",
                "EMAIL",
                "WEBHOOK",
                "BROWSER",
                "DESKTOP",
            ):
                raise SdkError(VALIDATION, "unknown sidecar transport", resource=capability_id)
            if capability_id == "legacy.erp" and action == "read.invoice":
                return 200, {
                    "capability_id": capability_id,
                    "output": {"total": 100},
                    "cursor": None,
                }
            raise SdkError(NOT_FOUND, "sidecar action not found", resource=capability_id)

        # Fixture control surface (test zone; never exposes secrets).
        if path == "/v1/fixture/malformed":
            # Returns genuinely malformed JSON bytes so the client must
            # fail closed on parsing (directive O.4).
            return 200, {"__raw_bytes__": b"not-json{{{"}  # type: ignore[dict-item]

        if path == "/v1/fixture/slow":
            delay = float(body.get("seconds", 5))
            time.sleep(delay)
            return 200, {"slept": delay}

        if path == "/v1/fixture/crash":
            os._exit(1)

        if path == "/v1/fixture/broker_unavailable":
            PROVIDER.set_broker(False)
            return 200, {"broker": "unavailable"}

        if path == "/v1/fixture/broker_available":
            PROVIDER.set_broker(True)
            return 200, {"broker": "available"}

        if path == "/v1/fixture/mutate":
            if PROVIDER._source_path is not None:
                with PROVIDER._source_path.open("a") as fh:
                    fh.write(json.dumps(body.get("record", {"row": 1})) + "\n")
                return 200, {"mutated": True}
            raise SdkError(UNAVAILABLE, "legacy source not configured", resource=capability_id)

        if path == "/v1/fixture/healthz":
            return 200, {"status": "ok"}

        raise SdkError(NOT_FOUND, "unknown path", resource=path)

    # -- HTTP verbs ----------------------------------------------------------

    def _handle(self) -> None:
        if not self._check_protocol():
            return
        try:
            body = self._read_body()
        except SdkError as err:
            self._send_error_envelope(400, err)
            return
        if body is None:
            self._send_error_envelope(
                400, SdkError(VALIDATION, "JSON body required", resource=self.path)
            )
            return
        try:
            status, payload = self._route(body)
            if status is None:
                return
            self._send_json(status, payload)
        except SdkError as err:
            status = 400
            if err.code == NOT_FOUND:
                status = 404
            elif err.code == CONFLICT:
                status = 409
            elif err.code == UNAVAILABLE:
                status = 503
            self._send_error_envelope(status, err)

    def do_POST(self) -> None:  # noqa: N802
        self._handle()

    def do_GET(self) -> None:  # noqa: N802
        if self.path == "/v1/fixture/healthz":
            self._send_json(200, {"status": "ok"})
            return
        self._send_error_envelope(
            405, SdkError(VALIDATION, "method not allowed", resource=self.path)
        )

    def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
        # Redacted access log: never query strings with secrets, never
        # headers. Keep it minimal and safe.
        sys.stderr.write(f"sidecar access: {self.address_string()} {format % args}\n")


def main() -> int:
    port = int(os.environ.get("NEXUS_FIXTURE_PORT", "0"))
    source = os.environ.get("NEXUS_FIXTURE_SOURCE")
    checkpoint = os.environ.get("NEXUS_FIXTURE_CHECKPOINT")
    if source and checkpoint:
        PROVIDER.set_source(source, checkpoint)

    server = ThreadingHTTPServer(("127.0.0.1", port), SidecarHandler)
    server.daemon_threads = True

    def _shutdown(_sig: int, _frame: Any) -> None:
        threading.Thread(target=server.shutdown, daemon=True).start()

    signal.signal(signal.SIGTERM, _shutdown)
    signal.signal(signal.SIGINT, _shutdown)

    print(f"PORT {server.server_address[1]}", flush=True)
    with contextlib.suppress(KeyboardInterrupt):
        server.serve_forever(poll_interval=0.25)
    return 0


if __name__ == "__main__":
    sys.exit(main())
