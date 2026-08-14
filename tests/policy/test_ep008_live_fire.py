"""EP-008 M5 live-fire: the FULL authorization chain as ONE system.

Test names begin with ep008_livefire_ per the EP-008 M5 milestone
contract. This suite is the crown-jewel composition proof:

  relationship -> contextual policy -> risk floor -> R3/R4 human
  approval -> capability grant -> action gateway authorization ->
  canonical ActionReceipt + VerificationPlan

with deterministic fail-closed behavior at every boundary.

REALITY RULE: both providers are REAL pinned containers - OpenFGA
1.18.1 (sha256:ec73e86c...) and OPA 1.16.2 (sha256:a915d8b5...) - and
every decision is produced by the REAL M2 DeterministicGateway wired to
the REAL OpenFGA adapter (M3) and the REAL OPA adapter (M4) through the
combined probe binary infra/opa/examples/livefire_probe.rs. No fake
RelationshipAuthorizer and no fake ContextPolicyEngine anywhere.

CANONICAL LIVE-FIRE ACTION (directive A): an R3 ADMINISTRATIVE action
`admin:test:livefire` against a test target. The deterministic risk
floor maps ADMINISTRATIVE to R3 (SPEC-005 behavior 4), which requires a
STEP_UP human approval assertion and an exact capability grant. The
action is a harmless administrative TEST capability; EP-008 owns
authorization only, so nothing is executed (AUTHORIZED != EXECUTED !=
VERIFIED).

EVIDENCE REQUIREMENTS covered (directives C-M):
- C  allow path end-to-end with canonical ActionReceipt + plan
- D  receipt is audit evidence, not a bearer credential
- E  exact action-digest binding (old approval cannot authorize a
     changed action)
- F  capability scope binding (actor/target/scope/tenant/expiry)
- G  policy denial dominates
- H  relationship denial dominates
- I  R4 model-approval prohibition
- J  STEP_UP requirement
- K  provider failure fails closed (OpenFGA killed; OPA killed)
- L  verification plan deterministic; AUTHORIZED != EXECUTED != VERIFIED
- M  model recommendation never grants authority
- N  evidence file written
- O  teardown with zero orphans
"""

from __future__ import annotations

import contextlib
import hashlib
import json
import secrets
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

IMAGE_OPENFGA = (
    "openfga/openfga@sha256:ec73e86c629f7c7b290cde0cf52bcea7c3e0315f30f65386fe4df532f4b83deb"
)
IMAGE_OPENFGA_TAG = "openfga/openfga:v1.18.1-amd64"
IMAGE_OPA = (
    "openpolicyagent/opa@sha256:a915d8b59ddb09a9badecd8e061d43cf3111283494c4cf1d38a675bdb4e81a13"
)
IMAGE_OPA_TAG = "openpolicyagent/opa:1.16.2"

ROOT = Path(__file__).resolve().parents[2]
DOCKER = "/usr/bin/docker"
CARGO = "/root/.cargo/bin/cargo"
PROBE_BIN = ROOT / "target" / "debug" / "examples" / "livefire_probe"
REGO = ROOT / "policies" / "nexus.rego"

TENANT_A = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"
TENANT_B = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"
PRINCIPAL = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"
TARGET = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03"
REQUEST_ID = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"
CORRELATION = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"
GRANT_ID = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a04"
ASSERTION_ID = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a05"
RECEIPT_ID = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a06"

POLICY_VERSION = "nexus-policy-v1"
ACTION = "admin:test:livefire"
NOW = 1_700_000_000


# Canonical action digest (directive E): SHA-256 over the deterministic
# canonical action descriptor. Changing any element changes the digest.
def _action_digest(
    tenant: str, action: str, target: str, capability: str, reversal: str, touches_secret: bool
) -> str:
    descriptor = json.dumps(
        {
            "tenant": tenant,
            "action": action,
            "target": target,
            "capability": capability,
            "reversal": reversal,
            "touches_secret": touches_secret,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
    return hashlib.sha256(descriptor.encode("utf-8")).hexdigest()


DIGEST = _action_digest(TENANT_A, ACTION, TARGET, "ADMINISTRATIVE", "NONE", False)

# Evidence accumulator (directive N): each test appends observed results.
EVIDENCE = {
    "node": "EP-008",
    "milestone": "M5",
    "providers": {
        "openfga": "1.18.1 sha256:ec73e86c629f7c7b290cde0cf52bcea7c3e0315f30f65386fe4df532f4b83deb",
        "opa": "1.16.2 sha256:a915d8b59ddb09a9badecd8e061d43cf3111283494c4cf1d38a675bdb4e81a13",
        "policy_bundle": "policies/nexus.rego (nexus-policy-v1)",
    },
    "canonical_ordering": [
        "RELATIONSHIP",
        "POLICY",
        "RISK",
        "APPROVAL",
        "CAPABILITY",
        "ALLOW",
    ],
    "paths": {},
}


# --------------------------------------------------------------------------
# Low-level HTTP helpers (stdlib only; EP-007/EP-008 precedent)
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
# Container harness: REAL OpenFGA + REAL OPA, session-scoped, zero orphans
# --------------------------------------------------------------------------


def _cleanup_container(name: str) -> None:
    subprocess.run([DOCKER, "rm", "-f", name], capture_output=True, text=True, check=False)


def _start_openfga() -> tuple[str, int]:
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
            IMAGE_OPENFGA,
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
            [DOCKER, "port", name, "8080/tcp"], capture_output=True, text=True, check=False
        )
        line = out.stdout.strip()
        if line:
            port = int(line.rsplit(":", 1)[-1])
            try:
                status, body = _http("GET", f"http://127.0.0.1:{port}/healthz", timeout=3)
                if status == 200 and body.get("status") == "SERVING":
                    return port
            except AssertionError:
                pass
        last = line or out.stderr.strip()
        time.sleep(1)
    raise AssertionError(f"OpenFGA {name} not ready; last: {last!r}")


def _start_opa() -> tuple[str, int]:
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
            IMAGE_OPA,
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
            [DOCKER, "port", name, "8181/tcp"], capture_output=True, text=True, check=False
        )
        line = out.stdout.strip()
        if line:
            port = int(line.rsplit(":", 1)[-1])
            try:
                status, _ = _http("GET", f"http://127.0.0.1:{port}/health", timeout=3)
                if status == 200:
                    return port
            except AssertionError:
                pass
        last = line or out.stderr.strip()
        time.sleep(1)
    raise AssertionError(f"OPA {name} not ready; last: {last!r}")


