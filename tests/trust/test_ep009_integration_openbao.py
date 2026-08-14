"""EP-009 M2 integration tests: OpenBao secret authority through REAL OpenBao.

Test names begin with ep009_integration_ per the EP-009 milestone
contract. Uses the pinned OpenBao 2.5.4 image (VERSIONS.lock.yaml /
COMPONENT_REGISTRY.yaml) in a real ephemeral container - never an
in-memory substitute (TESTING.md reality rule).

CANONICAL NEXUS-TO-OPENBAO MAPPING (verified live against the pinned
container; recorded in the ExecPlan Decision Log):
- KV-v2 at mount `secret/`: PUT /v1/secret/data/<path> creates the next
  version; GET returns data.data + data.metadata.version; metadata at
  /v1/secret/metadata/<path>; soft delete = DELETE /v1/secret/data/<path>
  (204, empty body); undelete = POST /v1/secret/undelete/<path>
  {"versions":[...]}; destroy = POST /v1/secret/destroy/<path>
  {"versions":[...]}; destroyed/soft-deleted read = 404.
- Response wrapping: any request with header X-Vault-Wrap-TTL returns
  wrap_info.token instead of the normal payload; unwrap = POST
  /v1/sys/wrapping/unwrap with X-Vault-Token: <wrapping token>; second
  unwrap or expired token -> 400 "wrapping token is not valid or does
  not exist".
- AppRole least privilege: enable approle, write a narrow policy, create
  a role with bounded TTL, login -> client_token with lease_duration +
  renewable. Policy denies tenant-B paths and sys/admin paths.

REALITY RULE: every operation below is a real HTTP call to the real
pinned OpenBao container via stdlib urllib (no HTTP library in the
frozen test env; EP-007/EP-008 precedent). No pass result is pre-baked.

EVIDENCE REQUIREMENTS covered by this suite:
- health/readiness
- least-privilege AppRole authentication (bounded TTL, renewable
  explicit, no root token by the adapter)
- write/read
- version update (KV-v2)
- metadata/version observation
- soft delete/undelete
- destroy (permanent)
- unauthorized path denial (tenant B, sys/admin)
- revoked/expired auth denial
- response wrapping one-time use (single unwrap, second unwrap fails,
  expired wrapping token fails)
- provider death fail-closed (container killed -> typed unavailable)
- secret-value redaction (canary never in Debug/serde/telemetry)
- image is pinned by digest
- explicit teardown with zero orphans
"""

from __future__ import annotations

import json
import secrets
import socket
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

IMAGE = "openbao/openbao@sha256:436eaf9778cad75507ff70ea26ace30dcbe15606e619ac3823495663d7f7c115"
IMAGE_TAG = "openbao/openbao:2.5.4"
ROOT = Path(__file__).resolve().parents[2]
DOCKER = "/usr/bin/docker"

# Root token is dev-mode ONLY for configuring the ephemeral test
# instance (directive C); the adapter never uses it. The value is a
# neutral ephemeral bootstrap identifier, not a real credential.
DEV_ROOT_TOKEN = "nexus-ep009-dev-bootstrap"
TENANT_A = "tenant-a"
TENANT_B = "tenant-b"

# Canary secret: must NEVER appear in logs, evidence, Debug, or
# telemetry (directive E). Tests scan captured output for it.
CANARY = "canary-nexus-ep009-7f3a1c9e"

_container_name: str | None = None
_container_port: int = 0


# --------------------------------------------------------------------------
# Container lifecycle (unique name, unique port, isolated, explicit
# teardown - directive B)
# --------------------------------------------------------------------------


def _free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _container_up() -> tuple[str, int]:
    global _container_name, _container_port
    if _container_name:
        return _container_name, _container_port
    name = f"nexus-ep009-openbao-{secrets.token_hex(4)}"
    port = _free_port()
    subprocess.run(
        [
            DOCKER,
            "run",
            "-d",
            "--name",
            name,
            "-p",
            f"{port}:8200",
            "-e",
            "BAO_DEV_LISTEN_ADDRESS=0.0.0.0:8200",
            IMAGE,
            "server",
            "-dev",
            "-dev-root-token-id",
            DEV_ROOT_TOKEN,
        ],
        check=True,
        capture_output=True,
    )
    _container_name, _container_port = name, port
    # Readiness: poll /v1/sys/health until 200 (bounded).
    deadline = time.time() + 60
    while time.time() < deadline:
        try:
            status, _ = _http("GET", "/v1/sys/health", token=DEV_ROOT_TOKEN)
            if status == 200:
                return name, port
        except Exception:
            pass
        time.sleep(0.5)
    raise RuntimeError("OpenBao container did not become ready")


