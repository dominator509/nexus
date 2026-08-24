#!/usr/bin/env sh
# EP-040 M4 gate: forced failures, abuse cases, and observability for
# security + hardware certification behavior.
#
# M4 owns tests/security/ (real secret-literal scanning, redaction,
# authorization, insecure-config rejection, scanner-unavailable
# capability blocking, stale/empty evidence rejection, mock-only
# distinction, and real abuse-case injection: terminate a real provider
# container, revoke a runtime token, corrupt a controlled message,
# exhaust a declared budget, deny a policy decision) and tests/hardware/
# (simulator-vs-real distinction, device identity ladder, fake-device
# rejection, missing-hardware capability blocking, stale evidence).
#
# The gate executes the REAL cargo machinery against REAL mechanisms.
# Vacuous green is impossible: every required proof must be observed by
# name with a non-zero passing count and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

# Ensure cargo is available to `sh -c` subshells.
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep040-m4-tests.log"
: > "$log"

fail() {
  echo "EP-040 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-040 M4 gate: $1"; }

# Vacuity guard 0: the owned crates must exist with their owned sources.
if [ ! -f tests/security/Cargo.toml ]; then
  fail "tests/security/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/scanner.rs \
  src/policy.rs \
  src/evidence.rs \
  src/abuse.rs \
  tests/ep040_m4_security.rs; do
  if [ ! -f "tests/security/$f" ]; then
    fail "tests/security/$f missing"
  fi
done
if [ ! -f tests/hardware/Cargo.toml ]; then
  fail "tests/hardware/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/device.rs \
  src/certifier.rs \
  tests/ep040_m4_hardware.rs; do
  if [ ! -f "tests/hardware/$f" ]; then
    fail "tests/hardware/$f missing"
  fi
done
ok "security + hardware crates and M4-owned sources present"

# Vacuity guard 0b: the workspace declares both crate members.
if ! grep -q 'tests/security' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/security member"
fi
if ! grep -q 'tests/hardware' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/hardware member"
fi
ok "workspace members declared"

# Vacuity guard 1: the real provider transport is available for the
# terminate-container abuse proof (composed from the M3 crate).
if [ ! -f tests/provider-certification/src/transport.rs ]; then
  fail "M3 provider transport missing (abuse proof dependency)"
fi
if ! grep -q 'nexus-provider-certification' tests/security/Cargo.toml; then
  fail "security crate must compose the M3 real provider transport"
fi
ok "real provider transport composed for terminate-container abuse proof"

# Vacuity guard 2: docker CLI must actually be present and answer (the
# terminate-container proof spawns a real container through docker).
if ! docker version >/dev/null 2>&1; then
  fail "docker CLI unavailable; real-container abuse proof cannot run"
fi
ok "docker CLI available"

# Real test run through cargo (rtk-tee compresses interactive output, so
# capture raw output to the log and grep real sentinels).
if ! sh -c 'cargo test -p nexus-security-core -p nexus-hardware-certification --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test failed" "$log"
fi

# Vacuity guard 3: every suite reported a non-zero pass.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 4: zero failures observed.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 5: zero ignored tests (no required test may be skipped).
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 6 (anti-masking): every required M4 proof observed by name.
# One sentinel per behavior family.
for sentinel in \
  ep040_failure_security_secret_literal_detected \
  ep040_failure_security_missing_scan_target_fails_closed \
  ep040_failure_security_zero_findings_not_automatically_green \
  ep040_failure_security_mock_scan_never_certifies \
  ep040_failure_security_strict_scan_denies \
  ep040_failure_security_denied_permission_fails_closed \
  ep040_failure_security_authorization_no_broad_bypass \
  ep040_failure_security_insecure_config_rejected \
  ep040_failure_security_stale_evidence_rejected \
  ep040_failure_security_empty_evidence_never_green \
  ep040_failure_security_redaction_proof \
  ep040_failure_security_terminate_container_fails_closed \
  ep040_failure_security_revoked_token_denied \
  ep040_failure_security_corrupt_message_fails_closed \
  ep040_failure_security_exhaust_budget_fails_closed \
  ep040_failure_security_budget_within_bound_succeeds \
  ep040_failure_security_observability_redacted \
  ep040_failure_security_runtime_token_helper \
  ep040_failure_hardware_display_name_only_rejected \
  ep040_failure_hardware_declared_never_observed \
  ep040_failure_hardware_simulator_never_certifies \
  ep040_failure_hardware_observed_never_exercised \
  ep040_failure_hardware_missing_hardware_capability_blocked \
  ep040_failure_hardware_exercised_requires_acceptance \
  ep040_failure_hardware_identity_binding_enforced \
  ep040_failure_hardware_incomplete_observation_rejected \
  ep040_failure_hardware_certification_requires_evidence \
  ep040_failure_hardware_fake_device_rejected \
  ep040_failure_hardware_certification_ladder_distinct \
  ep040_failure_hardware_verdict_serialization_honest; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-040-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "all 30 EP-040 M4-owned proofs observed"

