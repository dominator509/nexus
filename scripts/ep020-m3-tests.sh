#!/usr/bin/env sh
# EP-020 M3 gate: run the REAL Home Assistant integration suite.
#
# The M3 changed-files fence is infra/home-assistant/ (Python pytest
# suite + real HA container config). The authoritative gate is the
# ep020_integration pytest suite against the REAL pinned HA container.
# The vacuity guard is required: pytest exits 0 on a zero-collected
# run (EP-001 gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep020-m3-tests.log"
: > "$log"

# System python3 carries websocket-client + pytest in this environment
# (EP-011 sidecar precedent: `python3` runs repo test fixtures).
# --tb=native: pytest 9 dumps helper-frame locals (secrets) in default
# tracebacks on failure; the suite mints a real OAuth token per run.
if ! python3 -m pytest infra/home-assistant/tests/test_ep020_integration_home_assistant.py -q --tb=native >>"$log" 2>&1; then
  echo "EP-020 M3: FAIL - pytest integration suite failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# Vacuity guard: at least one ep020_integration test passed.
if ! grep -qE 'ep020_integration_.* PASSED' "$log" && ! grep -qE '^[0-9]+ passed' "$log"; then
  echo "EP-020 M3: FAIL - no ep020_integration tests passed (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-020 M3: ok"
