#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

# LF-019 self-healing-fix-loop (EP-019 M5; SPEC-018; ADR-026;
# LIVE_FIRE_PROOFS.md).
#
# Trigger a controlled defect, detect it through a real process
# failure, reproduce, patch, review, request approval, canary, verify,
# and close — with a deterministic rollback proof. The REAL EP-019
# composition is exercised end to end:
#   - real controlled failing fixture (tests/healing/fixtures/
#     failing-worker.sh, CONTROLLED_TEST_FIXTURE) crashes in a real
#     subprocess (exit 1) even with the correct marker path;
#   - real process-failure incident signal -> canonical correlation ->
#     incident memory record (tenant-scoped dedup key);
#   - diagnosis: hypothesis -> reproduction -> VALIDATED (a model can
#     never self-certify);
#   - real patch artifact (worker-fix.patch) with a real SHA-256 digest
#     applied to an isolated working copy with the real patch tool;
#   - gold-standard before/after: the SAME reproduction FAILS before
#     and PASSES after;
#   - sandbox + security verdicts (fail closed, scope preserved);
#   - independent review + human approval bound to the exact patch
#     digest (a different digest is never authorized);
#   - canary plan with health criteria -> healthy;
#   - post-deploy verification re-runs the original reproduction;
#   - incident memory records the closed incident (redacted);
#   - deterministic rollback: restore the previous artifact, the
#     original failing behavior returns (health restored to the known
#     previous state).
#
# The fixture is CONTROLLED_TEST_FIXTURE; the engine orchestration is
# the REAL nexus-healing contract machinery. Real OS-level sandbox and
# real production canary certification are DEFERRED and recorded in
# CERTIFICATION_REGISTRY.md (EP-040/EP-043 / deployment-owning node).

cargo test --locked -p nexus-healing --test lf019
echo "LF-019: ok"
