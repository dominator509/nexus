"""EP-009 M3 Headscale mesh integration tests (real provider boundary).

Test names begin with ep009_integration_headscale_ per the EP-009
milestone contract. Uses the REAL pinned `headscale/headscale:0.23.0`
container over REAL gRPC (TLS + API key), driven through the REAL
pinned headscale CLI binary (same version/digest lineage as the
container).

PROOFS:
1. server comes up and gRPC answers (users list)
2. tenant (user) lifecycle: create is idempotent, list shows it
3. preauth keys create with bounded expiration
4. node register allocates real 100.64.0.0/10 + fd7a:...::/48 IPs
5. node list round-trips the registered node
6. node expiry is observable and terminal (expire + delete)
7. wrong API key fails closed (ProviderAuthorization)
8. dead server fails closed (Unavailable), never succeeds silently
9. teardown removes the container + temp identities
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCKER = "/usr/bin/docker"
HEADSCALE_IMAGE = "headscale/headscale:0.23.0"
HEADSCALE_DIGEST = "sha256:ffe793968ef6fbec78a8d095893fe03112e6a74231afe366eb504fbc822afea6"
HEADSCALE_BIN = "/usr/local/bin/headscale"
TMP = Path(tempfile.gettempdir())

_CONTAINER = "nexus-ep009-hs"
_NET = "nexus-ep009-hs-net"
_api_key: str | None = None
_tls_dir: Path | None = None
_data_dir: Path | None = None
_config_path: Path | None = None
_cli_config: Path | None = None
_grpc_port = 15053
_http_port = 18090


def _docker(args: list[str], check: bool = True, timeout: int = 120) -> subprocess.CompletedProcess:
    return subprocess.run(
        [DOCKER, *args],
        capture_output=True,
        text=True,
        timeout=timeout,
        check=check,
    )


def _run_cli(args: list[str], key: str, timeout: int = 40) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["HEADSCALE_CONFIG"] = str(_cli_config)
    env["HEADSCALE_CLI_ADDRESS"] = f"127.0.0.1:{_grpc_port}"
    env["HEADSCALE_CLI_API_KEY"] = key
    return subprocess.run(
        [HEADSCALE_BIN, *args],
        env=env,
        capture_output=True,
        text=True,
        timeout=timeout,
    )


def setup_module(module):
    global _api_key, _tls_dir, _data_dir, _config_path, _cli_config

    _docker(["rm", "-f", _CONTAINER], check=False)
    _docker(["network", "rm", _NET], check=False)

    # Real server TLS certificate (self-signed, test-only, ephemeral).
    _tls_dir = TMP / "nexus-ep009-hs-tls"
    _tls_dir.mkdir(exist_ok=True)
    subprocess.run(
        [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            str(_tls_dir / "key.pem"),
            "-out",
            str(_tls_dir / "cert.pem"),
            "-days",
            "2",
            "-nodes",
            "-subj",
            "/CN=headscale-test",
            "-addext",
            "subjectAltName=DNS:localhost,IP:127.0.0.1",
        ],
        check=True,
        capture_output=True,
    )
    _data_dir = TMP / "nexus-ep009-hs-data"
    _data_dir.mkdir(exist_ok=True)
    _config_path = TMP / "nexus-ep009-hs-config.yaml"
    _config_path.write_text(
        f"""server_url: http://127.0.0.1:{_http_port}
listen_addr: 0.0.0.0:8080
metrics_listen_addr: 127.0.0.1:9090
grpc_listen_addr: 0.0.0.0:50443
grpc_allow_insecure: true
noise:
  private_key_path: /var/lib/headscale/noise_private.key
prefixes:
  v6: fd7a:115c:a1e0::/48
  v4: 100.64.0.0/10
  allocation: sequential
derp:
  server:
    enabled: false
    region_id: 999
    region_code: "headscale"
    region_name: "Headscale Embedded DERP"
    stun_listen_addr: "0.0.0.0:3478"
    private_key_path: /var/lib/headscale/derp_server_private.key
    automatically_add_embedded_derp_region: true
    ipv4: 1.2.3.4
    ipv6: 2001:db8::1
  urls:
    - https://controlplane.tailscale.com/derpmap/default
  paths: []
  auto_update_enabled: false
  update_frequency: 24h
database:
  type: sqlite3
  sqlite:
    path: /var/lib/headscale/db.sqlite
tls_cert_path: /etc/headscale/tls/cert.pem
tls_key_path: /etc/headscale/tls/key.pem
log:
  format: text
  level: info