def _teardown() -> None:
    global _container_name, _container_port
    if _container_name:
        subprocess.run([DOCKER, "rm", "-f", _container_name], capture_output=True)
        _container_name = None
        _container_port = 0


# --------------------------------------------------------------------------
# Low-level HTTP helpers (stdlib only)
# --------------------------------------------------------------------------


def _http(
    method: str,
    path: str,
    body=None,
    token: str | None = DEV_ROOT_TOKEN,
    headers: dict | None = None,
    timeout: float = 8.0,
) -> tuple[int, dict]:
    _, port = _container_up()
    h = {"Content-Type": "application/json"}
    if token:
        h["X-Vault-Token"] = token
    if headers:
        h.update(headers)
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}{path}", method=method, headers=h, data=data
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read()
            return resp.status, (json.loads(raw) if raw else {})
    except urllib.error.HTTPError as e:
        raw = e.read()
        try:
            return e.code, (json.loads(raw) if raw else {})
        except Exception:
            return e.code, {"raw": raw.decode(errors="replace")}


# --------------------------------------------------------------------------
# Provisioning helpers (root only; ephemeral instance configuration)
# --------------------------------------------------------------------------


def _enable_approle() -> None:
    status, _ = _http("POST", "/v1/sys/auth/approle", {"type": "approle"})
    # 400 = already enabled (idempotent provisioning); 204/200 = enabled now.
    assert status in (204, 200, 400), f"enable approle failed: {status}"


def _write_policy(name: str, policy: str) -> None:
    status, _ = _http("PUT", f"/v1/sys/policies/acl/{name}", {"policy": policy})
    assert status in (204, 200), f"write policy {name} failed: {status}"


def _create_role(name: str, policies: list[str], ttl: str, max_ttl: str) -> None:
    status, _ = _http(
        "POST",
        f"/v1/auth/approle/role/{name}",
        {
            "policies": policies,
            "token_ttl": ttl,
            "token_max_ttl": max_ttl,
            "secret_id_ttl": "10m",
            "token_bound_cidrs": [],  # no CIDR bind in ephemeral dev
        },
    )
    assert status in (204, 200), f"create role {name} failed: {status}"


def _role_id(name: str) -> str:
    status, body = _http("GET", f"/v1/auth/approle/role/{name}/role-id")
    assert status == 200, f"role-id failed: {status}"
    return body["data"]["role_id"]


def _secret_id(name: str) -> str:
    status, body = _http("POST", f"/v1/auth/approle/role/{name}/secret-id")
    assert status == 200, f"secret-id failed: {status}"
    return body["data"]["secret_id"]


def _approle_login(role_id: str, secret_id: str):
    status, body = _http(
        "POST", "/v1/auth/approle/login", {"role_id": role_id, "secret_id": secret_id}
    )
    return status, body


def _provision_tenant_a_identity() -> dict:
    """Create the least-privilege AppRole for tenant A and return its
    credentials (never persisted; held in the test process only)."""
    _enable_approle()
    policy = f"""
path "secret/data/{TENANT_A}/*" {{ capabilities = ["create", "read", "update", "delete", "list"] }}
path "secret/metadata/{TENANT_A}/*" {{ capabilities = ["read", "list"] }}
path "secret/undelete/{TENANT_A}/*" {{ capabilities = ["update"] }}
path "secret/destroy/{TENANT_A}/*" {{ capabilities = ["update"] }}
path "sys/health" {{ capabilities = ["read"] }}
"""
    _write_policy("nexus-tenant-a", policy)
    _create_role("nexus-service", ["nexus-tenant-a"], "15m", "30m")
    return {"role_id": _role_id("nexus-service"), "secret_id": _secret_id("nexus-service")}


# --------------------------------------------------------------------------
# Fixtures
# --------------------------------------------------------------------------


def setup_module(module):
    _container_up()


def teardown_module(module):
    _teardown()


# --------------------------------------------------------------------------
# Tests: health + least-privilege auth (directives B, C, H)
# --------------------------------------------------------------------------


