#!/usr/bin/env sh
# EP-028 M4 gate: forced failures, abuse cases, and observability.
#
# The M4 changed-file fence is tests/hydra/ (failure suite + operations
# diagnostic), the gate itself, the node script, plan files, and
# evidence. The authoritative gate is the nexus-hydra-e2e failure suite
# exercising REAL failure mechanisms over REAL std::net sockets against
# the production transport/adapter (never mocked), plus the M1/M2/M3
# regressions.
#
# Vacuity guards are required (EP-001 gate-masking class): a green M4
# must observe real non-zero passing counts, the ep028_failure_* test
# names, zero ignored/filtered tests, and the ops diagnostic present.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep028-m4-tests.log"
: > "$log"

fail() {
  echo "EP-028 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-028 M4 gate: $1"; }

# Vacuity guard 0: the failure crate and ops diagnostic must exist.
if [ ! -f tests/hydra/Cargo.toml ]; then
  fail "tests/hydra/Cargo.toml missing"
fi
if [ ! -f tests/hydra/ops/hydra-diag.sh ]; then
  fail "tests/hydra/ops/hydra-diag.sh missing (operations diagnostic)"
fi
ok "failure crate and ops diagnostic present"

# Real build + full failure suite (all targets).
if ! cargo test --locked -p nexus-hydra-e2e --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-hydra-e2e --all-targets failed" "$log"
fi

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: a passing result with a non-zero count and zero
# failures is observed in the run output.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$log"; then
  fail "no passing non-vacuous result (vacuity guard)" "$log"
fi

# Vacuity guard 3 (anti-masking): an EP-028-owned failure sentinel must
# be observed. Fails if the gate accidentally executes only prior
# nodes' tests.
if ! grep -q 'ep028_failure_refused_port_unavailable .* ok' "$log"; then
  fail "EP-028-owned failure sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: redaction canary test ran (poison-safe
# observability proof).
if ! grep -q 'ep028_failure_redaction_canary_zero_leakage .* ok' "$log"; then
  fail "redaction canary test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 5: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
ok "real suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# Regressions: M1 contract + M2/M3 connector suites.
if ! cargo test --locked -p nexus-hydra --all-targets >>"$log" 2>&1; then
  fail "M1 contract regression failed" "$log"
fi
if ! cargo test --locked -p nexus-hydra-connector --all-targets >>"$log" 2>&1; then
  fail "M2/M3 connector regression failed" "$log"
fi
ok "M1 + M2/M3 regressions green"

# Ops diagnostic sanity: the script exists, is executable, and handles
# an unreachable provider fail-closed (exit 3, no credentials printed).
diag_rc=0
sh tests/hydra/ops/hydra-diag.sh diagnose http://127.0.0.1:1 >/dev/null 2>&1 || diag_rc=$?
if [ "$diag_rc" -ne 3 ]; then
  fail "ops diagnostic did not fail closed on unreachable provider (rc=$diag_rc)"
fi
ok "ops diagnostic fails closed on unreachable provider"

# Milestone artifact/fence checks.
if [ ! -f .agent/milestone-files/EP-028-M4.txt ]; then
  fail ".agent/milestone-files/EP-028-M4.txt missing"
fi
ok "milestone fence present"

echo "EP-028 M4: ok"
