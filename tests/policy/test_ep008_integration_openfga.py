"""EP-008 M3 integration tests: relationship authorization through REAL OpenFGA.

Test names begin with ep008_integration_ per the EP-008 milestone
contract. Uses the pinned OpenFGA 1.18.1 image (VERSIONS.lock.yaml /
COMPONENT_REGISTRY.yaml) in a real ephemeral container - never an
in-memory substitute (TESTING.md reality rule).

CANONICAL NEXUS-TO-OPENFGA MAPPING (verified live against the pinned
container; recorded in the ExecPlan Decision Log):
- principal -> `user:<principal_id>` (all canonical principal types map
  to the OpenFGA `user` type; the canonical actor type is recorded in
  telemetry, not in the relationship model);
- object -> `<object_type>:<tenant_id>|<object_id>` (tenant embedded in
  the object id; an identically named object in another tenant is a
  DIFFERENT OpenFGA object - no cross-tenant wildcarding; colon
  separator is rejected by the provider, pipe is accepted);
- relation -> canonical relation name (owner/member/admin/operator/
  viewer/editor/delegated/actor);
- explicit deny is NOT modeled: absence of a relationship is the denial
  (fail closed); the model contains no typed wildcards.

RESPONSIBILITY BOUNDARY: OpenFGA proves relationship authorization
only. Contextual risk, time, auth strength, and approval are NOT
encoded here - they belong to OPA / nexus-policy / action-gateway
(directive B). The model has no wildcards and no contextual state.

REALITY RULE: every check/write/delete below is a real HTTP call to the
real pinned OpenFGA container via stdlib urllib (no HTTP library in the
frozen test env; EP-007 precedent). No pass result is pre-baked.

GATEWAY COMPOSITION (directive E): the ep008_integration_gateway_*
tests build the real Rust probe binary
(infra/openfga/examples/gateway_probe.rs) once, then run the REAL M2
DeterministicGateway + REAL OpenFGA adapter against the REAL container.

EVIDENCE REQUIREMENTS covered by this suite:
- owner_can_admin_household
- member_cannot_admin_household
- business_admin_can_manage_business_resource (transitive userset)
- unrelated_principal_denied
- device_operator_scope_is_bounded
- delegation_is_exact
- tuple_revocation_takes_effect (no stale local cache)
- wrong_store_or_model_fails_closed
- malformed_relation_request_fails_closed
- provider_unavailable_fails_closed (container killed)
- tenant isolation (tenant A allow, tenant B deny, same object suffix)
- gateway composition: relationship deny stops the gateway; valid
  relationship path continues to the next stage
- image is pinned by digest
- explicit teardown with zero orphans (directive I)
"""

from __future__ import annotations

import contextlib
import json
import secrets
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

IMAGE = "openfga/openfga@sha256:ec73e86c629f7c7b290cde0cf52bcea7c3e0315f30f65386fe4df532f4b83deb"
IMAGE_TAG = "openfga/openfga:v1.18.1-amd64"
ROOT = Path(__file__).resolve().parents[2]
DOCKER = "/usr/bin/docker"
CARGO = "/root/.cargo/bin/cargo"
PROBE_BIN = ROOT / "target" / "debug" / "examples" / "gateway_probe"

TENANT_A = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"
TENANT_B = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"

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


# --------------------------------------------------------------------------
# Container harness (EP-007 precedent: unique name, no persisted creds)
# --------------------------------------------------------------------------


def _cleanup_container(name: str) -> None:
    subprocess.run([DOCKER, "rm", "-f", name], capture_output=True, text=True, check=False)


