"""EP-020 M5 live-fire drivers (LF-006 / LF-007 / LF-024).

Each proof is a REAL Rust proof binary in
connectors/home-assistant/examples/ that drives the production adapter
against the REAL pinned Home Assistant container and prints evidence
JSON. The driver boots the container (M3 fixture), runs the binaries
with the fresh OAuth token, and asserts every evidence field. No
hard-coded PASS: the assertion binds to what the real instance
actually did.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
assert (ROOT / "AGENTS.md").is_file(), f"ROOT misresolved: {ROOT}"

sys.path.insert(0, str(ROOT / "infra/home-assistant/tests"))
from test_ep020_integration_home_assistant import (  # noqa: E402
    BASE,
    NAME,
    HaFixture,
)

FIXTURE = HaFixture()


def setup_module():
    FIXTURE.__enter__()


def teardown_module():
    FIXTURE.__exit__()


def run_proof(proof: str, extra_env: dict | None = None) -> dict:
    exe = ROOT / "target/debug/examples" / proof
    assert exe.is_file(), f"proof binary missing: {exe} (run cargo build --examples)"
    env = dict(os.environ)
    env["NEXUS_HA_BASE"] = BASE
    env["NEXUS_HA_TOKEN"] = FIXTURE.token
    if extra_env:
        env.update(extra_env)
    out = subprocess.run([str(exe)], capture_output=True, text=True, env=env, timeout=300)
    assert out.returncode == 0, f"{proof} failed:\n{out.stdout}\n{out.stderr}"
    line = out.stdout.strip().splitlines()[-1]
    return json.loads(line)


def _ws_seen_state_changed(ws, target: str, timeout: float = 15) -> bool:
    """Consume buffered events from an EXISTING subscription and report
    whether a real state_changed event for `target` was observed."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        evt = FIXTURE.ws_recv(ws)
        if evt is None:
            continue
        if evt.get("event", {}).get("event_type") != "state_changed":
            continue
        data = evt["event"]["data"]
        if data.get("entity_id") == target:
            return True
    return False


def test_ep020_livefire_lf006_deterministic_home_control():
    # The audit event must exist: subscribe BEFORE the proof runs and
    # consume the SAME subscription's buffered events afterward - the
    # exact-target state_changed occurred while this socket was live.
    ws = FIXTURE.ws_connect()
    ws.send(
        json.dumps(
            {
                "id": 1,
                "type": "subscribe_events",
                "event_type": "state_changed",
            }
        )
    )
    ack = json.loads(ws.recv())
    assert ack.get("success") is True, ack
    try:
        ev = run_proof("lf006_proof")
        audit_seen = _ws_seen_state_changed(ws, "light.nexus_test_light")
    finally:
        ws.close()
    assert ev["proof"] == "LF-006"
    assert ev["auth"] is True, "real authentication failed"
    assert ev["discovered"] is True, "fixture light not discovered"
    assert ev["entity"] == "light.nexus_test_light"
    assert ev["category"] == "LIGHT"
    assert ev["fast_path"] == "EXECUTE_LOCALLY", "fast path must be local"
    assert ev["no_model_call"] is True, "model surface must not exist"
    assert ev["receipt_state"] == "SUBMITTED", "COMMAND ACCEPTED != VERIFIED"
    assert ev["verification"] == "VERIFIED", "exact-target verification failed"
    assert ev["target_entity"] == "light.nexus_test_light"
    assert ev["provider_service"] == "light/turn_on"
    assert audit_seen, "no state_changed audit event for the exact target"


def test_ep020_livefire_lf007_conditional_home_workflow():
    ev_create = run_proof("lf007_proof", {"PHASE": "create"})
    assert ev_create["proof"] == "LF-007" and ev_create["phase"] == "create"
    assert ev_create["created"] is True
    assert ev_create["automation_true"] == "on"
    assert ev_create["automation_false"] == "on"

    # Persistence: restart the REAL container; the automations must
    # survive in HA's durable registry. The OAuth session must be
    # persisted by HA BEFORE the restart (same fresh-token race as
    # lf024) - settle first, then let the auth store load after.
    time.sleep(30)
    subprocess.run(["/usr/bin/docker", "restart", NAME], check=True, capture_output=True)
    FIXTURE._wait_ready(300)
    time.sleep(10)
    FIXTURE._wait_entities(180)

    ev_persist = run_proof("lf007_proof", {"PHASE": "persist"})
    assert ev_persist["phase"] == "persist" and ev_persist["persisted"] is True
    assert ev_persist["automation_true"] == "on"
    assert ev_persist["automation_false"] == "on"

    ev_exec = run_proof("lf007_proof", {"PHASE": "exec"})
    assert ev_exec["phase"] == "exec"
    assert ev_exec["conditional_execution"] is True, (
        "action did not run when the condition was true"
    )
    assert ev_exec["conditional_cancellation"] is True, (
        "action ran when the condition was false (cancellation failed)"
    )
    assert ev_exec["switch2_after_cancel"] == "on"


def test_ep020_livefire_lf024_offline_degraded_operation():
    # The proof stops/starts the container itself. The OAuth access
    # token's session must be persisted by HA BEFORE the immediate
    # stop (otherwise the restart rejects the token and HA's http.ban
    # makes it permanent). Let the freshly booted instance settle.
    time.sleep(30)
    ev = run_proof("lf024_proof")
    assert ev["proof"] == "LF-024"
    assert ev["execute_offline_fail_closed"] is True, (
        "command succeeded while the provider was offline"
    )
    assert ev["reconnect_offline_code"] != "INTERNAL"
    assert ev["queued"] == 1, "offline queue did not retain the command"
    assert ev["duplicate_conflict"] is True, "duplicate offline intent must conflict"
    assert ev["offline_fast_path"] == "EXECUTE_LOCALLY", "low-risk local capability lost offline"
    assert ev["no_model_call"] is True
    assert ev["drained"] is True, "queued command not drained on reconnect"
    assert ev["queued_verified"] == "VERIFIED", "queued command not verified after synchronization"
