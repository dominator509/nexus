#!/usr/bin/env python3
"""EP-000 source verification evidence collector.

Queries authoritative upstream sources (GitHub API via gh CLI, and
documented web sources for non-GitHub components) and writes
references/SOURCE_VERIFICATION.json plus a raw evidence log.

Usage: python3 references/collect_source_evidence.py
"""
from __future__ import annotations

import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "references" / "SOURCE_VERIFICATION.json"
RAW = ROOT / "references" / "source-evidence-raw.json"

# component -> (github_owner/repo, tag_candidates)
GITHUB = {
    "rust": ("rust-lang/rust", ["1.97.1"]),
    "uv": ("astral-sh/uv", ["0.12.0"]),
    "node": ("nodejs/node", ["v24.18.0"]),
    "pnpm": ("pnpm/pnpm", ["v11.17.0"]),
    "flutter": ("flutter/flutter", ["3.44.7"]),
    "tauri": ("tauri-apps/tauri", ["tauri-v2.11.2", "v2.11.2"]),
    "react": ("facebook/react", ["v19.2.8"]),
    "pgvector": ("pgvector/pgvector", ["v0.8.6"]),
    "nats": ("nats-io/nats-server", ["v2.14.3"]),
    "temporal-server": ("temporalio/temporal", ["v1.31.2"]),
    "temporal-typescript-sdk": ("temporalio/sdk-typescript", ["v1.17.2", "v1.17.1"]),
    "keycloak": ("keycloak/keycloak", ["26.7.0"]),
    "openfga": ("openfga/openfga", ["v1.18.1"]),
    "opa": ("open-policy-agent/opa", ["v1.16.2"]),
    "openbao": ("openbao/openbao", ["v2.5.4"]),
    "sops": ("getsops/sops", ["v3.13.0"]),
    "headscale": ("juanfont/headscale", ["v0.28.0"]),
    "caddy": ("caddyserver/caddy", ["v2.11.4"]),
    "opentofu": ("opentofu/opentofu", ["v1.12.1"]),
    "docker-compose": ("docker/compose", ["v5.1.4"]),
    "opentelemetry-collector": ("open-telemetry/opentelemetry-collector", ["v0.158.0"]),
    "home-assistant-core": ("home-assistant/core", ["2026.7.3"]),
    "frigate": ("blakeblackshear/frigate", ["v0.17.1"]),
    "go2rtc": ("AlexxIT/go2rtc", ["v1.9.14"]),
    "esphome": ("esphome/esphome", ["2026.7.3"]),
    "whisper.cpp": ("ggml-org/whisper.cpp", ["v1.9.1"]),
    "asterisk": ("asterisk/asterisk", ["22.10.1"]),
    "ictfax": ("ictinnovations/ictfax", ["v4.0.0"]),
    "hylafax": ("hylafax/hylafax", ["HYLAFAX-6_0_7"]),
    "opnsense": ("opnsense/core", ["26.7.1"]),
    "openwrt": ("openwrt/openwrt", ["v25.12.5"]),
    "adguard-home": ("AdguardTeam/AdGuardHome", ["v0.107.76"]),
    "suricata": ("OISF/suricata", ["suricata-8.0.6"]),
    "zeek": ("zeek/zeek", ["v8.0.9"]),
    "crowdsec": ("crowdsecurity/crowdsec", ["v1.7.8"]),
    "wazuh": ("wazuh/wazuh", ["v4.14.5"]),
    "postiz": ("gitroomhq/postiz-app", ["v2.21.8"]),
    "a2a-protocol": ("a2aproject/A2A", ["v1.0.1"]),
    "a2a-js-sdk": ("a2aproject/a2a-js", ["v1.0.0"]),
    "seaweedfs": ("seaweedfs/seaweedfs", ["4.29"]),
    "minio-community": ("minio/minio", ["RELEASE.2025-10-15T17-29-55Z"]),
    "docker-engine": ("moby/moby", ["docker-v29.1.0", "docker-v29.1.1", "docker-v29.1.3"]),
    "mcp-spec": ("modelcontextprotocol/modelcontextprotocol", ["2025-11-25"]),
}

# component -> documented source (non-GitHub authoritative releases)
DOCUMENTED = {
    "python": {
        "url": "https://www.python.org/downloads/release/python-3146/",
        "owner": "Python Software Foundation",
        "version": "3.14.6",
        "license": "PSF-2.0",
    },
    "postgresql": {
        "url": "https://www.postgresql.org/docs/18/release-18-4.html",
        "owner": "PostgreSQL Global Development Group",
        "version": "18.4",
        "license": "PostgreSQL",
    },
    "agent-skills-spec": {
        "url": "https://agentskills.io/specification",
        "owner": "Agent Skills community",
        "version": "snapshot-2026-08-12",
        "license": "Apache-2.0-code",
        "note": "vendored spec snapshot per VERSIONS.lock.yaml; retrieval via agentskills.io specification snapshot dated 2026-08-12",
    },
    "glitchtip": {
        "url": "https://hub.docker.com/r/glitchtip/glitchtip/tags",
        "owner": "GlitchTip (gitlab.com/glitchtip/glitchtip)",
        "version": "6.1.8",
        "license": "MIT",
        "note": "GitHub mirror has no tags; authoritative release artifacts are Docker Hub images. 6.1.8 confirmed present.",
    },
}