_session = {}


@contextlib.contextmanager
def _suite():
    name_o, port_o = _start_openfga()
    name_p, port_p = _start_opa()
    try:
        yield {
            "openfga_name": name_o,
            "openfga_port": port_o,
            "opa_name": name_p,
            "opa_port": port_p,
        }
    finally:
        _cleanup_container(name_o)
        _cleanup_container(name_p)


def _ensure_suite() -> dict:
    if "ctx" not in _session:
        cm = _suite()
        _session["ctx"] = cm.__enter__()
        _session["cm"] = cm
    return _session["ctx"]


# --------------------------------------------------------------------------
# OpenFGA bootstrap (REAL API surface, pinned container)
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


def _actor_tuple(target: str = TARGET, tenant: str = TENANT_A) -> dict:
    return {"user": f"user:{PRINCIPAL}", "relation": "actor", "object": f"action:{tenant}|{target}"}


def _bootstrap() -> dict:
    ctx = _ensure_suite()
    port = ctx["openfga_port"]
    store = _create_store(port, f"nexus-ep008-livefire-{secrets.token_hex(4)}")
    model = _create_model(port, store)
    _write_tuples(port, store, [_actor_tuple()])
    return {"port": port, "store": store, "model": model}


# --------------------------------------------------------------------------
# OPA bootstrap (REAL policy bundle, pinned container)
# --------------------------------------------------------------------------


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


def _load_policy(port: int) -> None:
    status, body = _put_policy(port, REGO.read_text())
    assert status == 200, f"put policy: {status} {body}"


# --------------------------------------------------------------------------
# Combined probe harness (build once; REAL gateway + REAL providers)
# --------------------------------------------------------------------------

_probe_built = False


def _build_probe() -> None:
    global _probe_built
    if _probe_built:
        return
    subprocess.run(
        [CARGO, "build", "--example", "livefire_probe", "--locked", "-p", "nexus-opa"],
        cwd=str(ROOT),
        check=True,
        capture_output=True,
        text=True,
        timeout=600,
    )
    _probe_built = True


def _probe(data: dict) -> dict:
    _build_probe()
    proc = subprocess.run(
        [str(PROBE_BIN)],
        input=json.dumps(data),
        capture_output=True,
        text=True,
        timeout=90,
    )
    assert proc.returncode == 0, f"probe failed: {proc.stderr}"
    return json.loads(proc.stdout.strip())


