#!/usr/bin/env sh
# EP-029 M4 gate: forced failures, abuse cases, and observability.
#
# The M4 changed-file fence is infra/postiz/ (failure e2e suite +
# operations diagnostic), the gate itself, the node script, and plan
# files. The authoritative gate is the nexus-postiz-e2e failure suite
# exercising REAL failure mechanisms over REAL std::net sockets against
# the production adapters/transports (never mocked), plus the M1/M2/M3
# regressions and the fail-closed ops diagnostic.
#
# Vacuity guards are required (EP-001 gate-masking class): a green M4
# must observe real non-zero passing counts, the ep029_failure_* test
# names, zero ignored/filtered tests, and the ops diagnostic present +
# fail-closed.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep029-m4-tests.log"
: > "$log"

fail() {
  echo "EP-029 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-029 M4 gate: $1"; }

# Vacuity guard 0: the failure crate and ops diagnostic must exist.
if [ ! -f infra/postiz/e2e/Cargo.toml ]; then
  fail "infra/postiz/e2e/Cargo.toml missing"
fi
if [ ! -f infra/postiz/postiz-diag.sh ]; then
  fail "infra/postiz/postiz-diag.sh missing"
fi
ok "failure crate and ops diagnostic present"

# Real build + full failure suite (all targets).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --locked -p nexus-postiz-e2e --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-postiz-e2e --all-targets failed" "$log"
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

# Vacuity guard 3 (anti-masking): an EP-029-owned failure sentinel must
# be observed. This fails if the gate accidentally executes only a
# prior node's tests or a zero-match filter.
if ! grep -q 'ep029_failure_policy_denied_zero_transport_calls .* ok' "$log"; then
  fail "EP-029-owned failure sentinel did not run (anti-masking guard)" "$log"
fi
if ! grep -q 'ep029_failure_redaction_canary_zero_leakage .* ok' "$log"; then
  fail "EP-029-owned redaction sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
if grep -q 'filtered out' "$log" && ! grep -q '0 filtered out' "$log"; then
  fail "required tests were filtered (vacuity guard)" "$log"
fi
ok "real failure suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# Ops diagnostic fail-closed check: unreachable endpoint must exit
# non-zero (never "healthy" from config existence).
port=1
if sh infra/postiz/postiz-diag.sh "http://127.0.0.1:$port" >/tmp/ep029-diag.log 2>&1; then
  fail "ops diagnostic reported healthy for unreachable endpoint (fail-closed violation)"
fi
grep -q "reachable=no" /tmp/ep029-diag.log || fail "ops diagnostic did not report reachable=no for unreachable endpoint"
ok "ops diagnostic fails closed on unreachable endpoint"

# M1 + M2 + M3 regressions: the contract, Postiz adapter, and direct
# connector crates stay green.
for crate in nexus-social nexus-postiz-connector nexus-social-direct-connector; do
  if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --locked -p "$crate" --all-targets >>"$log" 2>&1; then
    fail "$crate regression failed" "$log"
  fi
done
ok "M1 + M2 + M3 regressions green"

# Milestone artifact/fence checks: M4 fence paths exist.
for f in .agent/milestone-files/EP-029-M4.txt .agent/node-contracts/EP-029.md \
         .agent/execplans/EP-029-social-command-center.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-029 M4: ok"
