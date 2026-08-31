#!/usr/bin/env sh
# RX-007 remediation battery: sandboxed skill execution truth (AUD-011)
# and bounded, deadlock-free subprocess execution (AUD-022).
#
# AUD-011: SkillExecutor subprocess on Linux must be a REAL OS sandbox
# (namespaces, read-only host, bounded /tmp, privilege drop to nobody,
# no_new_privs, seccomp deny-list), proven by hostile payloads that
# probe the sandbox from the inside.
# AUD-022: SkillExecutor and ProcessRunner must drain stdout/stderr
# concurrently, enforce a wall-clock deadline, and surface timeouts as
# observable results - never a deadlock, never a fabricated success.
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

# --- AUD-011 + AUD-022: nexus-skills hostile battery ---
out=$(cargo test -p nexus-skills --test rx007_remediation 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 9 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "nexus-skills rx007 hostile set ($n tests: sandbox uid drop, host read-only, private netns, seccomp+nonwprivs, stderr-flood no-deadlock, deadline kill, tamper/permission fail-closed)"
else
  bad "nexus-skills rx007 hostile set"
  echo "$out" | tail -25
fi

# --- AUD-022: nexus-harness-adapters rx007 set ---
out=$(cargo test -p nexus-harness-adapters --test rx007_process_runner 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 3 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "nexus-harness-adapters rx007 set ($n tests: stderr flood completes, hung child -> Timeout, fail-closed contract holds)"
else
  bad "nexus-harness-adapters rx007 set"
  echo "$out" | tail -25
fi

# --- AUD-011/022: full crate suites must stay green ---
out=$(cargo test -p nexus-skills -p nexus-harness-adapters 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 150 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "full crate suites ($n tests, nexus-skills + nexus-harness-adapters)"
else
  bad "full crate suites"
  echo "$out" | tail -25
fi

# --- workspace check + clippy ---
if cargo check --workspace >/tmp/rx007-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx007-check.log)"
fi
if cargo clippy -p nexus-skills -p nexus-harness-adapters --all-targets >/tmp/rx007-clippy.log 2>&1; then
  note "nexus-skills + nexus-harness-adapters clippy clean"
else
  bad "clippy (see /tmp/rx007-clippy.log)"
fi

# --- remediation register must pass (90/90, quarantine active) ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
fi

echo "---"
echo "RX-007 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