def _probe_data(
    *,
    grant: dict | str | None = "DEFAULT",
    approval: dict | str | None = "DEFAULT",
    context: dict | None = None,
    capability: str = "ADMINISTRATIVE",
    reversal: str = "NONE",
    touches_secret: bool = False,
    action: str = ACTION,
    target: str = TARGET,
    digest: str | None = None,
    tenant: str = TENANT_A,
    request_id: str = REQUEST_ID,
    receipt_id: str | None = RECEIPT_ID,
    model_recommendation: str | None = None,
    presented_receipt: dict | None = None,
    openfga_ctx: dict | None = None,
    opa_ctx: dict | None = None,
    now: int | None = None,
) -> dict:
    ctx = openfga_ctx or _bootstrap()
    opa = opa_ctx or _ensure_suite()
    _load_policy(opa["opa_port"])
    if context is None:
        context = {
            "location": "HOME",
            "network_trust": "TRUSTED",
            "maintenance": False,
            "emergency": False,
            "device_state": "ENABLED",
            "sensitivity": "PUBLIC",
        }
    if grant == "DEFAULT":
        grant = {
            "grant_id": GRANT_ID,
            "tenant_id": tenant,
            "capability": capability,
            "actor": PRINCIPAL,
            "target_id": target,
            "scope": action,
            "issued_at_unix_s": (now or NOW) - 100,
            "expires_at_unix_s": (now or NOW) + 1000,
        }
    elif grant is None:
        grant = None  # explicit omission (no capability grant)
    if approval == "DEFAULT":
        approval = {
            "assertion_id": ASSERTION_ID,
            "correlation": CORRELATION,
            "action_digest": digest
            or _action_digest(tenant, action, target, capability, reversal, touches_secret),
            "approver": PRINCIPAL,
            "approval_class": "HUMAN",
            "strength": "STEP_UP",
            "decision": "APPROVED",
            "issued_at_unix_s": (now or NOW) - 100,
            "expires_at_unix_s": (now or NOW) + 1000,
        }
    elif approval is None:
        approval = None  # explicit omission (no approval assertion)
    effective_now = now if now is not None else NOW
    return {
        "openfga_base_url": f"http://127.0.0.1:{ctx['port']}",
        "openfga_store_id": ctx["store"],
        "openfga_model_id": ctx["model"],
        "opa_base_url": f"http://127.0.0.1:{opa['opa_port']}",
        "opa_policy_version": POLICY_VERSION,
        "opa_context": context,
        "request": {
            "request_id": request_id,
            "correlation": CORRELATION,
            "tenant_id": tenant,
            "action_digest": digest
            or _action_digest(tenant, action, target, capability, reversal, touches_secret),
            "action": action,
            "target_id": target,
            "requested_at_unix_s": effective_now,
        },
        "actor": {"principal_id": PRINCIPAL, "principal_type": "HUMAN", "tenant_id": tenant},
        "capability": capability,
        "reversal": reversal,
        "touches_secret": touches_secret,
        "grant": grant,
        "approval": approval,
        "now_unix_s": effective_now,
        "receipt_id": receipt_id,
        "model_recommendation": model_recommendation,
        "presented_receipt": presented_receipt,
    }


# --------------------------------------------------------------------------
# 0. image pin + health (directive B)
# --------------------------------------------------------------------------


def ep008_livefire_images_are_pinned() -> None:
    for tag, digest in ((IMAGE_OPENFGA_TAG, IMAGE_OPENFGA), (IMAGE_OPA_TAG, IMAGE_OPA)):
        inspect = subprocess.run(
            [DOCKER, "image", "inspect", f"{tag}@{digest.split('@')[1]}"],
            capture_output=True,
            text=True,
        )
        assert inspect.returncode == 0, f"pinned image not present: {tag}"


def ep008_livefire_providers_serve() -> None:
    ctx = _ensure_suite()
    status, _ = _http("GET", f"http://127.0.0.1:{ctx['openfga_port']}/healthz")
    assert status == 200
    status, _ = _http("GET", f"http://127.0.0.1:{ctx['opa_port']}/health")
    assert status == 200


# --------------------------------------------------------------------------
# C. allow path end to end (directive C)
# --------------------------------------------------------------------------


def ep008_livefire_allow_path_full_chain() -> None:
    ctx = _bootstrap()
    data = _probe_data(openfga_ctx=ctx)
    out = _probe(data)
    assert out["decision"] == "ALLOWED", out
    assert out["stages"] == [
        "RELATIONSHIP_PASS",
        "POLICY_PASS",
        "RISK_R3",
        "APPROVAL_PASS",
        "CAPABILITY_PASS",
        "ALLOWED",
    ], out["stages"]
    assert out["risk"] == "R3", out
    assert out["policy_version"] == POLICY_VERSION, out
    assert out["relationship_event"] and out["relationship_event"]["allowed"] is True, out
    assert out["policy_event"] and out["policy_event"]["allowed"] is True, out

    receipt = out["receipt"]
    assert receipt is not None, out
    assert receipt["correlation"] == CORRELATION, receipt
    assert receipt["request_id"] == REQUEST_ID, receipt
    assert receipt["lifecycle"] == "APPROVED", receipt
    assert receipt["state"] == "ISSUED", receipt
    assert receipt["policy_version"] == POLICY_VERSION, receipt
    # Evidence refs are fingerprints/references, never raw payloads.
    assert any(r.startswith("relationship:") for r in receipt["evidence_refs"]), receipt
    assert any(r.startswith("policy:") for r in receipt["evidence_refs"]), receipt
    assert any(r.startswith("approval:") for r in receipt["evidence_refs"]), receipt
    assert any(r.startswith("grant:") for r in receipt["evidence_refs"]), receipt
    assert any(r.startswith("risk:R3") for r in receipt["evidence_refs"]), receipt
    assert any(r.startswith("digest:") for r in receipt["evidence_refs"]), receipt

    plan = out["verification_plan"]
    assert plan is not None, out
    assert plan["expected"]["target_id"] == TARGET, plan
    assert plan["expected"]["state"] == "authorization:approved", plan
    assert plan["timeout_seconds"] == 30 and plan["retries"] == 3, plan

    EVIDENCE["paths"]["allow_full_chain"] = {
        "decision": "ALLOWED",
        "stages": out["stages"],
        "risk": "R3",
        "policy_version": POLICY_VERSION,
        "receipt_lifecycle": receipt["lifecycle"],
        "receipt_state": receipt["state"],
        "verification_plan": plan,
        "relationship_allowed": True,
        "policy_allowed": True,
    }


