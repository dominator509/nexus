#!/usr/bin/env sh
# EP-036 M4 gate: forced-failure suite (providers/contabo binding crate +
# tests/infra real failure mechanisms) through the REAL cargo machinery
# with vacuity guards.
#
# The M4 changed-file fence is providers/contabo/ (Contabo provider
# binding identity) + tests/infra/ (forced-failure suite), so the
# authoritative gate is the Contabo crate suite + the failure suite
# (including the REAL ephemeral sshd container termination and real
# ssh-keyscan timeout proofs), plus M1+M2+M3 regressions.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep036-m4-tests.log"
: > "$log"

fail() {
  echo "EP-036 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-036 M4 gate: $1"; }

# Vacuity guard 0: the owned roots must exist.
if [ ! -f providers/contabo/Cargo.toml ]; then
  fail "providers/contabo/Cargo.toml missing"
fi
if [ ! -f providers/contabo/src/lib.rs ]; then
  fail "providers/contabo/src/lib.rs missing"
fi
if [ ! -f tests/infra/Cargo.toml ]; then
  fail "tests/infra/Cargo.toml missing"
fi
if [ ! -f tests/infra/tests/ep036_failure_suite.rs ]; then
  fail "tests/infra/tests/ep036_failure_suite.rs missing"
fi
ok "contabo + failure-suite roots present"

# Real failure-suite + contabo suite (--nocapture so the real transport
# sentinels are observable).
if ! sh -c 'cargo test -p nexus-infra-failure-tests -p nexus-provider-contabo --locked -- --nocapture >> "$1" 2>&1' _ "$log"; then
  fail "cargo test -p nexus-infra-failure-tests -p nexus-provider-contabo failed" "$log"
fi

# Vacuity guard 1: non-zero pass observed.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: zero failures observed.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 3: zero ignored tests.
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 4 (anti-masking): EP-036-owned failure proofs observed,
# including the REAL mechanisms (terminated container, real timeout)
# and the Contabo binding proofs.
for sentinel in \
  ep036_failure_unavailable_dependency_terminated_container \
  ep036_failure_timeout_probe_fails_closed \
  ep036_failure_malformed_input_rejected \
  ep036_failure_duplicate_request_requires_reconciliation \
  ep036_failure_denied_permission_placement_fails_closed \
  ep036_failure_cancelled_work_delete_before_create_fails_closed \
  ep036_failure_partial_side_effect_receipt_no_overclaim \
  ep036_unit_contabo_region_code_shape \
  ep036_unit_contabo_binding_is_provider_kind_contabo; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-036-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
if grep -q "docker unavailable; skipping" "$log"; then
  fail "failure suite silently skipped docker (vacuity guard)" "$log"
fi
ok "all EP-036-owned M4 proofs observed"

total=$(grep -oE 'test result: ok\. [1-9][0-9]* passed' "$log" | awk '{s+=$4} END {print s}')
ok "real failure + contabo suites passed (${total} tests total)"

# Native compile/typecheck + format for the new crates.
if ! sh -c 'cargo clippy -p nexus-provider-contabo -p nexus-infra-failure-tests --locked --all-targets -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"
if ! cargo fmt -p nexus-provider-contabo -p nexus-infra-failure-tests -- --check >>"$log" 2>&1; then
  fail "cargo fmt --check failed" "$log"
fi
ok "cargo fmt clean"

# M1 + M2 + M3 regressions: the compute fabric contract, AWS/OpenTofu,
# and existing-SSH/cloud-init work must remain green.
sh scripts/ep036-m1-tests.sh >>"$log" 2>&1 || fail "M1 regression failed" "$log"
ok "EP-036 M1 regression green"
sh scripts/ep036-m2-tests.sh >>"$log" 2>&1 || fail "M2 regression failed" "$log"
ok "EP-036 M2 regression green"
sh scripts/ep036-m3-tests.sh >>"$log" 2>&1 || fail "M3 regression failed" "$log"
ok "EP-036 M3 regression green"

echo "EP-036 M4 gate: ok"
