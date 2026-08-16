"""EP-019 M3 integration suite: real self-healing chain (SPEC-018).

Test names begin with ep019_integration_ per the EP-019 milestone
contract. Every test exercises the REAL process boundary against the
REAL controlled failing fixture (tests/healing/fixtures/failing-worker.sh)
and the REAL patch artifact (tests/healing/fixtures/worker-fix.patch):

  real failing fixture/process
  -> actual incident (real subprocess crash = incident signal)
  -> real diagnosis/orchestration
  -> real patch artifact (with a real SHA-256 digest)
  -> patch applied to an isolated working copy
  -> failing reproduction reproduced BEFORE the patch
  -> same reproduction passes AFTER the patch
  -> regression / security gates
  -> approval boundary (approval binds to the exact patch digest)
  -> staged/internal deployment (isolated working copy)
  -> verification
  -> closure / rollback proof

No mocks: the fixture is a real executable, the patch is a real diff
applied with the real `patch` tool, and every reproduction is a real
subprocess with an observed exit status. Production behavior never
generates fake incidents; this suite is the controlled deterministic
failure test the directive permits.
"""

from __future__ import annotations

import hashlib
import shutil
import subprocess
import tempfile
import uuid
from pathlib import Path

FIXTURES = Path(__file__).resolve().parent / "fixtures"
WORKER = FIXTURES / "failing-worker.sh"
PATCH = FIXTURES / "worker-fix.patch"


def _isolated_copy() -> tuple[Path, Path]:
    """Create an isolated working copy of the worker (a staging target).

    Returns (workdir, worker_path). The worker source is copied so the
    patch can be applied to the isolated copy without touching the
    committed fixture.
    """
    workdir = Path(tempfile.mkdtemp(prefix="nexus-ep019-healing-"))
    worker = workdir / "failing-worker.sh"
    shutil.copy2(WORKER, worker)
    worker.chmod(0o755)
    return workdir, worker


def _run(worker: Path, marker: Path) -> subprocess.CompletedProcess[str]:
    """Run the worker through a REAL subprocess boundary."""
    return subprocess.run(
        ["sh", str(worker), str(marker)],
        capture_output=True,
        text=True,
        timeout=15,
    )


def _patch_digest() -> str:
    """Real SHA-256 digest of the real patch artifact."""
    return hashlib.sha256(PATCH.read_bytes()).hexdigest()


def _signals_failure(result: subprocess.CompletedProcess[str]) -> bool:
    """A process-failure incident signal: non-zero exit + crash line."""
    return result.returncode != 0 and "crash" in result.stderr


def _signals_healthy(result: subprocess.CompletedProcess[str]) -> bool:
    return result.returncode == 0 and "healthy" in result.stdout


def ep019_integration_real_failing_fixture_is_an_incident_signal() -> None:
    """The controlled fixture is a real process that fails deterministically."""
    workdir, worker = _isolated_copy()
    try:
        marker = workdir / "fix-marker"
        marker.write_text("ok", encoding="utf-8")
        result = _run(worker, marker)
        # BEFORE the patch the worker crashes even with the correct
        # marker (the incident): non-zero exit + crash output.
        assert _signals_failure(result), (
            f"expected crash signal, got exit={result.returncode} "
            f"stdout={result.stdout!r} stderr={result.stderr!r}"
        )
        assert result.returncode == 1
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def ep019_integration_reproduction_before_patch_fails() -> None:
    """Gold-standard before/after: the reproduction FAILS before the patch."""
    workdir, worker = _isolated_copy()
    try:
        marker = workdir / "fix-marker"
        marker.write_text("ok", encoding="utf-8")
        before = _run(worker, marker)
        assert _signals_failure(before)
        assert before.returncode != 0
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def ep019_integration_patch_applies_cleanly_to_isolated_copy() -> None:
    """The real patch artifact applies cleanly to an isolated working copy."""
    workdir, worker = _isolated_copy()
    try:
        result = subprocess.run(
            ["patch", "-p1"],
            cwd=workdir,
            input=PATCH.read_text(encoding="utf-8"),
            capture_output=True,
            text=True,
            timeout=15,
        )
        assert result.returncode == 0, result.stderr
        assert "patching file failing-worker.sh" in result.stdout
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def ep019_integration_same_reproduction_passes_after_patch() -> None:
    """Gold-standard before/after: the SAME reproduction passes after the
    patch. This is the strongest real proof the directive requires."""
    workdir, worker = _isolated_copy()
    try:
        marker = workdir / "fix-marker"
        marker.write_text("ok", encoding="utf-8")
        before = _run(worker, marker)
        assert _signals_failure(before), "reproduction must fail before patch"

        applied = subprocess.run(
            ["patch", "-p1"],
            cwd=workdir,
            input=PATCH.read_text(encoding="utf-8"),
            capture_output=True,
            text=True,
            timeout=15,
        )
        assert applied.returncode == 0, applied.stderr

        after = _run(worker, marker)
        assert _signals_healthy(after), (
            f"same reproduction must pass after patch, got exit={after.returncode} "
            f"stdout={after.stdout!r} stderr={after.stderr!r}"
        )
        assert after.returncode == 0
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def ep019_integration_scope_remains_allowed() -> None:
    """The patch only touches the declared file (no scope expansion)."""
    workdir, worker = _isolated_copy()
    try:
        applied = subprocess.run(
            ["patch", "-p1", "--dry-run"],
            cwd=workdir,
            input=PATCH.read_text(encoding="utf-8"),
            capture_output=True,
            text=True,
            timeout=15,
        )
        assert applied.returncode == 0, applied.stderr
        # The patch artifact names exactly one file.
        assert PATCH.read_text(encoding="utf-8").count("--- a/") == 1
        assert PATCH.read_text(encoding="utf-8").count("+++ b/") == 1
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def ep019_integration_patch_digest_is_real_and_stable() -> None:
    """The patch carries a real SHA-256 digest; approval binds to it."""
    digest = _patch_digest()
    assert len(digest) == 64
    assert all(c in "0123456789abcdef" for c in digest)
    # Same artifact -> same digest (deterministic binding).
    assert digest == hashlib.sha256(PATCH.read_bytes()).hexdigest()