def _start_openfga() -> tuple[str, int]:
    """Start a REAL pinned OpenFGA container; return (name, http port)."""
    name = f"nexus-ep008-{secrets.token_hex(4)}"
    subprocess.run(
        [
            DOCKER,
            "run",
            "-d",
            "--name",
            name,
            "-p",
            "127.0.0.1::8080",
            IMAGE,
            "run",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    try:
        port = _wait_for_openfga(name)
    except Exception:
        _cleanup_container(name)
        raise
    return name, port


def _wait_for_openfga(name: str, timeout: float = 60.0) -> int:
    deadline = time.time() + timeout
    last = ""
    while time.time() < deadline:
        out = subprocess.run(
            [DOCKER, "port", name, "8080/tcp"],
            capture_output=True,
            text=True,
            check=False,
        )
        line = out.stdout.strip()
        if line:
            port = int(line.rsplit(":", 1)[-1])
            try:
                status, body = _http("GET", f"http://127.0.0.1:{port}/healthz", timeout=3)
                if status == 200 and body.get("status") == "SERVING":
                    return port
            except AssertionError:
                # Container still starting; socket reset is expected.
                pass
        last = line or out.stderr.strip()
        time.sleep(1)
    raise AssertionError(f"OpenFGA {name} not ready; last port line: {last!r}")


@contextlib.contextmanager
def _openfga():
    name, port = _start_openfga()
    try:
        yield name, port
    finally:
        _cleanup_container(name)


# --------------------------------------------------------------------------
# OpenFGA bootstrap via REAL APIs (directive C)
# --------------------------------------------------------------------------


def _create_store(port: int, name: str) -> str:
    status, body = _http("POST", f"http://127.0.0.1:{port}/stores", {"name": name})
    assert status == 201, f"create store: {status} {body}"
    return body["id"]


def _create_model(port: int, store_id: str) -> str:
    model = {
        "schema_version": "1.1",
        "type_definitions": [
            {"type": "user"},
            {
                "type": "household",
                "relations": {
                    "owner": {"this": {}},
                    "member": {"this": {}},
                    "admin": {"computedUserset": {"relation": "owner"}},
                },
                "metadata": {
                    "relations": {
                        "owner": {"directly_related_user_types": [{"type": "user"}]},
                        "member": {"directly_related_user_types": [{"type": "user"}]},
                    }
                },
            },
            {
                "type": "business",
                "relations": {"admin": {"this": {}}, "member": {"this": {}}},
                "metadata": {
                    "relations": {
                        "admin": {"directly_related_user_types": [{"type": "user"}]},
                        "member": {"directly_related_user_types": [{"type": "user"}]},
                    }
                },
            },
            {
                "type": "device",
                "relations": {"operator": {"this": {}}},
                "metadata": {
                    "relations": {"operator": {"directly_related_user_types": [{"type": "user"}]}}
                },
            },
            {
                "type": "resource",
                "relations": {
                    "viewer": {"this": {}},
                    "editor": {"this": {}},
                    "owner": {"this": {}},
                },
                "metadata": {
                    "relations": {
                        "viewer": {
                            "directly_related_user_types": [
                                {"type": "user"},
                                {"type": "business", "relation": "admin"},
                            ]
                        },
                        "editor": {
                            "directly_related_user_types": [
                                {"type": "user"},
                                {"type": "business", "relation": "admin"},
                            ]
                        },
                        "owner": {"directly_related_user_types": [{"type": "user"}]},
                    }
                },
            },
            {
                "type": "capability",
                "relations": {"delegated": {"this": {}}},
                "metadata": {
                    "relations": {"delegated": {"directly_related_user_types": [{"type": "user"}]}}
                },
            },
            {
                "type": "action",
                "relations": {"actor": {"this": {}}},
                "metadata": {
                    "relations": {"actor": {"directly_related_user_types": [{"type": "user"}]}}
                },
            },
        ],
    }
    status, body = _http(
        "POST",
        f"http://127.0.0.1:{port}/stores/{store_id}/authorization-models",
        model,
    )
    assert status == 201, f"create model: {status} {body}"
    return body["authorization_model_id"]


def _write_tuples(port: int, store_id: str, tuples: list[dict]) -> None:
    status, body = _http(
        "POST",
        f"http://127.0.0.1:{port}/stores/{store_id}/write",
        {"writes": {"tuple_keys": tuples}},
    )
    assert status == 200, f"write tuples: {status} {body}"


def _delete_tuples(port: int, store_id: str, tuples: list[dict]) -> None:
    status, body = _http(
        "POST",
        f"http://127.0.0.1:{port}/stores/{store_id}/write",
        {"deletes": {"tuple_keys": tuples}},
    )
    assert status == 200, f"delete tuples: {status} {body}"


def _check(
    port: int,
    store_id: str,
    model_id: str,
    user: str,
    relation: str,
    obj: str,
    model_override: str | None = None,
) -> bool:
    body = {
        "tuple_key": {"user": user, "relation": relation, "object": obj},
        "authorization_model_id": model_override or model_id,
    }
    status, resp = _http("POST", f"http://127.0.0.1:{port}/stores/{store_id}/check", body)
    assert status == 200, f"check {user} {relation} {obj}: {status} {resp}"
    return bool(resp.get("allowed"))


# --------------------------------------------------------------------------
# Session fixture: one real container per suite, isolated stores per test
# --------------------------------------------------------------------------

_session = {}


@contextlib.contextmanager
def _suite_openfga():
    """One real container for the whole suite (unique name), explicit
    teardown in the finally; the zz teardown test proves zero orphans."""
    name, port = _start_openfga()
    try:
        yield name, port
    finally:
        _cleanup_container(name)


def _ensure_suite() -> tuple[str, int]:
    if "name" not in _session:
        cm = _suite_openfga()
        _session["name"], _session["port"] = cm.__enter__()
        _session["cm"] = cm
    return _session["name"], _session["port"]


def _suite_bootstrap(port: int) -> dict:
    """Bootstrap a FRESH store + model for one test (no shared state)."""
    store = _create_store(port, f"nexus-ep008-{secrets.token_hex(4)}")
    model = _create_model(port, store)
    return {"port": port, "store": store, "model": model}


def _ctx() -> dict:
    name, port = _ensure_suite()
    return _suite_bootstrap(port)


def ep008_integration_openfga_image_is_pinned() -> None:
    """The pinned digest image must be present locally (directive C)."""
    inspect = subprocess.run(
        [DOCKER, "image", "inspect", f"{IMAGE_TAG}@{IMAGE.split('@')[1]}"],
        capture_output=True,
        text=True,
    )
    assert inspect.returncode == 0, "pinned openfga 1.18.1 image not present locally"


# --------------------------------------------------------------------------
# 1. owner_can_admin_household
# --------------------------------------------------------------------------


def ep008_integration_owner_can_admin_household() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    _write_tuples(
        port,
        store,
        [{"user": "user:alice", "relation": "owner", "object": f"household:{TENANT_A}|h1"}],
    )
    assert _check(port, store, model, "user:alice", "owner", f"household:{TENANT_A}|h1")
    # owner -> admin is a computed userset (model-derived).
    assert _check(port, store, model, "user:alice", "admin", f"household:{TENANT_A}|h1")


# --------------------------------------------------------------------------
# 2. member_cannot_admin_household
# --------------------------------------------------------------------------


def ep008_integration_member_cannot_admin_household() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    _write_tuples(
        port,
        store,
        [{"user": "user:bob", "relation": "member", "object": f"household:{TENANT_A}|h1"}],
    )
    assert _check(port, store, model, "user:bob", "member", f"household:{TENANT_A}|h1")
    assert not _check(port, store, model, "user:bob", "admin", f"household:{TENANT_A}|h1")
    assert not _check(port, store, model, "user:bob", "owner", f"household:{TENANT_A}|h1")


# --------------------------------------------------------------------------
# 3. business_admin_can_manage_business_resource (transitive userset)
# --------------------------------------------------------------------------


def ep008_integration_business_admin_can_manage_business_resource() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    _write_tuples(
        port,
        store,
        [
            {"user": "user:ceo", "relation": "admin", "object": f"business:{TENANT_A}|b1"},
            # Model-derived: business admins are viewers/editors of the resource.
            {
                "user": f"business:{TENANT_A}|b1#admin",
                "relation": "viewer",
                "object": f"resource:{TENANT_A}|r1",
            },
            {
                "user": f"business:{TENANT_A}|b1#admin",
                "relation": "editor",
                "object": f"resource:{TENANT_A}|r1",
            },
        ],
    )
    assert _check(port, store, model, "user:ceo", "viewer", f"resource:{TENANT_A}|r1")
    assert _check(port, store, model, "user:ceo", "editor", f"resource:{TENANT_A}|r1")


# --------------------------------------------------------------------------
# 4. unrelated_principal_denied
# --------------------------------------------------------------------------


def ep008_integration_unrelated_principal_denied() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    _write_tuples(
        port,
        store,
        [{"user": "user:alice", "relation": "owner", "object": f"household:{TENANT_A}|h1"}],
    )
    assert not _check(port, store, model, "user:mallory", "owner", f"household:{TENANT_A}|h1")
    assert not _check(port, store, model, "user:mallory", "admin", f"household:{TENANT_A}|h1")


# --------------------------------------------------------------------------
# 5. device_operator_scope_is_bounded
# --------------------------------------------------------------------------


def ep008_integration_device_operator_scope_is_bounded() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    _write_tuples(
        port,
        store,
        [{"user": "user:tech", "relation": "operator", "object": f"device:{TENANT_A}|dA"}],
    )
    assert _check(port, store, model, "user:tech", "operator", f"device:{TENANT_A}|dA")
    # Same operator, DIFFERENT device -> deny (target binding is exact).
    assert not _check(port, store, model, "user:tech", "operator", f"device:{TENANT_A}|dB")


# --------------------------------------------------------------------------
# 6. delegation_is_exact
# --------------------------------------------------------------------------


def ep008_integration_delegation_is_exact() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    _write_tuples(
        port,
        store,
        [{"user": "user:carol", "relation": "delegated", "object": f"capability:{TENANT_A}|cap1"}],
    )
    # Delegated actor may perform ONLY the modeled relationship.
    assert _check(port, store, model, "user:carol", "delegated", f"capability:{TENANT_A}|cap1")
    # Delegation does NOT imply owner/admin anywhere.
    assert not _check(port, store, model, "user:carol", "owner", f"household:{TENANT_A}|h1")
    assert not _check(port, store, model, "user:carol", "admin", f"household:{TENANT_A}|h1")
    assert not _check(port, store, model, "user:carol", "operator", f"device:{TENANT_A}|dA")


# --------------------------------------------------------------------------
# 7. tuple_revocation_takes_effect (no stale local cache)
# --------------------------------------------------------------------------


def ep008_integration_tuple_revocation_takes_effect() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    t = {"user": "user:alice", "relation": "owner", "object": f"household:{TENANT_A}|h1"}
    _write_tuples(port, store, [t])
    assert _check(port, store, model, "user:alice", "owner", f"household:{TENANT_A}|h1")
    _delete_tuples(port, store, [t])
    # After removal, the check MUST be deny - no stale authorization cache.
    assert not _check(port, store, model, "user:alice", "owner", f"household:{TENANT_A}|h1")


# --------------------------------------------------------------------------
# 8. wrong_store_or_model_fails_closed
# --------------------------------------------------------------------------


def ep008_integration_wrong_store_or_model_fails_closed() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    _write_tuples(
        port,
        store,
        [{"user": "user:alice", "relation": "owner", "object": f"household:{TENANT_A}|h1"}],
    )
    # A model id that does not exist must NOT allow.
    status, resp = _http(
        "POST",
        f"http://127.0.0.1:{port}/stores/{store}/check",
        {
            "tuple_key": {
                "user": "user:alice",
                "relation": "owner",
                "object": f"household:{TENANT_A}|h1",
            },
            "authorization_model_id": "01BADBADBADBADBADBADBADB",
        },
    )
    assert status == 400, f"wrong model: expected 400, got {status} {resp}"
    assert "authorizationmodel" in json.dumps(resp).lower(), resp

    # A store that does not exist must NOT allow. The provider rejects
    # with validation_error (StoreId pattern) or 404 for unknown routes;
    # either way it is a typed denial, never an allow.
    status, resp = _http(
        "POST",
        f"http://127.0.0.1:{port}/stores/01NOSTORENOSTORENOSTORE/check",
        {
            "tuple_key": {
                "user": "user:alice",
                "relation": "owner",
                "object": f"household:{TENANT_A}|h1",
            },
            "authorization_model_id": model,
        },
    )
    assert status in (400, 404), f"wrong store: expected 400/404, got {status} {resp}"
    assert "store" in json.dumps(resp).lower(), resp


# --------------------------------------------------------------------------
# 9. malformed_relation_request_fails_closed
# --------------------------------------------------------------------------


def ep008_integration_malformed_relation_request_fails_closed() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    # Invalid user shape (no type prefix).
    status, resp = _http(
        "POST",
        f"http://127.0.0.1:{port}/stores/{store}/check",
        {
            "tuple_key": {
                "user": "not-a-user",
                "relation": "owner",
                "object": f"household:{TENANT_A}|h1",
            },
            "authorization_model_id": model,
        },
    )
    assert status == 400, f"malformed user: expected 400, got {status} {resp}"

    # Unknown relation must NOT allow.
    status, resp = _http(
        "POST",
        f"http://127.0.0.1:{port}/stores/{store}/check",
        {
            "tuple_key": {
                "user": "user:alice",
                "relation": "nonexistent",
                "object": f"household:{TENANT_A}|h1",
            },
            "authorization_model_id": model,
        },
    )
    assert status == 400, f"unknown relation: expected 400, got {status} {resp}"

    # Object with colon in the id is rejected by the provider (the
    # adapter's tenant-pipe encoding is the canonical form).
    status, resp = _http(
        "POST",
        f"http://127.0.0.1:{port}/stores/{store}/check",
        {
            "tuple_key": {
                "user": "user:alice",
                "relation": "owner",
                "object": f"household:{TENANT_A}:h1",
            },
            "authorization_model_id": model,
        },
    )
    assert status == 400, f"colon object: expected 400, got {status} {resp}"


# --------------------------------------------------------------------------
# 10. provider_unavailable_fails_closed (kill the real container)
# --------------------------------------------------------------------------


def ep008_integration_provider_unavailable_fails_closed() -> None:
    """Kill the real container; the REAL adapter must return a typed
    unavailable error and the gateway must not progress to ALLOW."""
    name, port = _start_openfga()
    try:
        store = _create_store(port, "nexus-ep008-unavail")
        model = _create_model(port, store)
        _write_tuples(
            port,
            store,
            [{"user": "user:alice", "relation": "owner", "object": f"household:{TENANT_A}|h1"}],
        )
        # Prove the path ALLOWS while the provider is up.
        ctx = {"port": port, "store": store, "model": model}
        data = _probe_common()
        _write_tuples(
            port,
            store,
            [
                {
                    "user": "user:0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
                    "relation": "actor",
                    "object": f"action:{TENANT_A}|0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
                }
            ],
        )
        out = _probe(ctx, data)
        assert out["decision"] == "ALLOWED", f"baseline should allow: {out}"

        # Kill the container (real failure mechanism).
        subprocess.run([DOCKER, "kill", name], check=True, capture_output=True, text=True)
        time.sleep(1)

        # The REAL adapter + gateway must NOT produce ALLOW: the check
        # fails with a typed unavailable error.
        out = _probe(ctx, data)
        assert out["decision"] != "ALLOWED", f"killed provider must fail closed: {out}"
        assert out["decision"] == "ERROR", out
        assert "unavailable" in (out.get("error") or "").lower(), out
    finally:
        _cleanup_container(name)


# --------------------------------------------------------------------------
# F. tenant isolation
# --------------------------------------------------------------------------


def ep008_integration_tenant_isolation_no_cross_tenant_wildcarding() -> None:
    ctx = _ctx()
    port, store, model = ctx["port"], ctx["store"], ctx["model"]
    # Tenant A: alice owns object with suffix ...0001.
    _write_tuples(
        port,
        store,
        [{"user": "user:alice", "relation": "owner", "object": f"household:{TENANT_A}|0001"}],
    )
    # Tenant A allow.
    assert _check(port, store, model, "user:alice", "owner", f"household:{TENANT_A}|0001")
    # Same principal, tenant B object with the SAME suffix -> deny.
    assert not _check(port, store, model, "user:alice", "owner", f"household:{TENANT_B}|0001")
    # Explicitly unrelated tenant B owner does not leak into tenant A.
    _write_tuples(
        port,
        store,
        [{"user": "user:bob", "relation": "owner", "object": f"household:{TENANT_B}|0002"}],
    )
    assert not _check(port, store, model, "user:bob", "owner", f"household:{TENANT_A}|0002")


# --------------------------------------------------------------------------
# E. gateway composition through the REAL M2 gateway + adapter
# --------------------------------------------------------------------------

_probe_built = False


def _build_probe() -> None:
    global _probe_built
    if _probe_built:
        return
    subprocess.run(
        [CARGO, "build", "--example", "gateway_probe", "--locked", "-p", "nexus-openfga"],
        cwd=str(ROOT),
        check=True,
        capture_output=True,
        text=True,
        timeout=600,
    )
    _probe_built = True


def _probe(ctx: dict, input_data: dict) -> dict:
    _build_probe()
    payload = {
        "base_url": f"http://127.0.0.1:{ctx['port']}",
        "store_id": ctx["store"],
        "model_id": ctx["model"],
        **input_data,
    }
    proc = subprocess.run(
        [str(PROBE_BIN)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert proc.returncode == 0, f"probe failed: {proc.stderr}"
    return json.loads(proc.stdout.strip())


def _probe_common(action: str = "task:complete") -> dict:
    now = 1_700_000_000
    return {
        "policy_version": "probe-v1",
        "request": {
            "request_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
            "correlation": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02",
            "tenant_id": TENANT_A,
            "action_digest": "digest-abc",
            "action": action,
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


def ep008_integration_gateway_relationship_deny_stops() -> None:
    """No `actor` tuple in OpenFGA -> the real gateway must stop at the
    relationship stage with a RELATIONSHIP denial (no policy/risk/
    approval/capability stage can produce an allow)."""
    ctx = _ctx()
    data = _probe_common()
    out = _probe(ctx, data)
    assert out["decision"] == "DENIED", out
    assert "RELATIONSHIP" in (out.get("reason") or ""), out
    assert out["risk"] == "R0", out  # risk floor never reached


def ep008_integration_gateway_valid_relationship_continues_to_policy() -> None:
    """A valid `actor` relationship path must continue to the next
    authorization stage (policy), proving the gateway does NOT collapse
    into OpenFGA: the allow-all policy port is consulted and the
    capability stage is reached (R2 COMMAND, no approval needed)."""
    ctx = _ctx()
    # Grant the actor relationship in OpenFGA for the exact action target.
    _write_tuples(
        ctx["port"],
        ctx["store"],
        [
            {
                "user": "user:0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
                "relation": "actor",
                "object": f"action:{TENANT_A}|0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
            }
        ],
    )
    data = _probe_common()
    out = _probe(ctx, data)
    # Relationship passes; COMMAND -> R2 -> no approval; grant covers the
    # action and target -> ALLOW.
    assert out["decision"] == "ALLOWED", out
    assert out["risk"] == "R2", out


def ep008_integration_gateway_relationship_deny_no_receipt_success() -> None:
    """Directive E: relationship deny -> gateway stops -> no action
    receipt marked successful. The probe returns DENIED (never a
    successful receipt path); assert the denial reason is RELATIONSHIP."""
    ctx = _ctx()
    data = _probe_common()
    out = _probe(ctx, data)
    assert out["decision"] == "DENIED", out
    assert out["reason"] and "RELATIONSHIP" in out["reason"], out
    # No success marker exists in the outcome: the gateway did not
    # reach the capability/allow stage.
    assert "grant" not in out, out


# --------------------------------------------------------------------------
# I. teardown invariant: zero orphans after the suite
# --------------------------------------------------------------------------


def ep008_integration_container_cleanup_leaves_no_orphans() -> None:
    """After the suite, prove ZERO EP-008 containers/networks remain.
    Runs last (zz ordering); explicit cleanup, failures surface here."""
    # The suite container is still running at this point; the teardown
    # of _session (finalizer registered on first use) runs at session
    # end. This test asserts no OTHER suite containers leaked and then
    # forces explicit cleanup now.
    leftovers = subprocess.run(
        [DOCKER, "ps", "-a", "--filter", "name=nexus-ep008", "--format", "{{.Names}}"],
        capture_output=True,
        text=True,
    ).stdout.split()
    # Allow exactly the live suite container (removed below); anything
    # else is an orphan.
    live = {_session.get("name")} if _session.get("name") else set()
    orphans = [c for c in leftovers if c not in live]
    assert not orphans, f"orphan EP-008 containers: {orphans}"

    # Explicitly tear the suite container down now (directive I primary
    # cleanup path) and re-prove zero leftovers.
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
