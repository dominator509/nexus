#!/usr/bin/env sh
# EP-026 M3 gate: run the nexus-microsoft-mail suite through the REAL
# cargo test machinery with vacuity guards.
#
# The M3 changed-files fence is connectors/microsoft-mail/ (Rust crate
# nexus-microsoft-mail), so the authoritative gate is the crate's own
# unit + integration suite (ep026_m3_transport, real-socket Graph
# fixture over real HTTP). Vacuity guards are required: `cargo test
# <filter>` exits 0 on a zero-match filter (EP-001 gate-masking
# class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep026-m3-tests.log"
: > "$log"

if ! cargo check -p nexus-microsoft-mail >>"$log" 2>&1; then
  echo "EP-026 M3: FAIL - cargo check nexus-microsoft-mail failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

if ! cargo test --locked -p nexus-microsoft-mail >>"$log" 2>&1; then
  echo "EP-026 M3: FAIL - cargo test nexus-microsoft-mail failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# Vacuity guard 1: at least one test binary ran a non-zero count of
# tests (lib unit suite AND integration suite both run).
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-026 M3: FAIL - no tests matched (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 2: the lib unit suite reports a passing non-zero
# result (integration suite result is asserted separately below).
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-026 M3: FAIL - no passing unit tests (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 3: the integration suite (ep026_m3_transport) ran and
# passed with a non-zero count. A gate that only ran the lib suite
# certifies nothing about the transport proof.
if ! grep -qE 'Running tests/ep026_m3_transport\.rs' "$log"; then
  echo "EP-026 M3: FAIL - integration suite did not run (gate masking)" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed.*0 filtered out' "$log"; then
  echo "EP-026 M3: FAIL - integration suite did not pass (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

tail -8 "$log"
echo "EP-026 M3: ok"