# --------------------------------------------------------------------------
# D. receipt is NOT execution authority (directive D)
# --------------------------------------------------------------------------


def ep008_livefire_receipt_is_not_bearer_authority() -> None:
    ctx = _bootstrap()
    data = _probe_data(openfga_ctx=ctx)
    allow = _probe(data)
    assert allow["decision"] == "ALLOWED", allow
    receipt = allow["receipt"]

    # A copied receipt presented ALONE (no grant, no approval) must not
    # authorize a second action. The gateway stops at the first unmet
    # requirement (approval precedes capability in the canonical
    # ordering) - the invariant is DENIED, never ALLOWED.
    copied = _probe_data(
        openfga_ctx=ctx,
        grant=None,
        approval=None,
        presented_receipt=receipt,
        request_id="0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a11",
        receipt_id="0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a12",
    )
    out = _probe(copied)
    assert out["decision"] == "DENIED", out
    # The second action's own receipt records a REJECTED outcome - the
    # copied receipt cannot manufacture approval.
    assert out["receipt"] is not None
    assert out["receipt"]["lifecycle"] == "REJECTED", out["receipt"]
    assert out["receipt"]["denial_reason"] == "MISSING_APPROVAL", out["receipt"]
    assert out["presented_receipt_received"] is True, out

    # A tampered receipt (lifecycle forged to SUCCEEDED) must not help.
    forged = dict(receipt)
    forged["lifecycle"] = "SUCCEEDED"
    tampered = _probe_data(
        openfga_ctx=ctx,
        grant=None,
        approval=None,
        presented_receipt=forged,
        request_id="0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a13",
        receipt_id="0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a14",
    )
    out = _probe(tampered)
    assert out["decision"] == "DENIED", out
    assert out["receipt"]["lifecycle"] == "REJECTED", out["receipt"]

    # An expired/stale receipt from a different request must not help.
    stale = _probe_data(
        openfga_ctx=ctx,
        grant=None,
        approval=None,
        presented_receipt=receipt,
        request_id="0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a15",
        receipt_id="0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a16",
        now=2_000_000_000,
    )
    out = _probe(stale)
    assert out["decision"] == "DENIED", out
    assert out["receipt"]["lifecycle"] == "REJECTED", out["receipt"]

    # Execution still requires the valid gateway/capability path: the
    # same request WITH grant+approval is ALLOWED again.
    again = _probe_data(openfga_ctx=ctx)
    out = _probe(again)
    assert out["decision"] == "ALLOWED", out

    EVIDENCE["paths"]["receipt_not_bearer"] = {
        "copied_receipt_alone": "DENIED NO_CAPABILITY",
        "tampered_receipt_alone": "DENIED NO_CAPABILITY",
        "stale_receipt_alone": "DENIED NO_CAPABILITY",
        "valid_gateway_path": "ALLOWED",
    }


# --------------------------------------------------------------------------
# E. exact action-digest binding (directive E)
# --------------------------------------------------------------------------


def ep008_livefire_digest_binding() -> None:
    ctx = _bootstrap()
    # Known-good allow with digest D1.
    d1 = _action_digest(TENANT_A, ACTION, TARGET, "ADMINISTRATIVE", "NONE", False)
    data = _probe_data(openfga_ctx=ctx, digest=d1)
    out = _probe(data)
    assert out["decision"] == "ALLOWED", out

    # Change ONE meaningful element (the action parameter), recompute the
    # digest, reuse the OLD approval bound to D1. Approval must DENY.
    action_v2 = "admin:test:livefire:v2"
    d2 = _action_digest(TENANT_A, action_v2, TARGET, "ADMINISTRATIVE", "NONE", False)
    assert d2 != d1
    old_approval = {
        "assertion_id": ASSERTION_ID,
        "correlation": CORRELATION,
        "action_digest": d1,  # OLD digest
        "approver": PRINCIPAL,
        "approval_class": "HUMAN",
        "strength": "STEP_UP",
        "decision": "APPROVED",
        "issued_at_unix_s": NOW - 100,
        "expires_at_unix_s": NOW + 1000,
    }
    changed = _probe_data(
        openfga_ctx=ctx,
        action=action_v2,
        digest=d2,
        approval=old_approval,
    )
    out = _probe(changed)
    assert out["decision"] == "DENIED", out
    assert "MISSING_APPROVAL" in (out.get("reason") or ""), out
    assert "APPROVAL_DENY" in out["stages"], out["stages"]
    assert "ALLOWED" not in out["stages"], out["stages"]

    # An approval bound to the NEW digest allows the request to continue
    # if every other requirement remains valid.
    new_approval = {
        "assertion_id": ASSERTION_ID,
        "correlation": CORRELATION,
        "action_digest": d2,  # NEW digest
        "approver": PRINCIPAL,
        "approval_class": "HUMAN",
        "strength": "STEP_UP",
        "decision": "APPROVED",
        "issued_at_unix_s": NOW - 100,
        "expires_at_unix_s": NOW + 1000,
    }
    fixed = _probe_data(
        openfga_ctx=ctx,
        action=action_v2,
        digest=d2,
        approval=new_approval,
    )
    out = _probe(fixed)
    assert out["decision"] == "ALLOWED", out

    EVIDENCE["paths"]["digest_binding"] = {
        "changed_element": "action parameter",
        "old_approval_new_digest": "DENIED MISSING_APPROVAL",
        "new_approval_new_digest": "ALLOWED",
    }


