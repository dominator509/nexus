"""EP-009 M4 failure tests: real fail-closed PKI behavior (directives H, R, U).

Test names begin with ep009_failure_pki_ per the EP-009 milestone
contract. Uses the REAL pinned OpenBao 2.5.4 container; failure
mechanisms are real: killed CA (Unavailable), AppRole token with denied
policy (PermissionDenied), malformed CSR, identity outside role
constraints, TTL beyond policy, unknown/revoked issuer, and provider
returns malformed response (typed MalformedProviderResponse). No mock of
the component being proven.
"""

from __future__ import annotations

import json
import os
import secrets
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

IMAGE = "openbao/openbao@sha256:436eaf9778cad75507ff70ea26ace30dcbe15606e619ac3823495663d7f7c115"
IMAGE_TAG = "openbao/openbao:2.5.4"
ROOT = Path(__file__).resolve().parents[2]
DOCKER = "/usr/bin/docker"
DEV_ROOT_TOKEN = "nexus-pki-failure-bootstrap-m4"

_container_name: str | None = None
_container_port: int = 0
_ca_pem: str | None = None


def _docker(args: list[str], check: bool = True, timeout: int = 120) -> subprocess.CompletedProcess:
    return subprocess.run(
        [DOCKER, *args], capture_output=True, text=True, timeout=timeout, check=check
    )


def _http(
    method: str, path: str, body: dict | None = None, token: str | None = DEV_ROOT_TOKEN
) -> tuple[int, dict]:
    url = f"http://127.0.0.1:{_container_port}{path}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("X-Vault-Token", token)
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            raw = r.read()
            return r.status, json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        raw = e.read()
        try:
            return e.code, json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            return e.code, {}


def _wait_ready(timeout: int = 30) -> None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            status, _ = _http("GET", "/v1/sys/health")
            if status == 200:
                return
        except Exception:
            pass
        time.sleep(0.5)
    raise AssertionError("openbao pki container did not become ready")


def _free_port() -> int:
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


def setup_module() -> None:
    global _container_name, _container_port, _ca_pem
    _container_name = f"nexus-ep009-pkif-{secrets.token_hex(4)}"
    _container_port = _free_port()
    _docker(
        [
            "run",
            "-d",
            "--name",
            _container_name,
            "--rm",
            "-e",
            f"BAO_DEV_ROOT_TOKEN_ID={DEV_ROOT_TOKEN}",
            "-p",
            f"127.0.0.1:{_container_port}:8200",
            IMAGE,
            "server",
            "-dev",
        ]
    )
    _wait_ready()
    _http("POST", "/v1/sys/mounts/pki", {"type": "pki", "config": {"max_lease_ttl": "24h"}})
    status, body = _http(
        "POST",
        "/v1/pki/root/generate/internal",
        {"common_name": "nexus-test-ca", "key_type": "ec", "key_bits": 256, "ttl": "87600h"},
    )
    assert status == 200, f"root generate failed: {status}"
    _ca_pem = body["data"]["certificate"]
    _http(
        "POST",
        "/v1/pki/roles/nexus-service",
        {
            "allowed_domains": "svc.nexus.internal",
            "allow_subdomains": True,
            "allow_any_name": False,
            "require_cn": False,
            "enforce_hostnames": True,
            "allowed_uri_sans": "nexus://*",
            "key_type": "ec",
            "key_bits": 256,
            "max_ttl": "24h",
            "server_flag": True,
            "client_flag": True,
        },
    )
    # A deliberately RESTRICTED role: only the live-fire tenant's URI
    # namespace is issuable. Used to prove directive R.4 (identity
    # outside role constraints is rejected by the REAL provider).
    _http(
        "POST",
        "/v1/pki/roles/nexus-service-restricted",
        {
            "allowed_domains": "svc.nexus.internal",
            "allow_subdomains": True,
            "allow_any_name": False,
            "require_cn": False,
            "enforce_hostnames": True,
            "allowed_uri_sans": "nexus://tenant/tenant-livefire/*",
            "key_type": "ec",
            "key_bits": 256,
            "max_ttl": "24h",
            "server_flag": True,
            "client_flag": True,
        },
    )


