#!/usr/bin/env sh
# EP-028 M3 gate: run the nexus-hydra-connector integration suite
# through the REAL cargo test machinery with vacuity guards.
#
# The M3 changed-file fence is schemas/hydra/ (canonical JSON Schemas)
# plus the connector's real-socket integration tests (tests/
# ep028_m3_transport.rs) under the already-owned connectors/hydra/
# directory. The authoritative gate is the nexus-hydra-connector cargo
# suite (unit + integration over REAL std::net sockets against a
# controlled local HTTP fixture emitting REAL Hydra-shaped responses)
# plus the M1/M2 regressions. Vacuity guards are required: `cargo test
# <filter>` exits 0 on a zero-match filter (EP-001 gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep028-m3-tests.log"
: > "$log"

fail() {
  echo "EP-028 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-028 M3 gate: $1"; }

# Vacuity guard 0: canonical schemas must exist and parse.
for f in schemas/hydra/business-context.schema.json \
         schemas/hydra/context-projection.schema.json \
         schemas/hydra/capability-map.schema.json \
         schemas/hydra/action-request.schema.json \
         schemas/hydra/event-envelope.schema.json; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
  if ! python3 -c "import json,sys; json.load(open('$f'))"; then
    fail "$f is not valid JSON"
  fi
done
ok "canonical Hydra schemas present and parse"

# Real build + full connector suite (all targets: unit + integration).
if ! cargo test --locked -p nexus-hydra-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-hydra-connector --all-targets failed" "$log"
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

# Vacuity guard 3: the integration suite binary ran (REAL sockets).
if ! grep -qE 'Running tests/ep028_m3_transport\.rs' "$log"; then
  fail "integration suite did not run (gate masking)" "$log"
fi

# Vacuity guard 4 (anti-masking): an EP-028-owned integration sentinel
# must be observed. Fails if the gate accidentally executes only prior
# nodes' tests.
if ! grep -q 'ep028_integration_read_context_real_http .* ok' "$log"; then
  fail "EP-028-owned integration sentinel did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 5: schema parity tests ran (real Rust serialization
# validated against the canonical schemas).
if ! grep -q 'ep028_integration_context_projection_matches_canonical_schema .* ok' "$log"; then
  fail "schema parity test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 6: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
ok "real suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# M1/M2 regressions.
if ! cargo test --locked -p nexus-hydra --all-targets >>"$log" 2>&1; then
  fail "M1 contract regression failed" "$log"
fi
if ! cargo test --locked -p nexus-hydra-connector --lib >>"$log" 2>&1; then
  fail "M2 adapter-core lib regression failed" "$log"
fi
ok "M1 + M2 regressions green"

# Milestone artifact/fence checks.
if [ ! -f .agent/milestone-files/EP-028-M3.txt ]; then
  fail ".agent/milestone-files/EP-028-M3.txt missing"
fi
ok "milestone fence present"

echo "EP-028 M3: ok"