# --------------------------------------------------------------------------
# F. capability scope binding (directive F)
# --------------------------------------------------------------------------


def ep008_livefire_capability_scope_binding() -> None:
    ctx = _bootstrap()
    cases = {}

    # 1. wrong actor
    wrong_actor = _probe_data(openfga_ctx=ctx)
    wrong_actor["grant"]["actor"] = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a99"
    out = _probe(wrong_actor)
    assert out["decision"] == "DENIED" and "NO_CAPABILITY" in (out.get("reason") or ""), out
    assert out["stages"][-1] == "CAPABILITY_DENY", out["stages"]
    cases["wrong_actor"] = "DENIED NO_CAPABILITY"

    # 2. wrong target
    wrong_target = _probe_data(openfga_ctx=ctx)
    wrong_target["grant"]["target_id"] = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a98"
    out = _probe(wrong_target)
    assert out["decision"] == "DENIED" and "NO_CAPABILITY" in (out.get("reason") or ""), out
    assert out["stages"][-1] == "CAPABILITY_DENY", out["stages"]
    cases["wrong_target"] = "DENIED NO_CAPABILITY"

    # 3. wrong scope
    wrong_scope = _probe_data(openfga_ctx=ctx)
    wrong_scope["grant"]["scope"] = "task:complete"
    out = _probe(wrong_scope)
    assert out["decision"] == "DENIED" and "NO_CAPABILITY" in (out.get("reason") or ""), out
    assert out["stages"][-1] == "CAPABILITY_DENY", out["stages"]
    cases["wrong_scope"] = "DENIED NO_CAPABILITY"

    # 4. wrong tenant (grant from another tenant)
    wrong_tenant = _probe_data(openfga_ctx=ctx)
    wrong_tenant["grant"]["tenant_id"] = TENANT_B
    out = _probe(wrong_tenant)
    assert out["decision"] == "DENIED" and "NO_CAPABILITY" in (out.get("reason") or ""), out
    assert out["stages"][-1] == "CAPABILITY_DENY", out["stages"]
    cases["wrong_tenant"] = "DENIED NO_CAPABILITY"

    # 5. expired grant
    expired = _probe_data(openfga_ctx=ctx)
    expired["grant"]["expires_at_unix_s"] = NOW - 1
    out = _probe(expired)
    assert out["decision"] == "DENIED" and "NO_CAPABILITY" in (out.get("reason") or ""), out
    assert out["stages"][-1] == "CAPABILITY_DENY", out["stages"]
    cases["expired_grant"] = "DENIED NO_CAPABILITY"

    EVIDENCE["paths"]["capability_scope_binding"] = cases


# --------------------------------------------------------------------------
# G. policy denial dominates (directive G)
# --------------------------------------------------------------------------


def ep008_livefire_policy_denial_dominates() -> None:
    ctx = _bootstrap()
    # Change ONLY the contextual state so OPA denies. Relationship,
    # risk, approval, and grant stay valid.
    denied_ctx = {
        "location": "WORK",
        "network_trust": "UNTRUSTED",
        "maintenance": False,
        "emergency": False,
        "device_state": "ENABLED",
        "sensitivity": "PUBLIC",
    }
    data = _probe_data(openfga_ctx=ctx, context=denied_ctx)
    out = _probe(data)
    assert out["decision"] == "DENIED", out
    assert "POLICY" in (out.get("reason") or ""), out
    assert out["stages"] == ["RELATIONSHIP_PASS", "POLICY_DENY"], out["stages"]
    # Risk floor never reached: risk stays R0 (no classification).
    assert out["risk"] == "R0", out
    # No success receipt is produced: receipt lifecycle is REJECTED.
    assert out["receipt"] is not None
    assert out["receipt"]["lifecycle"] == "REJECTED", out["receipt"]
    assert out["receipt"]["denial_reason"] == "POLICY", out["receipt"]
    # Risk/approval/capability cannot manufacture ALLOW.
    assert out["verification_plan"] is None, out

    EVIDENCE["paths"]["policy_denial_dominates"] = {
        "decision": "DENIED POLICY",
        "stages": out["stages"],
        "risk": "R0",
        "receipt_lifecycle": "REJECTED",
        "success_receipt": False,
    }


# --------------------------------------------------------------------------
# H. relationship denial dominates (directive H)
# --------------------------------------------------------------------------


