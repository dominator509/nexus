#!/usr/bin/env sh
# EP-036 M1 gate: run the nexus-compute contract suite through the REAL
# cargo machinery with vacuity guards.
#
# The M1 changed-file fence is crates/nexus-compute/ (compute fabric
# contract crate) + providers/digitalocean/ (provider binding root), so
# the authoritative gate is the crate suite (cargo test -p nexus-compute
# -p nexus-provider-digitalocean) plus dependency-direction proof and
# clippy -D warnings. Vacuity guards are required: `cargo test -t
# <filter>` exits 0 on a zero-match filter (EP-001 gate-masking class),
# so a green M1 must observe real non-zero passing counts, EP-036-owned
# test names, and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep036-m1-tests.log"
: > "$log"

fail() {
  echo "EP-036 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-036 M1 gate: $1"; }

# Vacuity guard 0: the crate must exist with its owned sources.
if [ ! -f crates/nexus-compute/Cargo.toml ]; then
  fail "crates/nexus-compute/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/error.rs \
  src/vocabulary.rs \
  src/model.rs \
  src/placement.rs \
  src/port.rs; do
  if [ ! -f "crates/nexus-compute/$f" ]; then
    fail "crates/nexus-compute/$f missing"
  fi
done
if [ ! -f providers/digitalocean/Cargo.toml ]; then
  fail "providers/digitalocean/Cargo.toml missing"
fi
if [ ! -f providers/digitalocean/src/lib.rs ]; then
  fail "providers/digitalocean/src/lib.rs missing"
fi
ok "nexus-compute + providers/digitalocean crate and sources present"

# Real test run through cargo, captured to the log for raw sentinels
# (rtk-tee compresses interactive cargo output).
if ! sh -c 'cargo test -p nexus-compute -p nexus-provider-digitalocean --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test failed" "$log"
fi

# Vacuity guard 1: every suite reported a non-zero pass.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: zero failures observed.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 3: zero ignored tests (no required test may be skipped).
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 4: the dependency-direction proof ran and passed.
if ! grep -q 'ep036_unit_dependency_direction' "$log"; then
  fail "dependency-direction test did not run" "$log"
fi

# Vacuity guard 5 (anti-masking): EP-036-owned contract tests observed.
for sentinel in \
  ep036_unit_vocabulary_rejects_unknown_provider \
  ep036_unit_resource_state_ladder_full \
  ep036_unit_placement_fails_closed_when_no_eligible_target \
  ep036_unit_placement_never_crosses_privacy_boundary \
  ep036_unit_ambiguous_outcome_requires_reconciliation \
  ep036_unit_credential_ref_rejects_secret_shape \
  ep036_unit_receipt_never_overclaims_readiness \
  ep036_unit_provider_api_health_distinct_from_resource_health \
  ep036_unit_workload_assignment_is_not_runtime_truth \
  ep036_unit_digitalocean_binding_is_provider_kind_do; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-036-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all EP-036-owned M1 proofs observed"

total=$(grep -oE 'test result: ok\. [1-9][0-9]* passed' "$log" | awk '{s+=$4} END {print s}')
ok "real contract suite passed (${total} tests total)"

# Native compile/typecheck gate: clippy -D warnings must be clean.
if ! sh -c 'cargo clippy -p nexus-compute -p nexus-provider-digitalocean --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

# Native format gate.
if ! cargo fmt -p nexus-compute -p nexus-provider-digitalocean -- --check >>"$log" 2>&1; then
  fail "cargo fmt --check failed" "$log"
fi
ok "cargo fmt clean"

echo "EP-036 M1 gate: ok"