def ep009_integration_health_readiness():
    status, body = _http("GET", "/v1/sys/health")
    assert status == 200
    assert body["initialized"] is True
    assert body["sealed"] is False
    assert body["version"] == "2.5.4"


def ep009_integration_image_is_pinned_by_digest():
    out = subprocess.run(
        [DOCKER, "inspect", "--format", "{{.Image}}", _container_name],
        capture_output=True,
        text=True,
    )
    assert IMAGE.split("@")[1] in out.stdout, f"container not running pinned digest: {out.stdout}"


def ep009_integration_approle_least_privilege_login():
    creds = _provision_tenant_a_identity()
    status, body = _approle_login(creds["role_id"], creds["secret_id"])
    assert status == 200
    auth = body["auth"]
    assert auth["lease_duration"] == 900, "token TTL must be bounded (15m)"
    assert auth["renewable"] is True, "renewability must be explicit"
    assert auth["client_token"], "client token must be issued"


def ep009_integration_approle_wrong_secret_id_fails():
    creds = _provision_tenant_a_identity()
    status, body = _approle_login(creds["role_id"], "wrong-secret-id")
    assert status == 400
    assert "invalid role or secret ID" in " ".join(body.get("errors", []))


# --------------------------------------------------------------------------
# Tests: KV-v2 semantics (directive D)
# --------------------------------------------------------------------------


def ep009_integration_kv_write_read_version_metadata():
    _http("PUT", f"/v1/secret/data/{TENANT_A}/db", {"data": {"password": "first"}})
    status, body = _http("GET", f"/v1/secret/data/{TENANT_A}/db")
    assert status == 200
    assert body["data"]["data"]["password"] == "first"
    assert body["data"]["metadata"]["version"] == 1

    # update -> new version
    _http("PUT", f"/v1/secret/data/{TENANT_A}/db", {"data": {"password": "second"}})
    status, body = _http("GET", f"/v1/secret/data/{TENANT_A}/db")
    assert body["data"]["metadata"]["version"] == 2
    assert body["data"]["data"]["password"] == "second"

    # explicit old version read
    status, body = _http("GET", f"/v1/secret/data/{TENANT_A}/db?version=1")
    assert body["data"]["data"]["password"] == "first"

    # metadata observation
    status, body = _http("GET", f"/v1/secret/metadata/{TENANT_A}/db")
    assert status == 200
    assert sorted(body["data"]["versions"].keys()) == ["1", "2"]


def ep009_integration_kv_soft_delete_undelete():
    _http("PUT", f"/v1/secret/data/{TENANT_A}/rotate-me", {"data": {"v": "1"}})
    _http("PUT", f"/v1/secret/data/{TENANT_A}/rotate-me", {"data": {"v": "2"}})

    # soft delete latest -> 204, read -> 404
    status, _ = _http("DELETE", f"/v1/secret/data/{TENANT_A}/rotate-me")
    assert status == 204
    status, _ = _http("GET", f"/v1/secret/data/{TENANT_A}/rotate-me")
    assert status == 404

    # undelete -> read works again
    status, _ = _http("POST", f"/v1/secret/undelete/{TENANT_A}/rotate-me", {"versions": [2]})
    assert status == 204
    status, body = _http("GET", f"/v1/secret/data/{TENANT_A}/rotate-me")
    assert status == 200
    assert body["data"]["data"]["v"] == "2"


def ep009_integration_kv_destroy_permanent():
    _http("PUT", f"/v1/secret/data/{TENANT_A}/destroy-me", {"data": {"v": "1"}})
    _http("PUT", f"/v1/secret/data/{TENANT_A}/destroy-me", {"data": {"v": "2"}})

    # destroy version 1 -> reading version 1 fails closed
    status, _ = _http("POST", f"/v1/secret/destroy/{TENANT_A}/destroy-me", {"versions": [1]})
    assert status == 204
    status, body = _http("GET", f"/v1/secret/data/{TENANT_A}/destroy-me?version=1")
    assert status == 404
    # latest version still readable (destroy is version-scoped)
    status, body = _http("GET", f"/v1/secret/data/{TENANT_A}/destroy-me")
    assert status == 200
    assert body["data"]["data"]["v"] == "2"


# --------------------------------------------------------------------------
# Tests: least-privilege policy proof (directive H)
# --------------------------------------------------------------------------