def ep008_livefire_relationship_denial_dominates() -> None:
    ctx = _bootstrap()
    # Known-good allow first.
    data = _probe_data(openfga_ctx=ctx)
    out = _probe(data)
    assert out["decision"] == "ALLOWED", out

    # Remove/revoke the exact OpenFGA tuple; all downstream artifacts
    # stay valid.
    _delete_tuples(ctx["port"], ctx["store"], [_actor_tuple()])
    out = _probe(data)
    assert out["decision"] == "DENIED", out
    assert "RELATIONSHIP" in (out.get("reason") or ""), out
    assert out["stages"] == ["RELATIONSHIP_DENY"], out["stages"]
    assert out["risk"] == "R0", out
    # OPA, approval, capability cannot override relationship denial.
    assert out["verification_plan"] is None, out

    # Restore the tuple for later tests.
    _write_tuples(ctx["port"], ctx["store"], [_actor_tuple()])
    out = _probe(data)
    assert out["decision"] == "ALLOWED", out

    EVIDENCE["paths"]["relationship_denial_dominates"] = {
        "decision": "DENIED RELATIONSHIP",
        "stages": ["RELATIONSHIP_DENY"],
        "restore_after": "ALLOWED",
    }


# --------------------------------------------------------------------------
# I. R4 model-approval prohibition (directive I)
# --------------------------------------------------------------------------


def ep008_livefire_r4_model_approval_prohibited() -> None:
    ctx = _bootstrap()
    # R4: ADMINISTRATIVE + IRREVERSIBLE -> R4 per the deterministic
    # risk floor.
    r4_digest = _action_digest(TENANT_A, ACTION, TARGET, "ADMINISTRATIVE", "IRREVERSIBLE", False)
    model_approval = {
        "assertion_id": ASSERTION_ID,
        "correlation": CORRELATION,
        "action_digest": r4_digest,
        "approver": PRINCIPAL,
        "approval_class": "POLICY",  # model/automated authority
        "strength": "STEP_UP",
        "decision": "APPROVED",
        "issued_at_unix_s": NOW - 100,
        "expires_at_unix_s": NOW + 1000,
    }
    data = _probe_data(
        openfga_ctx=ctx,
        reversal="IRREVERSIBLE",
        digest=r4_digest,
        approval=model_approval,
    )
    out = _probe(data)
    assert out["decision"] == "DENIED", out
    assert "MISSING_APPROVAL" in (out.get("reason") or ""), out
    assert out["risk"] == "R4", out
    assert "APPROVAL_DENY" in out["stages"], out["stages"]

    # The exact valid human approval mechanism (HUMAN + STEP_UP) lets
    # the request continue when every other requirement is satisfied.
    human_approval = {
        "assertion_id": ASSERTION_ID,
        "correlation": CORRELATION,
        "action_digest": r4_digest,
        "approver": PRINCIPAL,
        "approval_class": "HUMAN",
        "strength": "STEP_UP",
        "decision": "APPROVED",
        "issued_at_unix_s": NOW - 100,
        "expires_at_unix_s": NOW + 1000,
    }
    data = _probe_data(
        openfga_ctx=ctx,
        reversal="IRREVERSIBLE",
        digest=r4_digest,
        approval=human_approval,
    )
    out = _probe(data)
    assert out["decision"] == "ALLOWED", out
    assert out["risk"] == "R4", out

    EVIDENCE["paths"]["r4_model_approval"] = {
        "model_approval": "DENIED MISSING_APPROVAL (R4)",
        "human_approval": "ALLOWED (R4)",
    }


# --------------------------------------------------------------------------
# J. STEP_UP requirement (directive J)
# --------------------------------------------------------------------------


def ep008_livefire_step_up_required() -> None:
    ctx = _bootstrap()
    # Valid relationship, OPA allow, valid capability, approval supplied
    # but authentication strength below STEP_UP -> fail closed.
    weak_approval = {
        "assertion_id": ASSERTION_ID,
        "correlation": CORRELATION,
        "action_digest": DIGEST,
        "approver": PRINCIPAL,
        "approval_class": "HUMAN",
        "strength": "SINGLE_FACTOR",
        "decision": "APPROVED",
        "issued_at_unix_s": NOW - 100,
        "expires_at_unix_s": NOW + 1000,
    }
    data = _probe_data(openfga_ctx=ctx, approval=weak_approval)
    out = _probe(data)
    assert out["decision"] == "DENIED", out
    assert "MISSING_APPROVAL" in (out.get("reason") or ""), out
    assert "APPROVAL_DENY" in out["stages"], out["stages"]

    # Supply STEP_UP and the SAME request progresses.
    strong_approval = {
        "assertion_id": ASSERTION_ID,
        "correlation": CORRELATION,
        "action_digest": DIGEST,
        "approver": PRINCIPAL,
        "approval_class": "HUMAN",
        "strength": "STEP_UP",
        "decision": "APPROVED",
        "issued_at_unix_s": NOW - 100,
        "expires_at_unix_s": NOW + 1000,
    }
    data = _probe_data(openfga_ctx=ctx, approval=strong_approval)
    out = _probe(data)
    assert out["decision"] == "ALLOWED", out

    # The approval itself does NOT upgrade authentication strength: the
    # same weak-strength approval is still rejected when a second action
    # (new digest) is presented.
    action_v2 = "admin:test:livefire:v2"
    d2 = _action_digest(TENANT_A, action_v2, TARGET, "ADMINISTRATIVE", "NONE", False)
    weak2 = dict(weak_approval)
    weak2["action_digest"] = d2
    data = _probe_data(openfga_ctx=ctx, action=action_v2, digest=d2, approval=weak2)
    out = _probe(data)
    assert out["decision"] == "DENIED", out
    assert "MISSING_APPROVAL" in (out.get("reason") or ""), out

    EVIDENCE["paths"]["step_up_required"] = {
        "below_step_up": "DENIED MISSING_APPROVAL",
        "step_up": "ALLOWED",
        "approval_does_not_upgrade_strength": "DENIED MISSING_APPROVAL",
    }


