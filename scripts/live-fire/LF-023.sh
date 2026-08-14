#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
# EP-011 M5 live-fire proof LF-023: the real nexus-sidecar wrapper over
# the real local legacy protocol fixture (tests/connectors/). Real
# processes, real loopback HTTP: discover capabilities, read state,
# issue an idempotent write, receive a change event.
. scripts/env.sh
log=/tmp/lf023-pytest.log
uv run --frozen pytest tests/connectors/test_ep011_live_fire.py -q -o 'python_functions=ep011_livefire_*' >"$log" 2>&1 || {
  cat "$log" >&2
  echo "LF-023: FAIL - live-fire pytest did not pass" >&2
  exit 1
}
# Vacuous-run guard: the selector must actually run the proof.
if ! grep -Eq '^[1-9][0-9]* passed' "$log"; then
  cat "$log" >&2
  echo "LF-023: FAIL - no live-fire tests ran (vacuity guard)" >&2
  exit 1
fi
mkdir -p .agent/state/evidence
{
  echo "# LF-023 (EP-011 M5) live-fire evidence"
  echo
  echo "Proof: real nexus-sidecar wrapper over real legacy fixture"
  echo "(tests/connectors/fixture_sidecar.py); real loopback HTTP."
  echo "Chain: DISCOVER -> QUERY -> idempotent COMMAND (+replay) -> CHANGEFEED event."
  echo
  echo "Observed stages (asserted over real HTTP, single wrapper process):"
  echo "  1. DISCOVER  -> capabilities include fixture.contacts.query,"
  echo "                  fixture.contacts.command, fixture.audit.changefeed"
  echo "  2. QUERY     -> contacts list read through the wrapper"
  echo "  3. COMMAND   -> idempotency key lf023-op-1 returns output.id"
  echo "  4. REPLAY    -> same key returns the identical output.id"
  echo "  5. CHANGEFEED-> events include fixture.contact.updated with"
  echo "                  payload.id == the command output.id"
  echo "Teardown: wrapper SIGTERM exit 0; wrapper port released; fixture"
  echo "killed; fixture port released (zero orphans)."
  echo
  echo "Certification status:"
  echo "  - external provider certification: NOT ASSERTED (fixture is a"
  echo "    test-zone provider; no third-party connector is owned by EP-011)"
  echo "  - crash-durable idempotency: NOT ASSERTED (idempotency is"
  echo "    in-process/key-based at the fixture; no persistence across"
  echo "    process death is claimed)"
  echo "  - crash-durable webhook replay: NOT ASSERTED (sidecar replay"
  echo "    dedupe is in-memory, process-lifetime)"
  echo
  echo "Command: uv run --frozen pytest tests/connectors/test_ep011_live_fire.py -o python_functions=ep011_livefire_*"
  echo "Observed:"
  sed -n 's/^/  /p' "$log"
} > .agent/state/evidence/LF-023-ep011-m5.md
echo "LF-023: ok"
