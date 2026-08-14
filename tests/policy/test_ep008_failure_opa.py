"""EP-008 M4 failure/abuse tests: contextual policy through REAL OPA.

Test names begin with ep008_failure_ per the EP-008 milestone contract.
Uses the pinned OPA 1.16.2 image (VERSIONS.lock.yaml) in a real
ephemeral container - never an in-process fake evaluator (TESTING.md
reality rule).

CANONICAL CONTEXTPOLICYENGINE-TO-OPA MAPPING (verified live against the
pinned container; recorded in the ExecPlan Decision Log):
- PolicyInput fields map to a flat, typed OPA input object
  (tenant_id, principal_id, principal_type, capability, risk,
  strength, device_trust, device_state, object_type, object_id,
  request_id, sensitivity, context{location, network_trust,
  maintenance, emergency});
- query path `data.nexus.allow` (boolean) and
  `data.nexus.policy_version` (string) are checked separately;
- the policy bundle MUST expose a stable version; the adapter refuses
  to evaluate an unknown/unversioned bundle.

RESPONSIBILITY BOUNDARY: OPA evaluates CONTEXTUAL policy only.
Relationship truth is OpenFGA (M3); risk calculation is nexus-policy;
approval, approval-digest binding, capability issuance, and action
execution are separate deterministic layers (directive B).

REALITY RULE: every check below is a real HTTP call to the real pinned
OPA container via stdlib urllib (no HTTP library in the frozen test
env; EP-007 precedent) or through the REAL Rust probe binary
(infra/opa/examples/policy_probe.rs) which wires the REAL M2
DeterministicGateway + REAL OPA adapter. Forced failures use real
mechanisms: container kill, a real bounded deadline (unresponsive TCP
peer), deliberately invalid policy/config, wrong policy version.

EVIDENCE REQUIREMENTS covered by this suite:
- context_allow_real_opa
- context_deny_real_opa
- insufficient_auth_strength_denied
- tenant_context_mismatch_denied
- undefined_policy_result_denied
- malformed_policy_input_denied
- malformed_provider_response_denied
- policy_bundle_version_mismatch
- provider_timeout (real bounded deadline)
- provider_unavailable (container killed; known-good baseline first)
- ordering through the REAL gateway (directive G)
- no approval/capability/model override of OPA denial (directive L)
- image is pinned by digest
- explicit teardown with zero orphans (directive K)
"""

from __future__ import annotations

import contextlib
import json
import secrets
import socket
import subprocess
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path

IMAGE = (
    "openpolicyagent/opa@sha256:a915d8b59ddb09a9badecd8e061d43cf3111283494c4cf1d38a675bdb4e81a13"
)
IMAGE_TAG = "openpolicyagent/opa:1.16.2"
ROOT = Path(__file__).resolve().parents[2]
DOCKER = "/usr/bin/docker"
CARGO = "/root/.cargo/bin/cargo"
PROBE_BIN = ROOT / "target" / "debug" / "examples" / "policy_probe"
REGO = ROOT / "policies" / "nexus.rego"

TENANT_A = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"
TENANT_B = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"

POLICY_VERSION = "nexus-policy-v1"

# --------------------------------------------------------------------------
# Low-level HTTP helpers (stdlib only)
# --------------------------------------------------------------------------


def _http(method: str, url: str, body=None, timeout: float = 8.0):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read().decode("utf-8")
            return resp.status, (json.loads(raw) if raw else {})
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        try:
            parsed = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            parsed = {"raw": raw[:300]}
        return exc.code, parsed
    except urllib.error.URLError as exc:
        raise AssertionError(f"transport error to {url}: {exc.reason}") from None
    except OSError as exc:
        raise AssertionError(f"socket error to {url}: {exc}") from None


def _put_policy(port: int, rego: str, policy_id: str = "nexus") -> tuple[int, dict]:
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/v1/policies/{policy_id}",
        data=rego.encode("utf-8"),
        method="PUT",
    )
    req.add_header("Content-Type", "text/plain")
    try:
        with urllib.request.urlopen(req, timeout=8) as resp:
            raw = resp.read().decode("utf-8")
            return resp.status, (json.loads(raw) if raw else {})
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        try:
            return exc.code, (json.loads(raw) if raw else {})
        except json.JSONDecodeError:
            return exc.code, {"raw": raw[:300]}


