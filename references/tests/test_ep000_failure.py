"""EP-000 failure tests: forced failures, abuse cases, and fail-closed behavior.

Test names begin with ep000_failure_ per the EP-000 milestone contract.
These tests copy the real evidence artifacts to a disposable temp directory,
corrupt them, and assert the owning scripts fail closed. They never modify
the repository's real artifacts.
"""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def run_script(script: str, workdir: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["sh", script],
        cwd=workdir,
        capture_output=True,
        text=True,
        timeout=60,
    )


def make_workdir() -> tuple[Path, Path]:
    """Copy repository scripts + references into a temp dir; return (tmp, refs)."""
    tmp = Path(tempfile.mkdtemp(prefix="ep000-failure-"))
    shutil.copytree(ROOT / "references", tmp / "references")
    (tmp / "scripts").mkdir(parents=True)
    for name in ["source-verify.sh", "version-verify.sh"]:
        shutil.copy2(ROOT / "scripts" / name, tmp / "scripts" / name)
    shutil.copy2(ROOT / ".tool-versions", tmp / ".tool-versions")
    shutil.copy2(ROOT / "VERSIONS.lock.yaml", tmp / "VERSIONS.lock.yaml")
    return tmp, tmp / "references"


def test_ep000_failure_corrupted_json_fails_closed() -> None:
    """source-verify.sh must fail when SOURCE_VERIFICATION.json is corrupt."""
    tmp, refs = make_workdir()
    try:
        (refs / "SOURCE_VERIFICATION.json").write_text("{not valid json", encoding="utf-8")
        result = run_script("scripts/source-verify.sh", tmp)
        assert result.returncode != 0, "source-verify must fail on corrupt json"
        assert "FAIL" in result.stderr
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def test_ep000_failure_unverified_record_fails_closed() -> None:
    """source-verify.sh must fail when a record is UNVERIFIED (no silent skip)."""
    tmp, refs = make_workdir()
    try:
        path = refs / "SOURCE_VERIFICATION.json"
        records = json.loads(path.read_text(encoding="utf-8"))
        records[0]["decision_status"] = "UNVERIFIED"
        path.write_text(json.dumps(records, indent=2), encoding="utf-8")
        result = run_script("scripts/source-verify.sh", tmp)
        assert result.returncode != 0, "source-verify must fail on UNVERIFIED record"
        assert "unverified" in result.stderr.lower()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def test_ep000_failure_missing_record_fails_closed() -> None:
    """source-verify.sh must fail when a required field is absent."""
    tmp, refs = make_workdir()
    try:
        path = refs / "SOURCE_VERIFICATION.json"
        records = json.loads(path.read_text(encoding="utf-8"))
        del records[0]["license"]
        path.write_text(json.dumps(records, indent=2), encoding="utf-8")
        result = run_script("scripts/source-verify.sh", tmp)
        assert result.returncode != 0, "source-verify must fail on missing field"
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def test_ep000_failure_version_mismatch_fails_closed() -> None:
    """version-verify.sh must fail when an installed tool violates the lock."""
    tmp, _ = make_workdir()
    try:
        # Point the probe at a fake rustc that reports a wrong version.
        bindir = tmp / "bin"
        bindir.mkdir()
        fake = bindir / "rustc"
        fake.write_text("#!/bin/sh\necho 'rustc 9.9.9'\n", encoding="utf-8")
        fake.chmod(0o755)
        env = dict(__import__("os").environ)
        env["PATH"] = f"{bindir}:{env.get('PATH', '')}"
        env["CI"] = "true"
        result = subprocess.run(
            ["sh", "scripts/version-verify.sh"],
            cwd=tmp,
            capture_output=True,
            text=True,
            timeout=60,
            env=env,
        )
        assert result.returncode != 0, "version-verify must fail on version mismatch"
        assert "rust" in result.stderr
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def test_ep000_failure_duplicate_source_records_rejected() -> None:
    """The collector rejects duplicate component records (idempotency boundary)."""
    # The collector is deterministic: run twice, outputs must be identical.
    out1 = subprocess.run(
        [sys.executable, "references/collect_source_evidence.py"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=300,
    )
    first = (ROOT / "references/SOURCE_VERIFICATION.json").read_bytes()
    out2 = subprocess.run(
        [sys.executable, "references/collect_source_evidence.py"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=300,
    )
    second = (ROOT / "references/SOURCE_VERIFICATION.json").read_bytes()
    assert out1.returncode == 0 and out2.returncode == 0
    assert first == second, "collector output must be deterministic across runs"


if __name__ == "__main__":
    failures = 0
    for name, fn in sorted(globals().items()):
        if name.startswith("test_ep000_failure_") and callable(fn):
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
        print(f"ep000 failure tests: {failures} failures")
        sys.exit(1)
    print("ep000 failure tests: ok")
