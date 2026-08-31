#!/usr/bin/env sh
# RX-006 remediation battery: Headscale mesh identity truth (AUD-012).
#
# AUD-012: register_node() must bind the caller-supplied WireGuard public
# key (never a synthetic random mkey); wireguard_config() must resolve the
# private-key reference through a REAL secret store or fail closed (never
# fabricate); the live proof must use REAL X25519 key material stored in
# REAL OpenBao with a cryptographic binding check.
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

# --- AUD-012 unit regressions: identity binding + fail-closed reference ---
out=$(cargo test -p nexus-headscale 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 1 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "nexus-headscale unit regressions ($n tests: placeholder/empty keys rejected, no-store fail-closed, unresolved reference fail-closed)"
else
  bad "nexus-headscale unit regressions"
  echo "$out" | tail -20
fi

# --- AUD-012: the rx006 hostile unit filters must exist and pass ---
out=$(cargo test -p nexus-headscale rx006_ 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 6 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "rx006 hostile unit set ($n tests)"
else
  bad "rx006 hostile unit set"
  echo "$out" | tail -20
fi

# --- AUD-012: nexus-trust mesh contract tests stay green ---
out=$(cargo test -p nexus-trust 2>&1 || true)
n=$(echo "$out" | grep -oE "test result: ok\. [0-9]+ passed" | awk '{s += $4} END {print s+0}')
if [ "$n" -ge 1 ] && ! echo "$out" | grep -qE "test result: FAILED"; then
  note "nexus-trust suite ($n tests)"
else
  bad "nexus-trust suite"
  echo "$out" | tail -20
fi

# --- AUD-012: REAL Headscale + REAL OpenBao integration live-fire ---
# The adapter live proof registers with real X25519 keys, resolves the
# mesh private-key reference against real OpenBao, and asserts the
# cryptographic binding (stored mesh_key == registered identity).
out=$(uv run --frozen pytest tests/trust -q --tb=short -o python_functions="ep009_integration_headscale_*" 2>&1 || true)
n=$(echo "$out" | grep -oE "[0-9]+ passed" | grep -oE "[0-9]+" | tail -1)
if [ -n "$n" ] && [ "$n" -ge 10 ] && ! echo "$out" | grep -qE "failed|error"; then
  note "ep009 headscale+openbao integration ($n tests, real server + real key binding)"
else
  bad "ep009 headscale+openbao integration"
  echo "$out" | tail -25
fi

# --- orphan audit: no headscale/openbao leftovers ---
if sh scripts/ep009-orphan-audit.sh >/tmp/rx006-orphan.log 2>&1; then
  note "EP-009 orphan audit clean"
else
  bad "EP-009 orphan audit (see /tmp/rx006-orphan.log)"
fi

# --- workspace check + clippy ---
if cargo check --workspace >/tmp/rx006-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx006-check.log)"
fi
if cargo clippy -p nexus-headscale --all-targets >/tmp/rx006-clippy.log 2>&1; then
  note "nexus-headscale clippy clean"
else
  bad "nexus-headscale clippy (see /tmp/rx006-clippy.log)"
fi

# --- remediation register must pass (90/90, quarantine active) ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
fi

echo "---"
echo "RX-006 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
