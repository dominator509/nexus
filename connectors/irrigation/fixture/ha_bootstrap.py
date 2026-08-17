#!/usr/bin/env python3
"""EP-024 M4 irrigation fixture bootstrap for the REAL pinned Home Assistant provider.

Owns one real HA container (nexus-ep024-irr) for the irrigation
forced-failure integration suite. Authentication and the REST surface
REUSE the EP-020-certified boundary (same flow as EP-020 M3 and EP-024
M3 fixtures): the CLI auth `add` verb, the post-restart OAuth login
flow (login_flow -> authorization_code -> access_token), and the
documented HA REST API. No credentials are checked in; a fresh token
is minted per run and handed to the test process through an
environment file.

Classification:
  - Home Assistant server/API: REAL_EXTERNAL_DEPENDENCY
  - input_boolean.nexus_zone_a / nexus_zone_b: CONTROLLED_TEST_FIXTURE
    (real entities, real service calls; not physical irrigation
    hardware)

Usage:
  ha_bootstrap.py start   # create container, wait, mint token, write env
  ha_bootstrap.py teardown  # remove container + generated fixture state
"""

import hashlib
import json
import os
import pathlib
import subprocess
import sys
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[3]
CONFIG_DIR = ROOT / "connectors/irrigation/fixture/config"
ENV_FILE = pathlib.Path("/tmp/ep024-irr.env")

IMAGE = "ghcr.io/home-assistant/home-assistant:stable"
DIGEST = "sha256:56690a89c79a0de98035e1719f8324a92d5859c1192ff45adb0230ea81cb42a5"
NAME = "nexus-ep024-irr"
BASE = "http://127.0.0.1:8125"

EXPECTED_ENTITIES = (
    "input_boolean.nexus_zone_a",
    "input_boolean.nexus_zone_b",
    "sensor.nexus_zone_unknown",
)

GENERATED_STATE = [
    ".storage",
    "home-assistant.log",
    "home-assistant.log.1",
    "home-assistant.log.fault",
    "home-assistant_v2.db",
    "home-assistant_v2.db-shm",
    "home-assistant_v2.db-wal",
    ".HA_VERSION",
    ".ha_run.lock",
    "automations.yaml",
    "scenes.yaml",
    "scripts.yaml",
    "secrets.yaml",
    "blueprints",
    "deps",
    "tts",
]


def _docker(args, check=True):
    return subprocess.run(
        ["/usr/bin/docker", *args], capture_output=True, text=True, check=check
    )


def _random_pw() -> str:
    return hashlib.sha256(os.urandom(24)).hexdigest()[:24]


def _cleanup_generated_state() -> None:
    for name in GENERATED_STATE:
        p = CONFIG_DIR / name
        if p.exists() or p.is_symlink():
            subprocess.run(
                ["rm", "-rf", str(p)], check=False, capture_output=True, text=True
            )


def _cleanup_old() -> None:
    _docker(["rm", "-f", NAME], check=False)
    _cleanup_generated_state()


def _assert_repo_root() -> None:
    assert (ROOT / "AGENTS.md").is_file(), f"ROOT misresolved: {ROOT}"
    assert (CONFIG_DIR / "configuration.yaml").is_file(), (
        f"fixture config missing at {CONFIG_DIR}"
    )


def _wait_ready(timeout: int = 300) -> None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            req = urllib.request.Request(f"{BASE}/api/")
            with urllib.request.urlopen(req, timeout=3) as resp:
                if resp.status == 200:
                    return
        except urllib.error.HTTPError as e:
            if e.code == 401:
                return
        except Exception:
            pass
        time.sleep(2)
    raise RuntimeError("HA did not become ready")


def _wait_preauth_boot(timeout: int = 300) -> None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        reg = _docker(
            ["exec", NAME, "cat", "/config/.storage/core.entity_registry"],
            check=False,
        )
        if reg.returncode == 0 and all(
            e.split(".")[-1] in reg.stdout for e in EXPECTED_ENTITIES
        ):
            return
        time.sleep(2)
    raise RuntimeError("HA first boot did not finish integration setup")


