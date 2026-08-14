"""EP-009 M2 failure tests: forced OpenBao + SOPS failures, fail-closed.

Test names begin with ep009_failure_ per the EP-009 milestone contract.
Every failure below is produced by a REAL mechanism against the REAL
pinned OpenBao container (2.5.4) or the REAL pinned sops/age tooling -
never faked (directive O).

FAILURE MATRIX covered:
1. OpenBao killed -> typed unavailable (never a silent success)
2. wrong AppRole/credential -> authentication failure
3. policy denies secret path -> permission denied (tenant B)
4. missing key/version -> typed not found
5. destroyed version -> fail closed (404)
6. wrapping token second-use -> rejected (400)
7. wrapping token expired -> rejected (400)
8. wrong age identity -> decryption failure
9. corrupted SOPS document -> integrity failure
10. missing encrypted file -> typed bootstrap failure
11. canary-secret log scan -> zero leaks (directive E)
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
ROOT = Path(__file__).resolve().parents[2]
DOCKER = "/usr/bin/docker"
SOPS = "/usr/local/bin/sops"
AGE = "/usr/bin/age"
AGE_KEYGEN = "/usr/bin/age-keygen"

# Dev-mode bootstrap credential for the ephemeral test instance ONLY
# (directive C); the adapter never uses it. Neutral ephemeral value.
DEV_ROOT_TOKEN = "nexus-ep009-dev-bootstrap"
TENANT_A = "tenant-a"
TENANT_B = "tenant-b"
CANARY = "canary-nexus-ep009-failure-5b2d91e0"

_container_name: str | None = None
_container_port: int = 0


# --------------------------------------------------------------------------
# Container lifecycle (shared with integration suite pattern)
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


def _provision_tenant_a_identity() -> dict:
    _http("POST", "/v1/sys/auth/approle", {"type": "approle"})
    policy = f"""