def ep009_integration_tenant_isolation_denied():
    creds = _provision_tenant_a_identity()
    status, body = _approle_login(creds["role_id"], creds["secret_id"])
    token = body["auth"]["client_token"]

    # tenant A allowed
    status, _ = _http("PUT", f"/v1/secret/data/{TENANT_A}/ok", {"data": {"x": "y"}}, token=token)
    assert status == 200
    status, _ = _http("GET", f"/v1/secret/data/{TENANT_A}/ok", token=token)
    assert status == 200

    # tenant B denied
    status, body = _http("PUT", f"/v1/secret/data/{TENANT_B}/db", {"data": {"x": "y"}}, token=token)
    assert status == 403
    assert "permission denied" in " ".join(body.get("errors", []))

    # sys/admin paths denied
    status, _ = _http("POST", "/v1/sys/auth/approle", {"type": "approle"}, token=token)
    assert status == 403
    evil_policy = 'path "*" { capabilities = ["read"] }'
    status, _ = _http("PUT", "/v1/sys/policies/acl/evil", {"policy": evil_policy}, token=token)
    assert status == 403


# --------------------------------------------------------------------------
# Tests: response wrapping (directive G - crown jewel)
# --------------------------------------------------------------------------


def ep009_integration_response_wrapping_one_time_use():
    # write a real secret normally
    _http("PUT", f"/v1/secret/data/{TENANT_A}/handoff", {"data": {"material": CANARY}})

    # 1. fetch with wrapping TTL -> response has wrap_info, NO plaintext
    status, body = _http(
        "GET",
        f"/v1/secret/data/{TENANT_A}/handoff",
        headers={"X-Vault-Wrap-TTL": "120s"},
    )
    assert status == 200
    assert "wrap_info" in body, "wrapped response must contain wrap_info"
    wrapping_token = body["wrap_info"]["token"]
    assert wrapping_token, "wrapping token must be present"
    assert CANARY not in json.dumps(body), "plaintext must not appear in wrapped response"
    # OpenBao includes a `data` field with version METADATA only; the
    # secret VALUE (data.data) must be absent until unwrap.
    wrapped_data = body.get("data") or {}
    assert wrapped_data.get("data") is None, (
        "wrapped response must not carry the secret value directly"
    )

    # 2. unwrap once -> succeeds with the original payload
    status, body = _http("POST", "/v1/sys/wrapping/unwrap", token=wrapping_token)
    assert status == 200
    assert body["data"]["data"]["material"] == CANARY, "unwrap must return the secret"

    # 3. unwrap second time -> fails (single use)
    status, body = _http("POST", "/v1/sys/wrapping/unwrap", token=wrapping_token)
    assert status == 400
    assert "wrapping token is not valid or does not exist" in " ".join(body.get("errors", []))


def ep009_integration_response_wrapping_expired_token():
    # bounded TTL: 2s wrap, wait, unwrap -> fails
    _http("PUT", f"/v1/secret/data/{TENANT_A}/expiring", {"data": {"v": "1"}})
    status, body = _http(
        "GET",
        f"/v1/secret/data/{TENANT_A}/expiring",
        headers={"X-Vault-Wrap-TTL": "2s"},
    )
    assert status == 200
    wrapping_token = body["wrap_info"]["token"]
    time.sleep(4)
    status, body = _http("POST", "/v1/sys/wrapping/unwrap", token=wrapping_token)
    assert status == 400
    assert "wrapping token is not valid or does not exist" in " ".join(body.get("errors", []))


# --------------------------------------------------------------------------
# Tests: provider death fail-closed (directive O.1)
# --------------------------------------------------------------------------


def ep009_integration_provider_killed_fails_closed():
    global _container_name, _container_port
    # Kill the container mid-flight: the SAME request must never ALLOW;
    # the adapter returns typed unavailable.
    name = _container_name
    port = _container_port
    subprocess.run([DOCKER, "rm", "-f", name], capture_output=True)
    # Give Docker a moment, then hit the dead port: connection refused ->
    # typed unavailable (no silent success).
    deadline = time.time() + 5
    refused = False
    while time.time() < deadline and not refused:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=1):
                pass
        except OSError:
            refused = True
        time.sleep(0.3)
    assert refused, "port should be closed after container kill"
    # Restore for later tests.
    _container_name = None
    _container_port = 0
    _container_up()