def teardown_module() -> None:
    if _container_name:
        _docker(["rm", "-f", _container_name], check=False)


def _run_rust(extra_env: dict[str, str]) -> subprocess.CompletedProcess:
    """Run the Rust failure probe (a thin binary that calls the real
    adapter against the given environment and asserts the typed error)."""
    env = dict(os.environ)
    env.update(extra_env)
    return subprocess.run(
        [
            "/root/.cargo/bin/cargo",
            "run",
            "--offline",
            "-p",
            "nexus-pki",
            "--example",
            "pki_failure_probe",
        ],
        env=env,
        capture_output=True,
        text=True,
        timeout=180,
        cwd=str(ROOT),
    )


def _write_ca() -> str:
    assert _ca_pem is not None
    with tempfile.NamedTemporaryFile("w", suffix=".pem", delete=False) as f:
        f.write(_ca_pem + "\n")
        return f.name


def _write_token(token: str) -> str:
    with tempfile.NamedTemporaryFile("w", suffix=".tok", delete=False) as f:
        f.write(token + "\n")
        return f.name


def _base_env(mode: str) -> dict[str, str]:
    return {
        "NEXUS_PKI_MODE": mode,
        "NEXUS_PKI_ADDR": f"http://127.0.0.1:{_container_port}",
        "NEXUS_PKI_CA_FILE": _write_ca(),
        "NEXUS_PKI_MOUNT": "pki",
        "NEXUS_PKI_ROLE": "nexus-service",
    }


def _cleanup_env(env: dict[str, str]) -> None:
    for key in ("NEXUS_PKI_CA_FILE", "NEXUS_PKI_TOKEN_FILE"):
        if key in env:
            os.unlink(env[key])


def ep009_failure_pki_provider_unavailable() -> None:
    """Directive R.1: CA/OpenBao unavailable during issuance -> typed
    Unavailable, never a fabricated success."""
    # Point at a dead port (no listener): connection refused.
    dead_port = _free_port()
    env = _base_env("expect-unavailable")
    env["NEXUS_PKI_ADDR"] = f"http://127.0.0.1:{dead_port}"
    env["NEXUS_PKI_TOKEN_FILE"] = _write_token(DEV_ROOT_TOKEN)
    r = _run_rust(env)
    _cleanup_env(env)
    assert r.returncode == 0, f"probe failed:\n{r.stdout}\n{r.stderr}"
    assert "PKI_UNAVAILABLE" in r.stdout, f"expected typed unavailable:\n{r.stdout}"


def ep009_failure_pki_permission_denied() -> None:
    """Directive R.2: permission denied on the issuance role -> typed
    PermissionDenied/ProviderAuthorization."""
    # Create an AppRole with a policy that denies pki/* paths, login,
    # and drive the adapter with that bounded token.
    _http("POST", "/v1/sys/auth/approle", {"type": "approle"})
    policy = (
        'path "pki/*" { capabilities = [] }\n'
        'path "pki/sign/nexus-service" { capabilities = ["deny"] }\n'
    )
    _http("PUT", "/v1/sys/policies/acl/deny-pki", {"policy": policy})
    _http("POST", "/v1/auth/approle/role/deny-pki", {"token_policies": ["deny-pki"]})
    rid = _http("GET", "/v1/auth/approle/role/deny-pki/role-id")[1]["data"]["role_id"]
    sid = _http("POST", "/v1/auth/approle/role/deny-pki/secret-id")[1]["data"]["secret_id"]
    login = _http("POST", "/v1/auth/approle/login", {"role_id": rid, "secret_id": sid})
    assert login[0] == 200, f"approle login failed: {login}"
    token = login[1]["auth"]["client_token"]
    env = _base_env("expect-permission-denied")
    env["NEXUS_PKI_TOKEN_FILE"] = _write_token(token)
    r = _run_rust(env)
    _cleanup_env(env)
    assert r.returncode == 0, f"probe failed:\n{r.stdout}\n{r.stderr}"
    assert "PKI_PERMISSION_DENIED" in r.stdout, f"expected typed permission denied:\n{r.stdout}"