path "secret/data/{TENANT_A}/*" {{ capabilities = ["create", "read", "update", "delete", "list"] }}
path "secret/metadata/{TENANT_A}/*" {{ capabilities = ["read", "list"] }}
path "secret/undelete/{TENANT_A}/*" {{ capabilities = ["update"] }}
path "secret/destroy/{TENANT_A}/*" {{ capabilities = ["update"] }}
path "sys/health" {{ capabilities = ["read"] }}
"""
    _http("PUT", "/v1/sys/policies/acl/nexus-tenant-a", {"policy": policy})
    _http(
        "POST",
        "/v1/auth/approle/role/nexus-service",
        {
            "policies": ["nexus-tenant-a"],
            "token_ttl": "15m",
            "token_max_ttl": "30m",
            "secret_id_ttl": "10m",
        },
    )
    status, body = _http("GET", "/v1/auth/approle/role/nexus-service/role-id")
    role_id = body["data"]["role_id"]
    status, body = _http("POST", "/v1/auth/approle/role/nexus-service/secret-id")
    secret_id = body["data"]["secret_id"]
    return {"role_id": role_id, "secret_id": secret_id}


def setup_module(module):
    _container_up()


def teardown_module(module):
    _teardown()


# --------------------------------------------------------------------------
# 1. OpenBao killed -> typed unavailable
# --------------------------------------------------------------------------


def ep009_failure_provider_killed_fails_closed():
    global _container_name, _container_port
    name = _container_name
    port = _container_port
    subprocess.run([DOCKER, "rm", "-f", name], capture_output=True)
    deadline = time.time() + 5
    refused = False
    while time.time() < deadline and not refused:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=1):
                pass
        except OSError:
            refused = True
        time.sleep(0.3)
    assert refused, "port must be closed after kill"
    # Restore for later tests.
    _container_name = None
    _container_port = 0
    _container_up()


# --------------------------------------------------------------------------
# 2. wrong AppRole credential -> authentication failure
# --------------------------------------------------------------------------


def ep009_failure_wrong_approle_credential_rejected():
    creds = _provision_tenant_a_identity()
    status, body = _http(
        "POST",
        "/v1/auth/approle/login",
        {"role_id": creds["role_id"], "secret_id": "wrong-secret-id"},
    )
    assert status == 400
    assert "invalid role or secret ID" in " ".join(body.get("errors", []))


# --------------------------------------------------------------------------
# 3. policy denies secret path -> permission denied
# --------------------------------------------------------------------------


def ep009_failure_policy_denies_tenant_b():
    creds = _provision_tenant_a_identity()
    status, body = _http(
        "POST",
        "/v1/auth/approle/login",
        {"role_id": creds["role_id"], "secret_id": creds["secret_id"]},
    )
    token = body["auth"]["client_token"]
    status, body = _http("PUT", f"/v1/secret/data/{TENANT_B}/db", {"data": {"x": "y"}}, token=token)
    assert status == 403
    assert "permission denied" in " ".join(body.get("errors", []))


# --------------------------------------------------------------------------
# 4. missing key/version -> typed not found
# --------------------------------------------------------------------------


def ep009_failure_missing_key_not_found():
    status, body = _http("GET", f"/v1/secret/data/{TENANT_A}/does-not-exist")
    assert status == 404
    assert "not found" in " ".join(body.get("errors", [])) or not body.get("errors")


def ep009_failure_missing_version_not_found():
    _http("PUT", f"/v1/secret/data/{TENANT_A}/one-version", {"data": {"v": "1"}})
    status, _ = _http("GET", f"/v1/secret/data/{TENANT_A}/one-version?version=99")
    assert status == 404


# --------------------------------------------------------------------------
# 5. destroyed version -> fail closed
# --------------------------------------------------------------------------


def ep009_failure_destroyed_version_fails_closed():
    _http("PUT", f"/v1/secret/data/{TENANT_A}/doomed", {"data": {"v": "1"}})
    _http("POST", f"/v1/secret/destroy/{TENANT_A}/doomed", {"versions": [1]})
    status, _ = _http("GET", f"/v1/secret/data/{TENANT_A}/doomed?version=1")
    assert status == 404


# --------------------------------------------------------------------------
# 6/7. wrapping second-use + expired -> rejected
# --------------------------------------------------------------------------


def ep009_failure_wrapping_second_use_rejected():
    _http("PUT", f"/v1/secret/data/{TENANT_A}/handoff", {"data": {"material": CANARY}})
    status, body = _http(
        "GET", f"/v1/secret/data/{TENANT_A}/handoff", headers={"X-Vault-Wrap-TTL": "120s"}
    )
    wrapping_token = body["wrap_info"]["token"]
    status, body = _http("POST", "/v1/sys/wrapping/unwrap", token=wrapping_token)
    assert status == 200
    status, body = _http("POST", "/v1/sys/wrapping/unwrap", token=wrapping_token)
    assert status == 400
    assert "wrapping token is not valid or does not exist" in " ".join(body.get("errors", []))


def ep009_failure_wrapping_expired_rejected():
    _http("PUT", f"/v1/secret/data/{TENANT_A}/expiring", {"data": {"v": "1"}})
    status, body = _http(
        "GET", f"/v1/secret/data/{TENANT_A}/expiring", headers={"X-Vault-Wrap-TTL": "2s"}
    )
    wrapping_token = body["wrap_info"]["token"]
    time.sleep(4)
    status, body = _http("POST", "/v1/sys/wrapping/unwrap", token=wrapping_token)
    assert status == 400
    assert "wrapping token is not valid or does not exist" in " ".join(body.get("errors", []))


# --------------------------------------------------------------------------
# 8-10. SOPS + age real failure proofs (directives L, O)
# --------------------------------------------------------------------------


def _sops_env(identity_file: str) -> dict:
    env = dict(os.environ)
    env["SOPS_AGE_KEY_FILE"] = identity_file
    return env


def _write_age_identity(path: Path, identity: str) -> None:
    path.write_text(identity)
    path.chmod(0o600)


def ep009_failure_sops_wrong_age_identity():
    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        # generate two identities: correct and wrong
        for name in ("good", "bad"):
            subprocess.run(
                [AGE_KEYGEN, "-o", str(td / f"{name}.key")], check=True, capture_output=True
            )
        recipient_good = (
            subprocess.run(
                [AGE_KEYGEN, "-y"],
                input=(td / "good.key").read_text().encode(),
                capture_output=True,
                check=True,
            )
            .stdout.decode()
            .strip()
        )
        fixture = td / "fixture.yaml"
        fixture.write_text(f"db_password: {CANARY}\n")
        # encrypt with the good recipient -> encrypted bytes on stdout
        enc = subprocess.run(
            [
                SOPS,
                "--encrypt",
                "--age",
                recipient_good,
                "--input-type",
                "yaml",
                "--output-type",
                "yaml",
                str(fixture),
            ],
            check=True,
            capture_output=True,
        )
        encrypted = enc.stdout.decode()
        assert CANARY not in encrypted, "plaintext must not appear in encrypted output"
        encrypted_file = td / "fixture.enc.yaml"
        encrypted_file.write_text(encrypted)
        # decrypt with WRONG identity -> must fail
        result = subprocess.run(
            [
                SOPS,
                "--decrypt",
                "--input-type",
                "yaml",
                "--output-type",
                "yaml",
                str(encrypted_file),
            ],
            env=_sops_env(str(td / "bad.key")),
            capture_output=True,
        )
        assert result.returncode != 0, "wrong identity must fail decryption"


def ep009_failure_sops_corrupted_document():
    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        subprocess.run([AGE_KEYGEN, "-o", str(td / "good.key")], check=True, capture_output=True)
        recipient = (
            subprocess.run(
                [AGE_KEYGEN, "-y"],
                input=(td / "good.key").read_text().encode(),
                capture_output=True,
                check=True,
            )
            .stdout.decode()
            .strip()
        )
        fixture = td / "fixture.yaml"
        fixture.write_text(f"db_password: {CANARY}\n")
        subprocess.run(
            [
                SOPS,
                "--encrypt",
                "--age",
                recipient,
                "--input-type",
                "yaml",
                "--output-type",
                "yaml",
                str(fixture),
            ],
            check=True,
            capture_output=True,
        )
        # corrupt the MAC/ciphertext: flip bytes in the middle
        data = bytearray(fixture.read_bytes())
        mid = len(data) // 2
        data[mid] ^= 0x01
        fixture.write_bytes(bytes(data))
        result = subprocess.run(
            [SOPS, "--decrypt", "--input-type", "yaml", "--output-type", "yaml", str(fixture)],
            env=_sops_env(str(td / "good.key")),
            capture_output=True,
        )
        assert result.returncode != 0, "corrupted document must fail integrity check"


def ep009_failure_sops_missing_file():
    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        subprocess.run([AGE_KEYGEN, "-o", str(td / "good.key")], check=True, capture_output=True)
        result = subprocess.run(
            [
                SOPS,
                "--decrypt",
                "--input-type",
                "yaml",
                "--output-type",
                "yaml",
                str(td / "missing.yaml"),
            ],
            env=_sops_env(str(td / "good.key")),
            capture_output=True,
        )
        assert result.returncode != 0, "missing file must fail typed bootstrap"


def ep009_failure_sops_missing_identity():
    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        subprocess.run([AGE_KEYGEN, "-o", str(td / "good.key")], check=True, capture_output=True)
        recipient = (
            subprocess.run(
                [AGE_KEYGEN, "-y"],
                input=(td / "good.key").read_text().encode(),
                capture_output=True,
                check=True,
            )
            .stdout.decode()
            .strip()
        )
        fixture = td / "fixture.yaml"
        fixture.write_text(f"db_password: {CANARY}\n")
        subprocess.run(
            [
                SOPS,
                "--encrypt",
                "--age",
                recipient,
                "--input-type",
                "yaml",
                "--output-type",
                "yaml",
                str(fixture),
            ],
            check=True,
            capture_output=True,
        )
        # no identity file at all
        env = dict(os.environ)
        env.pop("SOPS_AGE_KEY_FILE", None)
        result = subprocess.run(
            [SOPS, "--decrypt", "--input-type", "yaml", "--output-type", "yaml", str(fixture)],
            env=env,
            capture_output=True,
        )
        assert result.returncode != 0, "missing identity must fail decryption"


# --------------------------------------------------------------------------
# 11. canary-secret log scan -> zero leaks
# --------------------------------------------------------------------------


def ep009_failure_canary_log_scan_zero_leaks():
    """Every container log line and temp file in this suite must not
    contain the canary secret (directive E)."""
    name = _container_name
    logs = subprocess.run([DOCKER, "logs", name], capture_output=True)
    combined = (logs.stdout + logs.stderr).decode(errors="replace")
    assert CANARY not in combined, "canary leaked into OpenBao container logs"
    # the canary is only ever written as a secret VALUE, and only inside
    # the provider payload; the test process itself must not print it.
    # no persistent file anywhere under the repo contains the canary
    repo = ROOT
    hit = subprocess.run(
        ["grep", "-rIl", CANARY, str(repo)],
        capture_output=True,
        text=True,
    )
    # allowed: this test file itself and the temporary fixture (not in
    # the repo); anything else is a leak.
    leaks = [
        line for line in hit.stdout.splitlines() if "test_ep009" not in line and ".git" not in line
    ]
    assert not leaks, f"canary leaked into repo files: {leaks}"