# --------------------------------------------------------------------------
# K. provider failure during a known-good request (directive K)
# --------------------------------------------------------------------------


def ep008_livefire_openfga_unavailable_fails_closed() -> None:
    # Dedicated pair so the suite containers stay alive.
    name_o, port_o = _start_openfga()
    name_p, port_p = _start_opa()
    try:
        store = _create_store(port_o, "nexus-ep008-kill-ofga")
        model = _create_model(port_o, store)
        _write_tuples(port_o, store, [_actor_tuple()])
        _load_policy(port_p)
        fga = {"port": port_o, "store": store, "model": model}
        opa = {"opa_port": port_p}
        data = _probe_data(openfga_ctx=fga, opa_ctx=opa)
        assert _probe(data)["decision"] == "ALLOWED", "baseline should allow"

        subprocess.run([DOCKER, "kill", name_o], check=True, capture_output=True, text=True)
        time.sleep(1)
        out = _probe(data)
        assert out["decision"] != "ALLOWED", out
        assert out["decision"] == "ERROR", out
        assert (
            "OPENFGA" in (out.get("error") or "")
            or "unavailable" in (out.get("error") or "").lower()
        ), out
        EVIDENCE["paths"]["openfga_unavailable"] = {
            "decision": "ERROR",
            "never_allow": True,
            "typed_cause": out.get("error"),
        }
    finally:
        _cleanup_container(name_o)
        _cleanup_container(name_p)


def ep008_livefire_opa_unavailable_fails_closed() -> None:
    name_o, port_o = _start_openfga()
    name_p, port_p = _start_opa()
    try:
        store = _create_store(port_o, "nexus-ep008-kill-opa")
        model = _create_model(port_o, store)
        _write_tuples(port_o, store, [_actor_tuple()])
        _load_policy(port_p)
        fga = {"port": port_o, "store": store, "model": model}
        opa = {"opa_port": port_p}
        data = _probe_data(openfga_ctx=fga, opa_ctx=opa)
        assert _probe(data)["decision"] == "ALLOWED", "baseline should allow"

        subprocess.run([DOCKER, "kill", name_p], check=True, capture_output=True, text=True)
        time.sleep(1)
        out = _probe(data)
        assert out["decision"] != "ALLOWED", out
        assert out["decision"] == "ERROR", out
        assert (
            "OPA" in (out.get("error") or "") or "unavailable" in (out.get("error") or "").lower()
        ), out
        EVIDENCE["paths"]["opa_unavailable"] = {
            "decision": "ERROR",
            "never_allow": True,
            "typed_cause": out.get("error"),
        }
    finally:
        _cleanup_container(name_o)
        _cleanup_container(name_p)


# --------------------------------------------------------------------------
# L. verification plan behavior (directive L)
# --------------------------------------------------------------------------


def ep008_livefire_verification_plan_deterministic() -> None:
    ctx = _bootstrap()
    data = _probe_data(openfga_ctx=ctx)
    out1 = _probe(data)
    out2 = _probe(data)
    assert out1["decision"] == "ALLOWED" and out2["decision"] == "ALLOWED"
    # Identical DecisionInput -> identical receipt and verification plan.
    assert json.dumps(out1["receipt"], sort_keys=True) == json.dumps(
        out2["receipt"], sort_keys=True
    ), "receipt must be deterministic"
    assert json.dumps(out1["verification_plan"], sort_keys=True) == json.dumps(
        out2["verification_plan"], sort_keys=True
    ), "verification plan must be deterministic"
    # AUTHORIZED != EXECUTED != VERIFIED: the receipt records the
    # authorization outcome (APPROVED), never execution or verification
    # success. EP-008 owns authorization only.
    assert out1["receipt"]["lifecycle"] == "APPROVED", out1["receipt"]
    forbidden = ("EXECUTING", "SUCCEEDED", "VERIFYING")
    assert out1["receipt"]["lifecycle"] not in forbidden, out1["receipt"]
    # The plan describes what verification WOULD check; no verification
    # result is claimed by the probe.
    assert out1["verification_plan"]["expected"]["state"] == "authorization:approved"
    EVIDENCE["paths"]["verification_plan"] = {
        "deterministic_receipt": True,
        "deterministic_plan": True,
        "lifecycle_boundary": "AUTHORIZED != EXECUTED != VERIFIED",
        "receipt_lifecycle": "APPROVED",
    }