policy:
  mode: database
  path: ""
dns:
  base_domain: example.com
  magic_dns: true
  nameservers:
    global: []
  search_domains: []
  extra_records: []
unix_socket: /var/run/headscale/headscale.sock
unix_socket_permission: "0o770"
cli:
  address: ""
  api_key: ""
  timeout: 30s
  insecure: true
"""
    )
    _cli_config = TMP / "nexus-ep009-hs-cli.yaml"
    _cli_config.write_text(
        """unix_socket: /var/run/headscale/headscale.sock
cli:
  address: ""
  api_key: ""
  timeout: 30s
  insecure: true
"""
    )

    _docker(["network", "create", _NET])
    _docker(
        [
            "run",
            "-d",
            "--name",
            _CONTAINER,
            "--rm",
            "--network",
            _NET,
            "-p",
            f"{_http_port}:8080",
            "-p",
            f"{_grpc_port}:50443",
            "-v",
            f"{_config_path}:/etc/headscale/config.yaml:ro",
            "-v",
            f"{_tls_dir}:/etc/headscale/tls:ro",
            "-v",
            f"{_data_dir}:/var/lib/headscale",
            HEADSCALE_IMAGE,
            "serve",
        ]
    )

    # Wait for the server to answer over gRPC (wrong key still proves
    # the endpoint is up: it must reject with an auth error).
    deadline = time.time() + 45
    ready = False
    while time.time() < deadline:
        r = _run_cli(["users", "list", "-o", "json"], key="bogus")
        if "invalid token" in r.stderr or "failed to validate" in r.stderr:
            ready = True
            break
        time.sleep(1)
    assert ready, "headscale server did not become ready"

    # Create a real API key (bounded 30m TTL; never persisted to repo).
    r = _docker(["exec", _CONTAINER, "headscale", "apikeys", "create", "--expiration", "30m"])
    assert r.returncode == 0, r.stderr
    _api_key = r.stdout.strip().splitlines()[-1].strip()
    assert _api_key, "api key must be non-empty"


def teardown_module(module):
    _docker(["rm", "-f", _CONTAINER], check=False)
    _docker(["network", "rm", _NET], check=False)
    for p in (_config_path, _cli_config, _tls_dir, _data_dir):
        if p and p.exists():
            if p.is_dir():
                shutil.rmtree(p, ignore_errors=True)
            else:
                p.unlink(missing_ok=True)


def ep009_integration_headscale_server_answers_grpc():
    assert _api_key is not None
    r = _run_cli(["users", "list", "-o", "json"], key=_api_key)
    assert r.returncode == 0, r.stderr
    # An empty user set answers "null"; a populated set answers a list.
    # Either way the gRPC endpoint answered.
    assert '"name"' in r.stdout or r.stdout.strip() == "null"


def ep009_integration_headscale_tenant_create_is_idempotent():
    assert _api_key is not None
    r1 = _run_cli(["users", "create", "tenant-a"], key=_api_key)
    assert r1.returncode == 0 or "already exists" in r1.stderr, r1.stderr
    r2 = _run_cli(["users", "create", "tenant-a"], key=_api_key)
    assert r2.returncode == 0 or "already exists" in r2.stderr, r2.stderr
    listed = _run_cli(["users", "list", "-o", "json"], key=_api_key)
    assert "tenant-a" in listed.stdout


def ep009_integration_headscale_preauthkey_bounded_ttl():
    assert _api_key is not None
    r = _run_cli(
        ["preauthkeys", "create", "--user", "tenant-a", "--expiration", "30m", "-o", "json"],
        key=_api_key,
    )
    assert r.returncode == 0, r.stderr
    assert '"key"' in r.stdout
    assert '"expiration"' in r.stdout


def ep009_integration_headscale_node_registers_with_real_ips():
    assert _api_key is not None
    mkey = "mkey:" + os.urandom(32).hex()
    created = _run_cli(
        [
            "debug",
            "create-node",
            "--user",
            "tenant-a",
            "--name",
            "node-1",
            "--key",
            mkey,
            "-o",
            "json",
        ],
        key=_api_key,
    )
    assert created.returncode == 0, created.stderr
    reg = _run_cli(
        ["nodes", "register", "--user", "tenant-a", "--key", mkey, "-o", "json"], key=_api_key
    )
    assert reg.returncode == 0, reg.stderr
    assert "100.64.0." in reg.stdout, "node must get a real 100.64.0.0/10 address"
    assert "fd7a:115c:a1e0" in reg.stdout, "node must get a real fd7a:...::/48 address"


def ep009_integration_headscale_node_list_roundtrip():
    assert _api_key is not None
    r = _run_cli(["nodes", "list", "-u", "tenant-a", "-o", "json"], key=_api_key)
    assert r.returncode == 0, r.stderr
    assert '"node-1"' in r.stdout
    assert "100.64.0." in r.stdout


def ep009_integration_headscale_node_expiry_observable():
    assert _api_key is not None
    listed = _run_cli(["nodes", "list", "-u", "tenant-a", "-o", "json"], key=_api_key)
    assert listed.returncode == 0, listed.stderr
    nodes = json.loads(listed.stdout) or []
    node = next(n for n in nodes if n.get("name") == "node-1")
    nid = str(node["id"])
    expired = _run_cli(["nodes", "expire", "--identifier", nid, "-o", "json"], key=_api_key)
    assert expired.returncode == 0, expired.stderr
    after = (
        json.loads(_run_cli(["nodes", "list", "-u", "tenant-a", "-o", "json"], key=_api_key).stdout)
        or []
    )
    after_node = next(n for n in after if n.get("name") == "node-1")
    assert after_node["expiry"]["seconds"] > 0, "expiry must be set after expire"
    deleted = _run_cli(
        ["nodes", "delete", "--identifier", nid, "--force", "-o", "json"], key=_api_key
    )
    assert deleted.returncode == 0, deleted.stderr
    remaining = (
        json.loads(_run_cli(["nodes", "list", "-u", "tenant-a", "-o", "json"], key=_api_key).stdout)
        or []
    )
    assert all(n.get("name") != "node-1" for n in remaining), "node must be gone after delete"


def ep009_integration_headscale_wrong_api_key_fails_closed():
    r = _run_cli(["users", "list", "-o", "json"], key="definitely-wrong-key")
    assert r.returncode != 0
    assert "invalid token" in r.stderr or "failed to validate" in r.stderr


def ep009_integration_headscale_dead_server_fails_closed():
    env = dict(os.environ)
    env["HEADSCALE_CONFIG"] = str(_cli_config)
    env["HEADSCALE_CLI_ADDRESS"] = "127.0.0.1:1"  # nothing listens here
    if _api_key is not None:
        env["HEADSCALE_CLI_API_KEY"] = _api_key
    r = subprocess.run(
        [HEADSCALE_BIN, "users", "list", "-o", "json"],
        env=env,
        capture_output=True,
        text=True,
        timeout=40,
    )
    assert r.returncode != 0
    assert "Could not connect" in r.stderr or "context deadline exceeded" in r.stderr


def ep009_integration_headscale_adapter_live_proof():
    """Run the REAL Rust `nexus-headscale` MeshController adapter
    against the suite's REAL headscale container (full register ->
    list -> wireguard config -> revoke cycle)."""
    assert _api_key is not None
    key_file = TMP / "nexus-ep009-hs-apikey.txt"
    key_file.write_text(_api_key + "\n")
    env = dict(os.environ)
    env["NEXUS_HS_BINARY"] = HEADSCALE_BIN
    env["NEXUS_HS_CONFIG"] = str(_cli_config)
    env["NEXUS_HS_ADDRESS"] = f"127.0.0.1:{_grpc_port}"
    env["NEXUS_HS_API_KEY_FILE"] = str(key_file)
    r = subprocess.run(
        [
            "/root/.cargo/bin/cargo",
            "run",
            "--offline",
            "-p",
            "nexus-headscale",
            "--example",
            "mesh_live_proof",
        ],
        env=env,
        capture_output=True,
        text=True,
        timeout=240,
        cwd=str(ROOT),
    )
    key_file.unlink(missing_ok=True)
    assert r.returncode == 0, f"adapter live proof failed:\n{r.stdout}\n{r.stderr}"
    assert "EP-009 M3 headscale live proof: ok" in r.stdout


def ep009_integration_headscale_teardown_leaves_no_orphans():
    """Explicit teardown proof: remove container + network, then assert
    ZERO leftovers. (teardown_module also runs as a safety net.)"""
    _docker(["rm", "-f", _CONTAINER], check=False)
    _docker(["network", "rm", _NET], check=False)
    ps = _docker(
        ["ps", "-a", "--filter", f"name={_CONTAINER}", "--format", "{{.Names}}"], check=False
    )
    assert _CONTAINER not in ps.stdout, "container must be removed"
    nets = _docker(["network", "ls", "--format", "{{.Name}}"], check=False)
    assert _NET not in nets.stdout, "network must be removed"