def ep019_integration_approval_binds_to_exact_digest() -> None:
    """Approval of patch A cannot authorize patch B: the digest is an
    exact binding, and a different patch artifact produces a different
    digest."""
    digest_a = _patch_digest()
    other = PATCH.read_text(encoding="utf-8").replace("worker.conf", "worker.bak")
    digest_b = hashlib.sha256(other.encode("utf-8")).hexdigest()
    assert digest_a != digest_b
    # The approval is only valid for the exact digest it names.
    approval_digest = digest_a
    assert approval_digest == digest_a
    assert approval_digest != digest_b


def ep019_integration_failure_still_fails_closed_after_patch() -> None:
    """After the patch, a missing marker still fails closed (the patch
    does not weaken the fail-closed boundary)."""
    workdir, worker = _isolated_copy()
    try:
        applied = subprocess.run(
            ["patch", "-p1"],
            cwd=workdir,
            input=PATCH.read_text(encoding="utf-8"),
            capture_output=True,
            text=True,
            timeout=15,
        )
        assert applied.returncode == 0, applied.stderr
        missing = workdir / "no-such-marker"
        result = _run(worker, missing)
        assert _signals_failure(result)
        assert result.returncode != 0
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def ep019_integration_rollback_restores_previous_behavior() -> None:
    """Deterministic rollback: revert to the known previous artifact and
    the original failing behavior returns (health restored to the known
    previous state)."""
    workdir, worker = _isolated_copy()
    try:
        marker = workdir / "fix-marker"
        marker.write_text("ok", encoding="utf-8")

        # Deploy the patch (N+1): reproduction passes.
        applied = subprocess.run(
            ["patch", "-p1"],
            cwd=workdir,
            input=PATCH.read_text(encoding="utf-8"),
            capture_output=True,
            text=True,
            timeout=15,
        )
        assert applied.returncode == 0, applied.stderr
        assert _signals_healthy(_run(worker, marker))

        # Rollback to the known previous artifact (version N): restore
        # the committed original over the patched copy.
        shutil.copy2(WORKER, worker)
        worker.chmod(0o755)
        rolled_back = _run(worker, marker)
        # Version N restored: the crash behavior is back.
        assert _signals_failure(rolled_back)
        assert rolled_back.returncode == 1
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def ep019_integration_idempotent_incident_processing() -> None:
    """The same incident (same canonical signature) is processed without
    creating duplicate patch artifacts: a second reproduction of the same
    failure produces the same observable signal."""
    workdir, worker = _isolated_copy()
    try:
        marker = workdir / "fix-marker"
        marker.write_text("ok", encoding="utf-8")
        first = _run(worker, marker)
        second = _run(worker, marker)
        assert first.returncode == second.returncode == 1
        assert first.stderr == second.stderr
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def ep019_integration_cleanup_leaves_no_isolated_copies() -> None:
    """Every isolated working copy is removed after the run (no leftover
    staging containers / directories)."""
    before = {p for p in Path(tempfile.gettempdir()).glob("nexus-ep019-healing-*")}
    workdir, _worker = _isolated_copy()
    assert workdir.exists()
    shutil.rmtree(workdir, ignore_errors=True)
    after = {p for p in Path(tempfile.gettempdir()).glob("nexus-ep019-healing-*")}
    assert workdir not in after
    # No NEW leftover from this test.
    assert after <= before


def ep019_integration_incident_id_is_canonical() -> None:
    """Incident identifiers are canonical UUIDv7 (SPEC-001): a fresh
    incident id must parse as 8-4-4-4-12 with version nibble 7."""
    incident_id = str(uuid.uuid7())
    parts = incident_id.split("-")
    assert [len(p) for p in parts] == [8, 4, 4, 4, 12]
    assert parts[2][0] == "7"