def _assert_config_mount() -> None:
    insp = _docker(
        [
            "inspect", NAME, "--format",
            "{{range .Mounts}}{{.Source}} -> {{.Destination}}{{\"\\n\"}}{{end}}",
        ]
    )
    mounts = insp.stdout
    assert "/config" in mounts, f"config mount missing: {mounts}"
    assert str(CONFIG_DIR) in mounts, (
        f"config mount is not the EP-024 irrigation fixture config: {mounts}"
    )
    assert "infra/infra" not in mounts, f"shallow-root config mount: {mounts}"


def _bootstrap_auth() -> str:
    pw = _random_pw()
    add_user = _docker(
        [
            "exec", NAME, "python3",
            "-m", "homeassistant", "--script", "auth",
            "--config", "/config", "add", "nexus-admin", pw,
        ],
        check=False,
    )
    if add_user.returncode != 0:
        raise RuntimeError(f"auth add failed: {add_user.stderr}")
    _docker(["restart", NAME], check=True)
    _wait_ready(300)

    def post_json(path: str, body: dict) -> dict:
        req = urllib.request.Request(f"{BASE}{path}", method="POST")
        req.add_header("Content-Type", "application/json")
        with urllib.request.urlopen(
            req, data=json.dumps(body).encode(), timeout=10
        ) as resp:
            return json.loads(resp.read())

    def post_form(path: str, body: str) -> dict:
        req = urllib.request.Request(f"{BASE}{path}", method="POST")
        req.add_header("Content-Type", "application/x-www-form-urlencoded")
        with urllib.request.urlopen(
            req, data=body.encode(), timeout=10
        ) as resp:
            return json.loads(resp.read())

    flow = post_json(
        "/auth/login_flow",
        {
            "client_id": "http://localhost:8125/",
            "handler": ["homeassistant", None],
            "redirect_uri": "http://localhost:8125/",
        },
    )
    step = post_json(
        f"/auth/login_flow/{flow['flow_id']}",
        {
            "client_id": "http://localhost:8125/",
            "username": "nexus-admin",
            "password": pw,
        },
    )
    if step.get("type") != "create_entry":
        raise RuntimeError(f"login flow did not create entry: {step}")
    code = step["result"]
    token = post_form(
        "/auth/token",
        (
            "grant_type=authorization_code"
            f"&code={code}"
            "&client_id=http://localhost:8125/"
        ),
    )
    access = token.get("access_token", "")
    if not access:
        raise RuntimeError(f"token exchange produced no access token: {token}")
    return access


def _wait_entities(token: str, timeout: int = 120) -> None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        req = urllib.request.Request(f"{BASE}/api/states")
        req.add_header("Authorization", f"Bearer {token}")
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read())
                ids = {s.get("entity_id") for s in data}
                if all(e in ids for e in EXPECTED_ENTITIES):
                    return
        except Exception:
            pass
        time.sleep(2)
    raise RuntimeError("HA fixture entities did not become ready")


def start() -> None:
    inspect = _docker(["image", "inspect", f"{IMAGE}@{DIGEST}"], check=False)
    if inspect.returncode != 0:
        raise RuntimeError("pinned HA image not present locally")
    _cleanup_old()
    _assert_repo_root()
    _docker(
        [
            "run", "-d", "--name", NAME,
            "-v", f"{CONFIG_DIR}:/config",
            "-p", "127.0.0.1:8125:8123",
            "-e", "TZ=UTC",
            f"{IMAGE}@{DIGEST}",
        ],
        check=True,
    )
    _wait_ready()
    _wait_preauth_boot()
    _assert_config_mount()
    token = _bootstrap_auth()
    _wait_entities(token)
    ENV_FILE.write_text(
        f"NEXUS_HA_BASE={BASE}\nNEXUS_HA_TOKEN={token}\nNEXUS_HA_CONTAINER={NAME}\n",
        encoding="utf8",
    )
    print(f"EP-024 irrigation fixture ready: {BASE} container={NAME}")


def teardown() -> None:
    _docker(["rm", "-f", NAME], check=False)
    _cleanup_generated_state()
    try:
        ENV_FILE.unlink()
    except FileNotFoundError:
        pass
    print("EP-024 irrigation fixture torn down")


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "start":
        start()
    elif cmd == "teardown":
        teardown()
    else:
        print(f"usage: {sys.argv[0]} start|teardown", file=sys.stderr)
        sys.exit(2)
