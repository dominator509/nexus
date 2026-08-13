#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
. scripts/env.sh
export NO_COLOR=1

# LF-017 durable-human-approval (EP-006 M5; LIVE_FIRE_PROOFS.md).
#
# Proof: start a workflow, restart the worker while waiting, approve
# later from "mobile" (the approval signal arrives while no worker
# polls), and prove exactly-once continuation - against a REAL Temporal
# server 1.31.2 backed by REAL postgres:18.4 (the M3 real-server
# harness in tests/workflows). The second proof replays the recorded
# history through the worker bundle (determinism under replay).
#
# Evidence is written under .agent/state/evidence/ (L6 always-writable
# state, governed not fenced). The sentinel is printed only after the
# real tests pass AND the post-suite orphan audit proves zero leftover
# EP-006 resources.

log="/tmp/lf017-vitest.log"
: > "$log"

pnpm --filter @nexus/workflows-tests exec vitest run \
  -t "ep006_integration_(worker_restart_delayed_approval_exactly_once|replay_recorded_history_succeeds)" \
  >>"$log" 2>&1 || {
  echo "LF-017: FAIL - live-fire vitest run failed" >&2
  tail -40 "$log" >&2
  exit 1
}

# Vacuity guard: at least one passing test must appear in the summary
# (vitest exits 0 even when the name filter matches nothing - the
# EP-001 gate-masking class).
if ! sed 's/\x1b\[[0-9;]*m//g' "$log" | grep -qE 'Tests[[:space:]]+[1-9][0-9]* passed'; then
  echo "LF-017: FAIL - no passing tests matched the proof filter (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Orphan audit: the live-fire run must leave zero EP-006 resources.
sh scripts/ep006-orphan-audit.sh || {
  echo "LF-017: FAIL - orphan audit after live-fire" >&2
  exit 1
}

evidence=".agent/state/evidence/LF-017-durable-human-approval.md"
{
  echo "# LF-017 Durable Human Approval (EP-006 M5)"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "Node: EP-006"
  echo "Command: sh scripts/live-fire/LF-017.sh"
  echo
  echo "## Real proof (no mocks, no in-memory engine)"
  echo "- ep006_integration_worker_restart_delayed_approval_exactly_once: REAL"
  echo "  Temporal server 1.31.2 (digest b5ecdb82...) + REAL postgres:18.4"
  echo "  (digest a02db8ca...); workflow started on worker A, worker A shut down"
  echo "  while step 2 awaited approval, the delayed approval signal was delivered"
  echo "  while NO worker polled (the mobile 'approve later' path), worker B"
  echo "  resumed from recorded history; each effect executed exactly once per"
  echo "  idempotency key (counting test activity, TESTING.md test zone)."
  echo "- ep006_integration_replay_recorded_history_succeeds: the completed"
  echo "  workflow's recorded history replays through the worker bundle with no"
  echo "  DeterminismViolationError (replay compatibility invariant)."
  echo
  echo "## Observed sentinel"
  echo "LF-017: ok"
  echo
  echo "## Post-proof hygiene"
  echo "- EP-006 orphan audit: ok (zero nexus-ep006 containers/networks/volumes/"
  echo "  temporal-server processes after the live-fire run)."
} > "$evidence"

echo "LF-017: ok"