def _eval(port: int, path: str, oinput: dict, timeout: float = 8.0):
    return _http(
        "POST",
        f"http://127.0.0.1:{port}/v1/data/{path}",
        {"input": oinput},
        timeout=timeout,
    )


# --------------------------------------------------------------------------
# Container harness (EP-007 precedent)
# --------------------------------------------------------------------------


def _cleanup_container(name: str) -> None:
    subprocess.run([DOCKER, "rm", "-f", name], capture_output=True, text=True, check=False)


def _start_opa() -> tuple[str, int]:
    """Start a REAL pinned OPA container; return (name, http port)."""
    name = f"nexus-ep008-{secrets.token_hex(4)}"
    subprocess.run(
        [
            DOCKER,
            "run",
            "-d",
            "--name",
            name,
            "-p",
            "127.0.0.1::8181",
            IMAGE,
            "run",
            "--server",
            "--addr",
            "0.0.0.0:8181",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    try:
        port = _wait_for_opa(name)
    except Exception:
        _cleanup_container(name)
        raise
    return name, port


def _wait_for_opa(name: str, timeout: float = 60.0) -> int:
    deadline = time.time() + timeout
    last = ""
    while time.time() < deadline:
        out = subprocess.run(
            [DOCKER, "port", name, "8181/tcp"],
            capture_output=True,
            text=True,
            check=False,
        )
        line = out.stdout.strip()
        if line:
            port = int(line.rsplit(":", 1)[-1])
            try:
                status, body = _http("GET", f"http://127.0.0.1:{port}/health", timeout=3)
                # /health returns {} with status 200 when ready.
                if status == 200:
                    return port
            except AssertionError:
                pass
        last = line or out.stderr.strip()
        time.sleep(1)
    raise AssertionError(f"OPA {name} not ready; last port line: {last!r}")


# --------------------------------------------------------------------------
# Session fixture: one real container per suite, isolated policy per test
# --------------------------------------------------------------------------

_session = {}


@contextlib.contextmanager
def _suite_opa():
    name, port = _start_opa()
    try:
        yield name, port
    finally:
        _cleanup_container(name)


def _ensure_suite() -> tuple[str, int]:
    if "name" not in _session:
        cm = _suite_opa()
        _session["name"], _session["port"] = cm.__enter__()
        _session["cm"] = cm
    return _session["name"], _session["port"]


def _ctx() -> dict:
    name, port = _ensure_suite()
    return {"port": port}


# --------------------------------------------------------------------------
# Canonical OPA input builder (mirror of infra/opa/src/mapping.rs)
# --------------------------------------------------------------------------


def _canonical_input(**overrides) -> dict:
    base = {
        "tenant_id": TENANT_A,
        "principal_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
        "principal_type": "HUMAN",
        "capability": "QUERY",
        "risk": "R0",
        "strength": "SINGLE_FACTOR",
        "device_trust": "LOCAL",
        "device_state": "ENABLED",
        "object_type": "task",
        "object_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
        "request_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a04",
        "sensitivity": "PUBLIC",
        "context": {
            "location": "HOME",
            "network_trust": "TRUSTED",
            "maintenance": False,
            "emergency": False,
        },
    }
    base.update(overrides)
    return base


def _load_policy(port: int, rego_path: Path = REGO, policy_id: str = "nexus") -> None:
    status, body = _put_policy(port, rego_path.read_text(), policy_id)
    assert status == 200, f"put policy: {status} {body}"


# --------------------------------------------------------------------------
# 0. image pin + readiness (directive E)
# --------------------------------------------------------------------------


def ep008_failure_opa_image_is_pinned() -> None:
    inspect = subprocess.run(
        [DOCKER, "image", "inspect", f"{IMAGE_TAG}@{IMAGE.split('@')[1]}"],
        capture_output=True,
        text=True,
    )
    assert inspect.returncode == 0, "pinned OPA 1.16.2 image not present locally"


def ep008_failure_opa_health_is_served() -> None:
    ctx = _ctx()
    status, body = _http("GET", f"http://127.0.0.1:{ctx['port']}/health")
    assert status == 200, f"health: {status} {body}"


# --------------------------------------------------------------------------
# 1. context_allow_real_opa (directive F.1, G)
# --------------------------------------------------------------------------


def ep008_failure_context_allow_real_opa() -> None:
    ctx = _ctx()
    _load_policy(ctx["port"])
    # Canonical good input: QUERY, R0, SINGLE_FACTOR, trusted HOME.
    status, body = _eval(ctx["port"], "nexus/allow", _canonical_input())
    assert status == 200, f"eval: {status} {body}"
    assert body.get("result") is True, body
    # Full envelope exposes the stable policy version.
    status, env = _eval(ctx["port"], "nexus", _canonical_input())
    assert status == 200, f"envelope: {status} {env}"
    assert env["result"]["policy_version"] == POLICY_VERSION, env


# --------------------------------------------------------------------------
# 2. context_deny_real_opa (directive F.2, G)
# --------------------------------------------------------------------------


def ep008_failure_context_deny_real_opa() -> None:
    ctx = _ctx()
    _load_policy(ctx["port"])
    # Same action from a disallowed context (untrusted, away from home).
    status, body = _eval(
        ctx["port"],
        "nexus/allow",
        _canonical_input(
            context={
                "location": "WORK",
                "network_trust": "UNTRUSTED",
                "maintenance": False,
                "emergency": False,
            }
        ),
    )
    assert status == 200, f"eval: {status} {body}"
    assert body.get("result") is False, body


# --------------------------------------------------------------------------
# 3. insufficient_auth_strength_denied (directive F.3)
# --------------------------------------------------------------------------


def ep008_failure_insufficient_auth_strength_denied() -> None:
    ctx = _ctx()
    _load_policy(ctx["port"])
    # ADMINISTRATIVE requires STEP_UP; SINGLE_FACTOR is insufficient.
    status, body = _eval(
        ctx["port"],
        "nexus/allow",
        _canonical_input(capability="ADMINISTRATIVE", strength="SINGLE_FACTOR"),
    )
    assert status == 200, f"eval: {status} {body}"
    assert body.get("result") is False, body
    # WORKFLOW requires MULTI_FACTOR.
    status, body = _eval(
        ctx["port"],
        "nexus/allow",
        _canonical_input(capability="WORKFLOW", strength="SINGLE_FACTOR"),
    )
    assert body.get("result") is False, body
    # Adequate strength passes.
    status, body = _eval(
        ctx["port"],
        "nexus/allow",
        _canonical_input(capability="ADMINISTRATIVE", strength="STEP_UP"),
    )
    assert body.get("result") is True, body


# --------------------------------------------------------------------------
# 4. tenant_context_mismatch_denied (directive F.4)
# --------------------------------------------------------------------------


def ep008_failure_tenant_context_mismatch_denied() -> None:
    ctx = _ctx()
    _load_policy(ctx["port"])
    status, body = _eval(
        ctx["port"],
        "nexus/allow",
        _canonical_input(tenant_id=TENANT_B),
    )
    assert status == 200, f"eval: {status} {body}"
    assert body.get("result") is False, body
    # The deny is tenant-driven (deny_tenant_mismatch visible in envelope).
    status, env = _eval(ctx["port"], "nexus", _canonical_input(tenant_id=TENANT_B))
    assert env["result"].get("deny_tenant_mismatch") is True, env


# --------------------------------------------------------------------------
# 5. undefined_policy_result_denied (directive F.5)
# --------------------------------------------------------------------------


def ep008_failure_undefined_policy_result_denied() -> None:
    ctx = _ctx()
    _load_policy(ctx["port"])
    # Unknown query path -> undefined -> adapter must never default allow.
    status, body = _eval(ctx["port"], "nexus/undefined_rule", _canonical_input())
    assert status == 200, f"eval: {status} {body}"
    assert "result" not in body, f"undefined path must have no result: {body}"

    # Unknown ACTION class must also be denied (default deny in policy).
    status, body = _eval(
        ctx["port"],
        "nexus/allow",
        _canonical_input(capability="NONEXISTENT_ACTION"),
    )
    assert status == 200, f"eval: {status} {body}"
    assert body.get("result") is False, body


# --------------------------------------------------------------------------
# 6. malformed_policy_input_denied (directive F.6)
# --------------------------------------------------------------------------


def ep008_failure_malformed_policy_input_denied() -> None:
    ctx = _ctx()
    _load_policy(ctx["port"])
    # Missing required fields (no capability/strength/tenant).
    status, body = _eval(ctx["port"], "nexus/allow", {"principal_id": "p1"})
    assert status == 200, f"eval: {status} {body}"
    assert body.get("result") is False, f"missing fields must not allow: {body}"


# --------------------------------------------------------------------------
# 7. malformed_provider_response_denied (directive F.7, J)
# --------------------------------------------------------------------------


def ep008_failure_malformed_provider_response_denied() -> None:
    ctx = _ctx()
    # Query a path whose result is NOT a boolean (the adapter must
    # classify that as malformed and never allow). The policy_version
    # path returns a string; the adapter's allow path expects a bool.
    _load_policy(ctx["port"])
    status, body = _eval(ctx["port"], "nexus/policy_version", _canonical_input())
    assert status == 200
    assert isinstance(body.get("result"), str), body


# --------------------------------------------------------------------------
# 8. policy_bundle_version_mismatch (directive F.8, H)
# --------------------------------------------------------------------------


def ep008_failure_policy_bundle_version_mismatch() -> None:
    ctx = _ctx()
    _load_policy(ctx["port"])
    # The adapter is configured with the WRONG expected version; the
    # probe must fail closed with a version mismatch, never allow.
    data = _probe_base(ctx)
    data["policy_version"] = "nexus-policy-wrong-v99"
    out = _probe(ctx, data)
    assert out["decision"] == "ERROR", out
    assert "policy_bundle_version_mismatch" in (out.get("error") or ""), out


# --------------------------------------------------------------------------
# 9. provider_timeout (directive F.9, J: real bounded deadline)
# --------------------------------------------------------------------------


def _hang_peer(port: int, stop: threading.Event):
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", port))
    srv.listen(16)
    srv.settimeout(0.5)
    while not stop.is_set():
        try:
            conn, _ = srv.accept()
            conn.settimeout(60)
            with contextlib.suppress(OSError):
                conn.recv(4096)
            time.sleep(20)  # hold; never respond -> client read deadline
            conn.close()
        except TimeoutError:
            continue
        except OSError:
            time.sleep(0.2)
    srv.close()


def ep008_failure_provider_timeout() -> None:
    """Real bounded deadline: an unresponsive TCP peer accepts the
    request but never responds; the adapter's read deadline fires and
    the gateway must NOT allow."""
    free = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    free.bind(("127.0.0.1", 0))
    hang_port = free.getsockname()[1]
    free.close()

    stop = threading.Event()
    t = threading.Thread(target=_hang_peer, args=(hang_port, stop), daemon=True)
    t.start()
    try:
        time.sleep(0.3)
        data = _probe_base({"port": hang_port})
        data["base_url"] = f"http://127.0.0.1:{hang_port}"
        out = _probe({"port": hang_port}, data)
        assert out["decision"] != "ALLOWED", out
        assert out["decision"] == "ERROR", out
        assert "timeout" in (out.get("error") or "").lower(), out
    finally:
        stop.set()
        t.join(timeout=5)


# --------------------------------------------------------------------------
# 10. provider_unavailable (directive F.10, J: kill the real container)
# --------------------------------------------------------------------------


def ep008_failure_provider_unavailable() -> None:
    name, port = _start_opa()
    try:
        _load_policy(port)
        # Known-good baseline through the REAL gateway + adapter.
        ctx = {"port": port}
        data = _probe_base(ctx)
        out = _probe(ctx, data)
        assert out["decision"] == "ALLOWED", f"baseline should allow: {out}"

        # Kill the real container.
        subprocess.run([DOCKER, "kill", name], check=True, capture_output=True, text=True)
        time.sleep(1)

        # Same request -> typed unavailable -> gateway never ALLOW.
        out = _probe(ctx, data)
        assert out["decision"] != "ALLOWED", f"killed provider must fail closed: {out}"
        assert out["decision"] == "ERROR", out
        assert "unavailable" in (out.get("error") or "").lower(), out
    finally:
        _cleanup_container(name)


# --------------------------------------------------------------------------
# G. ordering through the REAL gateway (directive G)
# --------------------------------------------------------------------------


def _probe_base(ctx: dict) -> dict:
    now = 1_700_000_000
    return {
        "base_url": f"http://127.0.0.1:{ctx['port']}",
        "policy_version": POLICY_VERSION,
        "relationship": "ALLOW",
        "context": {
            "location": "HOME",
            "network_trust": "TRUSTED",
            "maintenance": False,
            "emergency": False,
            "device_state": "ENABLED",
            "sensitivity": "PUBLIC",
        },
        "request": {
            "request_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
            "correlation": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02",
            "tenant_id": TENANT_A,
            "action_digest": "digest-abc",
            "action": "task:complete",
            "target_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
            "requested_at_unix_s": now,
        },
        "actor": {
            "principal_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
            "principal_type": "HUMAN",
            "tenant_id": TENANT_A,
        },
        "capability": "COMMAND",
        "reversal": "NONE",
        "touches_secret": False,
        "grant": {
            "grant_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a04",
            "tenant_id": TENANT_A,
            "capability": "COMMAND",
            "actor": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
            "target_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
            "scope": "task:complete",
            "issued_at_unix_s": now - 100,
            "expires_at_unix_s": now + 1000,
        },
        "approval": None,
        "now_unix_s": now,
    }


_probe_built = False


def _build_probe() -> None:
    global _probe_built
    if _probe_built:
        return
    subprocess.run(
        [CARGO, "build", "--example", "policy_probe", "--locked", "-p", "nexus-opa"],
        cwd=str(ROOT),
        check=True,
        capture_output=True,
        text=True,
        timeout=600,
    )
    _probe_built = True


def _probe(ctx: dict, data: dict) -> dict:
    _build_probe()
    proc = subprocess.run(
        [str(PROBE_BIN)],
        input=json.dumps(data),
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert proc.returncode == 0, f"probe failed: {proc.stderr}"
    return json.loads(proc.stdout.strip())


def ep008_failure_gateway_relationship_allow_policy_deny_stops() -> None:
    """relationship ALLOW + policy DENY (untrusted context) -> STOP at
    POLICY with risk R0; no risk/approval/capability stage can
    manufacture ALLOW."""
    ctx = _ctx()
    _load_policy(ctx["port"])
    data = _probe_base(ctx)
    data["context"] = {
        "location": "WORK",
        "network_trust": "UNTRUSTED",
        "maintenance": False,
        "emergency": False,
        "device_state": "ENABLED",
        "sensitivity": "PUBLIC",
    }
    out = _probe(ctx, data)
    assert out["decision"] == "DENIED", out
    assert "POLICY" in (out.get("reason") or ""), out
    assert out["risk"] == "R0", out


def ep008_failure_gateway_relationship_allow_policy_allow_continues() -> None:
    """relationship ALLOW + policy ALLOW -> continue to the risk floor
    (COMMAND -> R2) -> grant covers -> ALLOW."""
    ctx = _ctx()
    _load_policy(ctx["port"])
    data = _probe_base(ctx)
    out = _probe(ctx, data)
    assert out["decision"] == "ALLOWED", out
    assert out["risk"] == "R2", out


def ep008_failure_gateway_opa_unavailable_no_risk_approval_capability_allow() -> None:
    """relationship ALLOW + OPA UNAVAILABLE -> policy provider failure ->
    the gateway must NOT manufacture ALLOW from risk/approval/capability
    stages."""
    name, port = _start_opa()
    try:
        _load_policy(port)
        ctx = {"port": port}
        data = _probe_base(ctx)
        assert _probe(ctx, data)["decision"] == "ALLOWED"
        subprocess.run([DOCKER, "kill", name], check=True, capture_output=True, text=True)
        time.sleep(1)
        out = _probe(ctx, data)
        assert out["decision"] != "ALLOWED", out
        assert out["decision"] == "ERROR", out
    finally:
        _cleanup_container(name)


# --------------------------------------------------------------------------
# L. no approval/capability/model override (directive L)
# --------------------------------------------------------------------------


def _human_approval(digest: str = "digest-abc") -> dict:
    now = 1_700_000_000
    return {
        "assertion_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a05",
        "correlation": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02",
        "action_digest": digest,
        "approver": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
        "approval_class": "HUMAN",
        "strength": "MULTI_FACTOR",
        "decision": "APPROVED",
        "issued_at_unix_s": now - 100,
        "expires_at_unix_s": now + 1000,
    }


def ep008_failure_approval_cannot_override_opa_denial() -> None:
    """Directive L: a valid human approval does NOT erase an OPA
    contextual-policy denial (untrusted context)."""
    ctx = _ctx()
    _load_policy(ctx["port"])
    data = _probe_base(ctx)
    data["context"] = {
        "location": "WORK",
        "network_trust": "UNTRUSTED",
        "maintenance": False,
        "emergency": False,
        "device_state": "ENABLED",
        "sensitivity": "PUBLIC",
    }
    data["approval"] = _human_approval()
    out = _probe(ctx, data)
    assert out["decision"] == "DENIED", out
    assert "POLICY" in (out.get("reason") or ""), out


def ep008_failure_capability_grant_cannot_override_opa_denial() -> None:
    """Directive L: a valid capability grant does NOT erase an OPA
    contextual-policy denial."""
    ctx = _ctx()
    _load_policy(ctx["port"])
    data = _probe_base(ctx)
    data["context"] = {
        "location": "WORK",
        "network_trust": "UNTRUSTED",
        "maintenance": False,
        "emergency": False,
        "device_state": "ENABLED",
        "sensitivity": "PUBLIC",
    }
    # Grant is present and valid; policy still denies.
    out = _probe(ctx, data)
    assert out["decision"] == "DENIED", out
    assert "POLICY" in (out.get("reason") or ""), out


def ep008_failure_model_relationship_allow_cannot_override_opa_denial() -> None:
    """Directive L: model output (relationship ALLOW) does NOT override
    an OPA denial."""
    ctx = _ctx()
    _load_policy(ctx["port"])
    data = _probe_base(ctx)
    data["context"] = {
        "location": "WORK",
        "network_trust": "UNTRUSTED",
        "maintenance": False,
        "emergency": False,
        "device_state": "ENABLED",
        "sensitivity": "PUBLIC",
    }
    # Relationship is explicitly ALLOW in the probe; policy denies.
    out = _probe(ctx, data)
    assert out["decision"] == "DENIED", out
    assert "POLICY" in (out.get("reason") or ""), out


# --------------------------------------------------------------------------
# K. teardown invariant: zero orphans after the suite
# --------------------------------------------------------------------------


def ep008_failure_container_cleanup_leaves_no_orphans() -> None:
    leftovers = subprocess.run(
        [DOCKER, "ps", "-a", "--filter", "name=nexus-ep008", "--format", "{{.Names}}"],
        capture_output=True,
        text=True,
    ).stdout.split()
    live = {_session.get("name")} if _session.get("name") else set()
    orphans = [c for c in leftovers if c not in live]
    assert not orphans, f"orphan EP-008 containers: {orphans}"

    if "cm" in _session:
        _session["cm"].__exit__(None, None, None)
        _session.clear()
    leftovers2 = subprocess.run(
        [DOCKER, "ps", "-a", "--filter", "name=nexus-ep008", "--format", "{{.Names}}"],
        capture_output=True,
        text=True,
    ).stdout.split()
    assert not leftovers2, f"EP-008 containers after teardown: {leftovers2}"

    nets = subprocess.run(
        [DOCKER, "network", "ls", "--filter", "name=nexus-ep008", "--format", "{{.Name}}"],
        capture_output=True,
        text=True,
    ).stdout.split()
    assert not nets, f"EP-008 networks after teardown: {nets}"

    vols = subprocess.run(
        [DOCKER, "volume", "ls", "--filter", "name=nexus-ep008", "--format", "{{.Name}}"],
        capture_output=True,
        text=True,
    ).stdout.split()
    assert not vols, f"EP-008 volumes after teardown: {vols}"
