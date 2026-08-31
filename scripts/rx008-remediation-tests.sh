#!/usr/bin/env sh
# RX-008 remediation battery: runtime telemetry bootstrap (AUD-083) +
# canonical control-plane composition root with SPEC-003 surfaces
# (AUD-084).
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

# --- AUD-083: telemetry unit regressions (context init, no tenant leak) ---
out=$(cargo test -p nexus-control-plane telemetry 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 4 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "telemetry unit regressions ($n tests: context init, empty-component fail-closed, structured exportable line, tenant never leaks)"
else
  bad "telemetry unit regressions"
  echo "$out" | tail -25
fi

# --- AUD-084: composition + router unit regressions ---
out=$(cargo test -p nexus-control-plane composition 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 5 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "composition unit regressions ($n tests: descriptor, MCP session, A2A lifecycle, artifact hash-bound, outbox, router surfaces)"
else
  bad "composition unit regressions"
  echo "$out" | tail -25
fi

# --- AUD-083/084: live-fire integration over real HTTP ---
out=$(cargo test -p nexus-control-plane --test ep044_integration_http 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 10 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "ep044 integration live-fire ($n tests: healthz/readyz/capabilities/discover, MCP init/list/call, A2A submit/run/stream, artifact publish/fetch + fabricated-404, events append/pending)"
else
  bad "ep044 integration live-fire"
  echo "$out" | tail -25
fi

# --- A2A crate stays green (gateway port bound changed) ---
out=$(cargo test -p nexus-a2a 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 1 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "nexus-a2a suite ($n tests)"
else
  bad "nexus-a2a suite"
  echo "$out" | tail -25
fi

# --- workspace check + clippy ---
if cargo check --workspace >/tmp/rx008-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx008-check.log)"
fi
if cargo clippy -p nexus-control-plane -p nexus-a2a --all-targets >/tmp/rx008-clippy.log 2>&1; then
  note "control-plane + a2a clippy clean"
else
  bad "clippy (see /tmp/rx008-clippy.log)"
fi

# --- remediation register must pass (90/90, quarantine active) ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
fi

echo "---"
echo "RX-008 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