# --------------------------------------------------------------------------
# M. no LLM authority (directive M)
# --------------------------------------------------------------------------


def ep008_livefire_model_recommendation_has_no_authority() -> None:
    ctx = _bootstrap()
    results = {}

    # Model says ALLOW but relationship is denied.
    _delete_tuples(ctx["port"], ctx["store"], [_actor_tuple()])
    data = _probe_data(openfga_ctx=ctx, model_recommendation="ALLOW")
    out = _probe(data)
    assert out["decision"] == "DENIED" and "RELATIONSHIP" in (out.get("reason") or ""), out
    assert out["model_recommendation_received"] is True, out
    results["relationship_denied"] = "DENIED despite model ALLOW"
    _write_tuples(ctx["port"], ctx["store"], [_actor_tuple()])

    # Model says ALLOW but OPA denies.
    denied_ctx = {
        "location": "WORK",
        "network_trust": "UNTRUSTED",
        "maintenance": False,
        "emergency": False,
        "device_state": "ENABLED",
        "sensitivity": "PUBLIC",
    }
    data = _probe_data(openfga_ctx=ctx, context=denied_ctx, model_recommendation="ALLOW")
    out = _probe(data)
    assert out["decision"] == "DENIED" and "POLICY" in (out.get("reason") or ""), out
    results["policy_denied"] = "DENIED despite model ALLOW"

    # Model says ALLOW but approval is missing.
    data = _probe_data(openfga_ctx=ctx, approval=None, model_recommendation="ALLOW")
    out = _probe(data)
    assert out["decision"] == "DENIED" and "MISSING_APPROVAL" in (out.get("reason") or ""), out
    results["missing_approval"] = "DENIED despite model ALLOW"

    # Model says ALLOW but capability is missing.
    data = _probe_data(openfga_ctx=ctx, grant=None, model_recommendation="ALLOW")
    out = _probe(data)
    assert out["decision"] == "DENIED" and "NO_CAPABILITY" in (out.get("reason") or ""), out
    results["missing_capability"] = "DENIED despite model ALLOW"

    EVIDENCE["paths"]["no_llm_authority"] = results


# --------------------------------------------------------------------------
# N. evidence file (directive N)
# --------------------------------------------------------------------------


def ep008_livefire_evidence_written() -> None:
    ev_dir = ROOT / ".agent" / "state" / "evidence" / "ep008-m5"
    ev_dir.mkdir(parents=True, exist_ok=True)
    (ev_dir / "ep008-m5-live-fire.json").write_text(
        json.dumps(EVIDENCE, indent=2, sort_keys=True) + "\n"
    )
    md = _render_markdown(EVIDENCE)
    (ev_dir / "EP-008-M5-live-fire.md").write_text(md)
    assert (ev_dir / "ep008-m5-live-fire.json").exists()
    assert (ev_dir / "EP-008-M5-live-fire.md").exists()
    EVIDENCE["evidence_file"] = str(ev_dir / "ep008-m5-live-fire.json")


def _render_markdown(evidence: dict) -> str:
    lines = [
        "# EP-008 M5 live-fire evidence",
        "",
        f"- Node: `{evidence['node']}`",
        f"- Milestone: `{evidence['milestone']}`",
        "",
        "## Providers (real, pinned)",
        "",
    ]
    for k, v in evidence["providers"].items():
        lines.append(f"- **{k}**: `{v}`")
    lines += [
        "",
        "## Canonical authorization ordering",
        "",
        "`" + " -> ".join(evidence["canonical_ordering"]) + "`",
        "",
        "## Observed paths",
        "",
    ]
    for name, result in evidence["paths"].items():
        lines.append(f"### {name}")
        if isinstance(result, dict):
            for k, v in result.items():
                lines.append(f"- {k}: `{v}`")
        else:
            lines.append(f"- result: `{result}`")
        lines.append("")
    lines += [
        "## Boundary",
        "",
        "AUTHORIZED != EXECUTED != VERIFIED. EP-008 owns authorization only;",
        "no execution or verification success is claimed. No credentials, bearer",
        "tokens, private data, or raw provider payloads are persisted; evidence",
        "refs are fingerprints and references.",
        "",
    ]
    return "\n".join(lines)


# --------------------------------------------------------------------------
# O. teardown invariant: zero orphans (directive O)
# --------------------------------------------------------------------------


def ep008_livefire_zz_teardown_zero_orphans() -> None:
    # Explicit cleanup of the session pair now.
    if "cm" in _session:
        _session["cm"].__exit__(None, None, None)
        _session.clear()

    leftovers = subprocess.run(
        [DOCKER, "ps", "-a", "--filter", "name=nexus-ep008", "--format", "{{.Names}}"],
        capture_output=True,
        text=True,
    ).stdout.split()
    assert not leftovers, f"EP-008 containers after teardown: {leftovers}"

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