# Vacuity guard 7: real mechanisms actually wired - docker rm -f in the
# abuse module, runtime token revocation, controlled corruption, budget
# exhaustion, and deny-default policy.
if ! grep -q 'docker rm -f' tests/security/src/abuse.rs; then
  fail "abuse module does not terminate the real container"
fi
if ! grep -q 'fn revoke' tests/security/src/abuse.rs; then
  fail "abuse module does not implement token revocation"
fi
if ! grep -q 'fn corrupt_controlled_message' tests/security/src/abuse.rs; then
  fail "abuse module does not implement controlled corruption"
fi
if ! grep -q 'fn exhaust_declared_budget' tests/security/src/abuse.rs; then
  fail "abuse module does not implement budget exhaustion"
fi
if ! grep -q 'fn require' tests/security/src/policy.rs; then
  fail "policy module does not implement deny-default authorization"
fi
ok "real abuse-case mechanisms wired"

# Vacuity guard 8: no placeholder content in the M4 crates.
# NOTE: "fake" is legitimate canonical vocabulary here (fake-device
# rejection is a required M4 proof), so the scan targets actual
# placeholder markers only.
if grep -rqiE 'placeholder|TODO|FIXME|sample only|not implemented yet' tests/security/src tests/hardware/src; then
  fail "M4 crate contains placeholder content"
fi
ok "M4 crate content validated"

# Vacuity guard 9: dependency direction - the M4 crates depend only on
# canonical M1/M2/M3 surfaces plus serde.
bad_dep=$(cargo tree -p nexus-security-core --depth 1 2>/dev/null | grep -vE 'nexus-security-core|nexus-test-contract|nexus-provider-certification|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-security-core: $bad_dep"
fi
bad_dep=$(cargo tree -p nexus-hardware-certification --depth 1 2>/dev/null | grep -vE 'nexus-hardware-certification|nexus-test-contract|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-hardware-certification: $bad_dep"
fi
ok "dependency-direction clean (canonical surfaces only)"

# Clippy -D warnings and fmt on the owned crates.
if ! sh -c 'cargo clippy -p nexus-security-core -p nexus-hardware-certification --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-security-core -p nexus-hardware-certification -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# License/security of the crates themselves: declared MIT.
if ! grep -q '^license = "MIT"' tests/security/Cargo.toml; then
  fail "nexus-security-core license must be MIT"
fi
if ! grep -q '^license = "MIT"' tests/hardware/Cargo.toml; then
  fail "nexus-hardware-certification license must be MIT"
fi
ok "crate licenses declared (MIT)"

# Vacuity guard 10: resource hygiene - zero EP-040-owned containers and
# zero M4 temp evidence roots remain after the real runs (plain docker ps
# output; no format templates that could be mistaken for placeholders).
leftover=$(docker ps -a | awk '{print $NF}' | grep '^nexus-ep040-m3-' || true)
if [ -n "$leftover" ]; then
  fail "EP-040 M3/M4 containers left running: $leftover"
fi
leftover_evid=$(ls -d /tmp/ep040-m4-sec-* /tmp/ep040-m4-hw-* 2>/dev/null || true)
if [ -n "$leftover_evid" ]; then
  fail "EP-040 M4 temp evidence residue: $leftover_evid"
fi
ok "resource hygiene verified (zero EP-040-owned containers/temp evidence)"

# M1 regression: the contract + performance suites must stay green.
if ! sh -c 'cargo test -p nexus-test-contract -p nexus-test-performance --locked >> "$1" 2>&1' _ "$log"; then
  fail "M1 regression failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "M1 regression ran no tests (vacuity guard)" "$log"
fi
ok "M1 regression green"

# M2 regression: the execution + accessibility suites must stay green.
if ! sh -c 'cargo test -p nexus-test-execution -p nexus-accessibility-audit --locked >> "$1" 2>&1' _ "$log"; then
  fail "M2 regression failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "M2 regression ran no tests (vacuity guard)" "$log"
fi
ok "M2 regression green"

# M3 regression: provider certification + e2e transport must stay green
# (these spawn real containers; docker is already verified available).
if ! sh -c 'cargo test -p nexus-provider-certification -p nexus-e2e-transport --locked >> "$1" 2>&1' _ "$log"; then
  fail "M3 regression failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "M3 regression ran no tests (vacuity guard)" "$log"
fi
ok "M3 regression green"

echo "EP-040 M4 gate: ok"
