#!/usr/bin/env sh
# EP-035 M2 gate: run the nexus-setup behavior suite through the REAL
# cargo machinery with vacuity guards.
#
# The M2 changed-file fence is crates/nexus-setup/ plus workspace
# manifests, so the authoritative gate is the crate suite (cargo test
# -p nexus-setup) plus dependency-direction proof and clippy -D
# warnings. Vacuity guards are required: `cargo test -t <filter>` exits
# 0 on a zero-match filter (EP-001 gate-masking class), so a green M2
# must observe real non-zero passing counts, EP-035-owned test names,
# and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep035-m2-tests.log"
: > "$log"

fail() {
  echo "EP-035 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-035 M2 gate: $1"; }

# Vacuity guard 0: the crate must exist with its owned sources.
if [ ! -f crates/nexus-setup/Cargo.toml ]; then
  fail "crates/nexus-setup/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/error.rs \
  src/vocabulary.rs \
  src/model.rs \
  src/wizard.rs \
  src/port.rs; do
  if [ ! -f "crates/nexus-setup/$f" ]; then
    fail "crates/nexus-setup/$f missing"
  fi
done
ok "nexus-setup crate and sources present"

# Real test run through cargo, captured to the log for raw sentinels
# (rtk-tee compresses interactive cargo output).
if ! sh -c 'cargo test -p nexus-setup --locked >> "$1" 2>&1' _ "$log"; then
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
if ! grep -q 'ep035_unit_dependency_direction' "$log"; then
  fail "dependency-direction test did not run" "$log"
fi

# Vacuity guard 5 (anti-masking): EP-035-owned behavior tests observed.
for name in \
  ep035_unit_wizard_completes_only_when_every_step_verified \
  ep035_unit_deployment_selection_is_unverified_always \
  ep035_unit_hardware_user_declared_gpu_is_not_detected \
  ep035_unit_owner_competing_request_is_conflict \
  ep035_unit_enrollment_secret_never_appears_in_any_surface \
  ep035_unit_discovery_hostile_content_is_data_never_authority \
  ep035_unit_integration_credential_exists_never_mints_healthy \
  ep035_unit_recovery_ambiguous_forces_reconcile_never_blind_retry; do
  if ! grep -q "$name" "$log"; then
    fail "EP-035-owned test '$name' did not run (anti-masking guard)" "$log"
  fi
done
ok "EP-035-owned behavior proofs observed"

# Guard 6: clippy -D warnings clean.
if ! cargo clippy -p nexus-setup --all-targets -- -D warnings >>"$log" 2>&1; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

# Guard 7: rustfmt clean on the crate.
if ! cargo fmt -p nexus-setup -- --check >>"$log" 2>&1; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

total=$(grep -oE 'test result: ok\. [1-9][0-9]* passed' "$log" | awk '{s+=$4} END {print s}')
ok "real suite passed (${total} tests total)"

# Milestone artifact/fence checks.
for f in .agent/milestone-files/EP-035-M2.txt .agent/node-contracts/EP-035.md \
         .agent/execplans/EP-035-setup-wizard-and-onboarding.md crates/nexus-setup/Cargo.toml; do
  if [ ! -e "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-035 M2: ok"
