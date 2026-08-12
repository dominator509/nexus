"""EP-000 integration tests: live verification of source records against upstreams.

Test names begin with ep000_integration_ per the EP-000 milestone contract.
These tests reach the authoritative upstream (GitHub API via gh, Docker Hub,
documented release pages) and confirm the recorded tags/releases actually exist.
They use real network access to controlled public endpoints; no production data.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def load_sources() -> list[dict]:
    path = ROOT / "references" / "SOURCE_VERIFICATION.json"
    return json.loads(path.read_text(encoding="utf-8"))


def gh(*args: str, timeout: int = 30) -> dict:
    out = subprocess.run(
        ["gh", "api", *args],
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    if out.returncode != 0:
        raise RuntimeError(f"gh api {' '.join(args)}: {out.stderr.strip()[:160]}")
    return json.loads(out.stdout)


def test_ep000_integration_github_tags_exist() -> None:
    """Every github-sourced record's tag resolves on the authoritative repo."""
    checked = 0
    for record in load_sources():
        if record.get("source_kind") != "github":
            continue
        tag = record.get("tag", "")
        if not tag:
            continue
        url = record["url"]
        repo = url.replace("https://github.com/", "").rstrip("/")
        try:
            rel = gh(f"repos/{repo}/releases/tags/{tag}", timeout=20)
            assert rel.get("tag_name") == tag, f"{repo} {tag} mismatch"
            checked += 1
        except Exception:  # noqa: BLE001
            # Some repos tag without a GitHub release; try git ref fallback.
            ref = gh(f"repos/{repo}/git/refs/tags/{tag}", timeout=20)
            assert ref.get("ref", "").endswith(tag), f"{repo} {tag} ref missing"
            checked += 1
    assert checked >= 30, f"expected >=30 live github checks, got {checked}"


def test_ep000_integration_docker_hub_tag_exists() -> None:
    """GlitchTip 6.1.8 exists on Docker Hub (authoritative release artifact)."""
    out = subprocess.run(
        [
            "curl", "-fsS", "--max-time", "20",
            "https://hub.docker.com/v2/repositories/glitchtip/glitchtip/tags?page_size=100",
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert out.returncode == 0, f"docker hub query failed: {out.stderr[:160]}"
    data = json.loads(out.stdout)
    names = {t["name"] for t in data.get("results", [])}
    assert "6.1.8" in names, "glitchtip 6.1.8 tag missing on Docker Hub"


def test_ep000_integration_documented_release_pages_reachable() -> None:
    """Documented release pages for python/postgresql/agent-skills are reachable."""
    urls = [
        "https://www.python.org/downloads/release/python-3146/",
        "https://www.postgresql.org/docs/18/release-18-4.html",
        "https://agentskills.io/specification",
    ]
    for url in urls:
        out = subprocess.run(
            ["curl", "-fsSI", "--max-time", "20", "-o", "/dev/null", "-w", "%{http_code}", url],
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert out.returncode == 0 and out.stdout.strip() == "200", f"{url} not 200: {out.stdout}"


if __name__ == "__main__":
    failures = 0
    for name, fn in sorted(globals().items()):
        if name.startswith("test_ep000_integration_") and callable(fn):
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
        print(f"ep000 integration tests: {failures} failures")
        sys.exit(1)
    print("ep000 integration tests: ok")
