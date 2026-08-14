"""EP-011 M3 observability proof (directive R).

The fixture sidecar emits redacted structured telemetry to stderr:
connector ID fingerprint, capability ID, capability class, tenant
fingerprint, transport, latency, result/error class, and correlation
ID. Raw credentials, authorization headers, secret values, provider
tokens, webhook secrets, and complete sensitive payloads must never
appear.
"""

from __future__ import annotations

import json
import subprocess
import sys
import urllib.error
import urllib.request

from .conftest import REPO_ROOT, SIDECAR, make_context


def ep011_integration_observability_telemetry_redacted(tmp_path):
    stderr_path = tmp_path / "sidecar-stderr.log"
    with open(stderr_path, "w") as stderr_fh:
        proc = subprocess.Popen(
            [sys.executable, str(SIDECAR)],
            cwd=str(REPO_ROOT),
            stdout=subprocess.PIPE,
            stderr=stderr_fh,
            text=True,
        )
        try:
            line = proc.stdout.readline()  # type: ignore[union-attr]
            port = int(line.strip().split()[1])

            body = json.dumps(
                {
                    "capability_id": "fixture.contacts.query",
                    "context": make_context(),
                    "input": {},
                }
            ).encode("utf-8")
            req = urllib.request.Request(
                f"http://127.0.0.1:{port}/v1/query",
                data=body,
                method="POST",
                headers={
                    "Content-Type": "application/json",
                    "X-Nexus-Protocol-Version": "1",
                },
            )
            with urllib.request.urlopen(req, timeout=5):
                pass

            # Force one denial so error_class telemetry is emitted too.
            req_err = urllib.request.Request(
                f"http://127.0.0.1:{port}/v1/query",
                data=json.dumps(
                    {
                        "capability_id": "fixture.does.not.exist",
                        "context": make_context(),
                        "input": {},
                    }
                ).encode("utf-8"),
                method="POST",
                headers={
                    "Content-Type": "application/json",
                    "X-Nexus-Protocol-Version": "1",
                },
            )
            try:
                with urllib.request.urlopen(req_err, timeout=5):
                    pass
            except urllib.error.HTTPError:
                pass
        finally:
            proc.terminate()
            proc.wait(timeout=5)

    lines = stderr_path.read_text().splitlines()
    telemetry = [
        json.loads(line) for line in lines if line.startswith("{") and "connector_id" in line
    ]
    assert len(telemetry) >= 2, "sidecar must emit structured telemetry"

    for entry in telemetry:
        # Structured fields present (directive R).
        assert "connector_id" in entry
        assert "connector_id_fingerprint" in entry
        assert "capability_id" in entry
        assert "class" in entry
        assert "tenant_fingerprint" in entry
        assert "transport" in entry
        assert "latency_ms" in entry
        assert "result_class" in entry
        assert "correlation_id" in entry
        # Tenant appears only as a fingerprint, never the raw id.
        assert "018f0f6f-9c1e-7b6e-8000-000000000003" not in json.dumps(entry)

    text = stderr_path.read_text()
    # Never secrets, never raw payloads, never auth headers.
    assert "fixture-secret-value" not in text
    assert "user:alice" not in text
    assert "X-Nexus" not in text
    # Never complete sensitive payloads.
    assert '"contacts"' not in text