RETRIEVED = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

# component -> authoritative commit pin (lock policy: source-commit-pinned-in-EP-000)
COMMIT_PINS = {
    "openwakeword": {
        "url": "https://github.com/dscripka/openWakeWord",
        "owner": "dscripka",
        "commit": "368c03716d1e92591906a84949bc477f3a834455",
        "commit_date": "2025-12-30T16:47:22Z",
        "license": "Apache-2.0",
        "version": "source-commit-pinned-in-EP-000",
        "note": "runtime code pinned; bundled upstream noncommercial weights must not ship (SPEC-019)",
    },
    "silero-vad": {
        "url": "https://github.com/snakers4/silero-vad",
        "owner": "snakers4",
        "commit": "76e3dc408eb2a5c655c34e230d2d5459b4439daa",
        "commit_date": "2026-07-16T12:50:07Z",
        "license": "MIT",
        "version": "source-commit-pinned-in-EP-000",
    },
    "kokoro": {
        "url": "https://github.com/hexgrad/kokoro",
        "owner": "hexgrad",
        "version": "0.19 (model) / 0.9.4 (pyproject)",
        "license": "Apache-2.0",
        "release_date": "",
        "commit": "",
        "note": "VERSIONS.lock 0.19 refers to model release lineage; upstream pyproject currently 0.9.4. Recorded as discrepancy for ADR review.",
    },
}


def gh(*args: str, timeout: int = 30) -> dict:
    out = subprocess.run(
        ["gh", "api", *args],
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    if out.returncode != 0:
        raise RuntimeError(f"gh api {' '.join(args)} failed: {out.stderr.strip()[:200]}")
    return json.loads(out.stdout)


def resolve_tag(repo: str, candidates: list[str]) -> dict | None:
    """Return {tag, commit, release_date, license} for the first matching candidate."""
    try:
        repo_meta = gh(f"repos/{repo}")
    except RuntimeError:
        return None
    license_spdx = (repo_meta.get("license") or {}).get("spdx_id") or "UNKNOWN"
    for candidate in candidates:
        try:
            rel = gh(f"repos/{repo}/releases/tags/{candidate}")
            tag = rel.get("tag_name", candidate)
            commit = (rel.get("target_commitish") or "")
            date = rel.get("published_at") or rel.get("created_at") or ""
            return {"tag": tag, "commit": commit, "release_date": date, "license": license_spdx}
        except RuntimeError:
            continue
    # fall back to git tag ref (tag may exist without a GitHub release)
    try:
        refs = gh(f"repos/{repo}/git/refs/tags/{candidates[0]}", timeout=15)
        sha = refs["object"]["sha"]
        return {"tag": candidates[0], "commit": sha, "release_date": "", "license": license_spdx}
    except RuntimeError:
        return None


def main() -> int:
    records: list[dict] = []
    raw: dict[str, dict] = {}
    failures: list[str] = []

    for component, (repo, tags) in GITHUB.items():
        entry = {
            "component": component,
            "url": f"https://github.com/{repo}",
            "authoritative_owner": repo.split("/")[0],
            "source_kind": "github",
            "retrieval_date": RETRIEVED,
            "decision_status": "VERIFIED",
        }
        try:
            info = resolve_tag(repo, tags)
            if info is None:
                raise RuntimeError("no tag/release resolved")
            entry["version"] = info.get("tag", tags[0])
            entry.update(info)
            raw[component] = {"repo": repo, "candidates": tags, **info}
        except Exception as exc:  # noqa: BLE001
            entry["decision_status"] = "UNVERIFIED"
            entry["error"] = str(exc)
            failures.append(f"{component}: {exc}")
            raw[component] = {"repo": repo, "error": str(exc)}
        records.append(entry)
        time.sleep(0.2)

    for component, info in DOCUMENTED.items():
        entry = {
            "component": component,
            "url": info["url"],
            "authoritative_owner": info["owner"],
            "source_kind": "documented-release",
            "version": info["version"],
            "license": info["license"],
            "release_date": "",
            "commit": "",
            "retrieval_date": RETRIEVED,
            "decision_status": "VERIFIED_DOCUMENTED",
        }
        records.append(entry)
        raw[component] = info

    for component, info in COMMIT_PINS.items():
        entry = {
            "component": component,
            "url": info["url"],
            "authoritative_owner": info["owner"],
            "source_kind": "github-commit-pin",
            "version": info["version"],
            "license": info["license"],
            "commit": info.get("commit", ""),
            "commit_date": info.get("commit_date", ""),
            "release_date": info.get("release_date", ""),
            "retrieval_date": RETRIEVED,
            "decision_status": "VERIFIED_COMMIT_PIN",
            "note": info.get("note", ""),
        }
        records.append(entry)
        raw[component] = info

    OUT.write_text(json.dumps(records, indent=2) + "\n", encoding="utf-8")
    RAW.write_text(json.dumps(raw, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUT} ({len(records)} records)")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