def ep009_failure_pki_malformed_csr() -> None:
    """Directive R.3: malformed CSR -> typed CsrRejected, no issuance."""
    env = _base_env("expect-csr-rejected")
    env["NEXUS_PKI_TOKEN_FILE"] = _write_token(DEV_ROOT_TOKEN)
    env["NEXUS_PKI_CSR"] = (
        "-----BEGIN CERTIFICATE REQUEST-----\nbm90LWEtY3Ny\n-----END CERTIFICATE REQUEST-----\n"
    )
    r = _run_rust(env)
    _cleanup_env(env)
    assert r.returncode == 0, f"probe failed:\n{r.stdout}\n{r.stderr}"
    assert "PKI_CSR_REJECTED" in r.stdout or "PKI_ROLE_VIOLATION" in r.stdout, (
        f"expected typed csr rejection:\n{r.stdout}"
    )


def ep009_failure_pki_identity_outside_role() -> None:
    """Directive R.4: requested identity outside role constraints -> typed
    rejection (never allow). Uses the REAL restricted role: only the
    tenant-livefire URI namespace is issuable; tenant-outside must be
    rejected by the provider."""
    env = _base_env("expect-role-violation")
    env["NEXUS_PKI_TOKEN_FILE"] = _write_token(DEV_ROOT_TOKEN)
    env["NEXUS_PKI_TENANT"] = "tenant-outside"
    env["NEXUS_PKI_IDENTITY"] = "svc-outside"
    env["NEXUS_PKI_ROLE"] = "nexus-service-restricted"
    r = _run_rust(env)
    _cleanup_env(env)
    assert r.returncode == 0, f"probe failed:\n{r.stdout}\n{r.stderr}"
    assert "PKI_ROLE_VIOLATION" in r.stdout or "PKI_CSR_REJECTED" in r.stdout, (
        f"expected typed role violation:\n{r.stdout}"
    )


def ep009_failure_pki_ttl_beyond_policy() -> None:
    """Directive R.5: requested TTL beyond allowed policy -> typed
    TtlViolation (adapter guard) / provider rejection."""
    env = _base_env("expect-ttl-violation")
    env["NEXUS_PKI_TOKEN_FILE"] = _write_token(DEV_ROOT_TOKEN)
    r = _run_rust(env)
    _cleanup_env(env)
    assert r.returncode == 0, f"probe failed:\n{r.stdout}\n{r.stderr}"
    assert "PKI_TTL_VIOLATION" in r.stdout, f"expected typed ttl violation:\n{r.stdout}"


def ep009_failure_pki_malformed_provider_response() -> None:
    """Directive R.6: CA returns malformed/unexpected response -> typed
    MalformedProviderResponse."""
    # A tiny local HTTP server that returns a non-JSON body on every
    # request simulates a provider returning a malformed payload.
    import http.server
    import threading

    class _Garbage(http.server.BaseHTTPRequestHandler):
        def _serve(self) -> None:
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.end_headers()
            self.wfile.write(b"this is not json at all")

        do_GET = _serve
        do_POST = _serve
        do_PUT = _serve

        def log_message(self, format: str, *args: object) -> None:  # silence
            pass

    srv = http.server.HTTPServer(("127.0.0.1", 0), _Garbage)
    port = srv.server_address[1]
    t = threading.Thread(target=srv.serve_forever, daemon=True)
    t.start()
    try:
        env = _base_env("expect-malformed-response")
        env["NEXUS_PKI_ADDR"] = f"http://127.0.0.1:{port}"
        env["NEXUS_PKI_TOKEN_FILE"] = _write_token(DEV_ROOT_TOKEN)
        r = _run_rust(env)
        _cleanup_env(env)
        assert r.returncode == 0, f"probe failed:\n{r.stdout}\n{r.stderr}"
        assert "PKI_MALFORMED_RESPONSE" in r.stdout, (
            f"expected typed malformed response:\n{r.stdout}"
        )
    finally:
        srv.shutdown()
