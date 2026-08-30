#!/usr/bin/env sh
# RX-005 remediation battery: real PostgreSQL/NATS live-fire + Temporal
# retry classification truth (AUD-007, AUD-008, AUD-023).
#
# AUD-007: nexus-pg adapters (memory/world_graph/vector/outbox/inbox) proven
#          against a real pgvector/pgvector:pg18 container.
# AUD-008: NATS checkpoint persistence + outbox/inbox implementations proven
#          against a real nats:2.14.3 container and real PostgreSQL.
# AUD-023: Temporal retry classification proven by the workflows + temporal
#          suites (nonRetryableErrorTypes + classified ApplicationFailure).
set -eu
cd "$(dirname "$0")/.."
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

pass=0
fail=0
note() { echo "ok - $1"; pass=$((pass + 1)); }
bad() { echo "FAIL - $1"; fail=$((fail + 1)); }

# --- AUD-007 + AUD-008 (PG portion): nexus-pg live-fire adapters ---
out=$(cargo test -p nexus-pg 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 1 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "nexus-pg live-fire adapters ($n tests, real pgvector/pgvector:pg18)"
else
  bad "nexus-pg live-fire adapters"
fi

# --- AUD-008 (NATS portion): nexus-nats checkpoint live-fire ---
out=$(cargo test -p nexus-nats 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 1 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "nexus-nats checkpoint+consumer live-fire ($n tests, real nats:2.14.3)"
else
  bad "nexus-nats checkpoint+consumer live-fire"
fi

# --- data + events unit suites (ports touched by AUD-007/AUD-008) ---
out=$(cargo test -p nexus-data -p nexus-events 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 1 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "nexus-data + nexus-events suites ($n tests)"
else
  bad "nexus-data + nexus-events suites"
fi

# --- AUD-023: workflows retry policy suite ---
out=$(pnpm --filter @nexus/workflows test:unit 2>&1 | python3 -c "import re,sys; print(re.sub(r'\x1b\[[0-9;]*m', '', sys.stdin.read()))" || true)
n=$(echo "$out" | grep -oE "Tests +[0-9]+ passed" | grep -oE "[0-9]+" | tail -1)
if [ -n "$n" ] && [ "$n" -ge 1 ] && ! echo "$out" | grep -qE "failed \("; then
  note "workflows retry policy suite ($n tests)"
else
  bad "workflows retry policy suite"
fi

# --- AUD-023: temporal retry classification suite ---
out=$(pnpm --filter @nexus/temporal test:unit 2>&1 | python3 -c "import re,sys; print(re.sub(r'\x1b\[[0-9;]*m', '', sys.stdin.read()))" || true)
n=$(echo "$out" | grep -oE "Tests +[0-9]+ passed" | grep -oE "[0-9]+" | tail -1)
if [ -n "$n" ] && [ "$n" -ge 1 ] && ! echo "$out" | grep -qE "failed \("; then
  note "temporal retry classification suite ($n tests)"
else
  bad "temporal retry classification suite"
fi

# --- AUD-023: REAL Temporal server E2E (TESTING.md line 36) ---
# The interceptor/retry classification must be proven at the real activity
# boundary: NexusWorkflowError thrown through the real worker+interceptor
# against temporalio/server:1.31.2. Unit tests with a next() double are
# NOT sufficient (GAP-002d). Full suite: integration + failure + new
# retry-classification real-server proofs.
out=$(pnpm --filter @nexus/workflows-tests test:integration 2>&1 | python3 -c "import re,sys; print(re.sub(r'\x1b\[[0-9;]*m', '', sys.stdin.read()))" || true)
n=$(echo "$out" | grep -oE "Tests +[0-9]+ passed" | grep -oE "[0-9]+" | tail -1)
if [ -n "$n" ] && [ "$n" -ge 1 ] && ! echo "$out" | grep -qE "failed \("; then
  note "temporal real-server E2E suite ($n tests, temporalio/server:1.31.2)"
else
  bad "temporal real-server E2E suite (see full output below)"
  echo "$out" | tail -30
fi

# --- typechecks ---
if cargo check --workspace >/tmp/rx005-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx005-check.log)"
fi
if pnpm --filter @nexus/temporal typecheck >/tmp/rx005-tc.log 2>&1; then
  note "temporal typecheck clean"
else
  bad "temporal typecheck (see /tmp/rx005-tc.log)"
fi

# --- remediation register must pass (90/90, quarantine active) ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
fi

echo "---"
echo "RX-005 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
