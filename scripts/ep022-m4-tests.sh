#!/usr/bin/env sh
# EP-022 M4 gate: run the REAL nexus-bluetooth-audio forced-failure
# suite through the REAL cargo test machinery with vacuity guards.
#
# The M4 changed-files fence is connectors/bluetooth-audio/ (Rust crate
# nexus-bluetooth-audio). The suite exercises REAL failure mechanisms:
# the real system bus (org.bluez NameHasNoOwner), real unreachable /
# silent / garbage / auth-rejecting peers, the pure connector state
# machine, policy denial, and payload redaction.
#
# Vacuity guards (EP-001 gate-masking class):
# 1. `cargo test <filter>` exits 0 on a zero-match filter, so at least
#    one ep022_failure test must have run and passed.
# 2. The two REAL system-bus tests must be present and green; without
#    them the suite could pass without proving the real mechanism.
# 3. The ops diagnostic binary must run against the REAL bus and report
#    bluez absent - real observed output, not a canned string.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep022-m4-tests.log"
: > "$log"

if ! cargo test --locked -p nexus-bluetooth-audio ep022_failure >>"$log" 2>&1; then
  echo "EP-022 M4: FAIL - cargo test ep022_failure failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# Vacuity guard 1: tests ran and passed with a non-zero count.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-022 M4: FAIL - no tests matched ep022_failure (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-022 M4: FAIL - no passing ep022_failure tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Vacuity guard 2: the real system-bus tests must be present and green.
for test_name in \
  ep022_failure_canary_probe_is_live_on_real_bus \
  ep022_failure_bluez_absent_on_real_system_bus; do
  if ! grep -qE "^test ${test_name} \\.\\.\\. ok$" "$log"; then
    echo "EP-022 M4: FAIL - real system-bus test ${test_name} missing or not ok (vacuity guard)" >&2
    tail -20 "$log" >&2
    exit 1
  fi
done

# Vacuity guard 3: the ops diagnostic reports the real observation.
diag="$(cargo run --quiet --locked -p nexus-bluetooth-audio --bin bluetooth-diag -- status 2>>"$log")" || {
  echo "EP-022 M4: FAIL - bluetooth-diag status exited non-zero" >&2
  tail -10 "$log" >&2
  exit 1
}
case "$diag" in
  *'"bluez":"absent"'*) ;;
  *)
    echo "EP-022 M4: FAIL - bluetooth-diag did not report bluez absent" >&2
    echo "$diag" >&2
    exit 1
    ;;
esac

tail -8 "$log"
echo "EP-022 M4: ok"
