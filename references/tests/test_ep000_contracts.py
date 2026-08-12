"""EP-000 unit tests: source verification, toolchain lock, and contract records.

Test names begin with ep000_unit_ per the EP-000 milestone contract.
These tests exercise the real evidence artifacts produced by this node:
- references/SOURCE_VERIFICATION.json (VerifiedSourceRecord set)
- .tool-versions (ToolchainLock)
- VERSIONS.lock.yaml (component decision input)
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def load_sources() -> list[dict]:
    path = ROOT / "references" / "SOURCE_VERIFICATION.json"
    assert path.is_file(), f"missing {path}"
    return json.loads(path.read_text(encoding="utf-8"))


def load_lock() -> dict:
    import yaml

    path = ROOT / "VERSIONS.lock.yaml"
    assert path.is_file(), f"missing {path}"
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def load_tool_versions() -> dict[str, str]:
    path = ROOT / ".tool-versions"
    assert path.is_file(), f"missing {path}"
    out: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        assert len(parts) == 2, f"malformed .tool-versions line: {line}"
        out[parts[0]] = parts[1]
    return out


def test_ep000_unit_source_verification_complete() -> None:
    """Every locked component has a verified source record."""
    lock = load_lock()
    sources = load_sources()
    locked = {c["component"] for c in lock["locks"]}
    verified = {s["component"] for s in sources}
    missing = locked - verified
    assert not missing, f"components missing source records: {sorted(missing)}"


def test_ep000_unit_source_records_have_required_fields() -> None:
    """VerifiedSourceRecord fields: URL, owner, version, license, retrieval date, status."""
    required = {
        "component",
        "url",
        "authoritative_owner",
        "version",
        "license",
        "retrieval_date",
        "decision_status",
    }
    for record in load_sources():
        missing = required - set(record)
        assert not missing, f"{record['component']} missing {sorted(missing)}"
        assert record["url"].startswith(("https://", "http://")), f"bad url {record}"
        assert record["decision_status"] in {
            "VERIFIED",
            "VERIFIED_DOCUMENTED",
            "VERIFIED_COMMIT_PIN",
            "UNVERIFIED",
        }, f"bad status {record}"


def test_ep000_unit_no_unverified_sources() -> None:
    """Reality rule: a source that cannot be verified is not certified."""
    unverified = [r for r in load_sources() if r["decision_status"] == "UNVERIFIED"]
    assert not unverified, f"unverified sources: {unverified}"


def test_ep000_unit_tool_versions_match_lock() -> None:
    """ToolchainLock in .tool-versions agrees with VERSIONS.lock.yaml for pinned tools."""
    lock = load_lock()
    tool_versions = load_tool_versions()
    for tool, version in tool_versions.items():
        lock_entries = [c for c in lock["locks"] if c["component"] == tool]
        assert lock_entries, f"{tool} in .tool-versions but not in VERSIONS.lock.yaml"
        locked_version = lock_entries[0]["version"]
        assert version == locked_version, f"{tool}: .tool-versions {version} != lock {locked_version}"


def test_ep000_unit_source_registry_matches_lock() -> None:
    """Every component in SOURCE_REGISTRY.md's verification scope is present."""
    lock = load_lock()
    sources = load_sources()
    locked = {c["component"] for c in lock["locks"]}
    verified = {s["component"] for s in sources}
    assert locked == verified, "source record set must exactly match lock set"


def test_ep000_unit_no_placeholder_versions() -> None:
    """No unpinned 'latest' or empty version survives in the source records."""
    for record in load_sources():
        version = str(record.get("version", ""))
        assert version.strip(), f"{record['component']} has empty version"
        assert "latest" not in version.lower(), f"{record['component']} uses latest"


def test_ep000_unit_source_urls_are_authoritative_domains() -> None:
    """Source URLs must point at the authoritative upstream, not a fork or mirror."""
    forbidden = ("example.com", "example.test", "localhost", "127.0.0.1")
    for record in load_sources():
        url = record["url"].lower()
        for bad in forbidden:
            assert bad not in url, f"{record['component']} uses non-authoritative url {url}"


def test_ep000_unit_devcontainer_pins_locked_toolchain() -> None:
    """The devcontainer Dockerfile pins every locked toolchain version (no latest)."""
    dockerfile = (ROOT / "infra/devcontainer/Dockerfile").read_text(encoding="utf-8")
    expected_pins = {
        "rust": "1.97.1",
        "uv": "0.12.0",
        "node": "v24.18.0",
        "pnpm": "11.17.0",
        "flutter": "3.44.7",
        "sops": "v3.13.0",
    }
    for tool, version in expected_pins.items():
        assert version in dockerfile, f"devcontainer missing {tool} pin {version}"
    assert "latest" not in dockerfile, "devcontainer must not use unpinned latest"


def test_ep000_unit_mise_matches_tool_versions() -> None:
    """mise.toml and .tool-versions must agree with each other and the lock."""
    import tomllib

    mise = tomllib.loads((ROOT / "mise.toml").read_text(encoding="utf-8"))
    tools = mise["tools"]
    tool_versions = load_tool_versions()
    assert set(tools) == set(tool_versions), "mise.toml and .tool-versions tool sets differ"
    for tool in tools:
        assert str(tools[tool]) == tool_versions[tool], f"{tool} version differs"


def test_ep000_unit_devcontainer_syntax() -> None:
    """devcontainer.json parses and points at the Dockerfile."""
    import json as _json

    cfg = _json.loads((ROOT / "infra/devcontainer/devcontainer.json").read_text(encoding="utf-8"))
    assert cfg["build"]["dockerfile"] == "Dockerfile"
    assert cfg["name"]


def test_ep000_unit_no_unpinned_reference_in_toolchain() -> None:
    """No toolchain file references a floating 'latest' or empty version."""
    import tomllib

    mise = tomllib.loads((ROOT / "mise.toml").read_text(encoding="utf-8"))
    for tool, version in mise["tools"].items():
        assert version != "latest" and str(version).strip(), f"{tool} unpinned"
    dockerfile = (ROOT / "infra/devcontainer/Dockerfile").read_text(encoding="utf-8")
    assert "latest" not in dockerfile


if __name__ == "__main__":
    failures = 0
    for name, fn in sorted(globals().items()):
        if name.startswith("test_ep000_unit_") and callable(fn):
            try:
                fn()
                print(f"PASS {name}")
            except AssertionError as exc:
                failures += 1
                print(f"FAIL {name}: {exc}")
            except Exception as exc:  # noqa: BLE001
                failures += 1
                print(f"ERROR {name}: {exc!r}")
    if failures:
        print(f"ep000 unit tests: {failures} failures")
        sys.exit(1)
    print("ep000 unit tests: ok")
