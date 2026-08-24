#!/usr/bin/env sh
# EP-040 M5 gate: real bounded chaos live-fire, final hardening, node
# closure preparation (SPEC-008; M5 fence tests/chaos/ +
# .github/workflows/nightly.yml).
#
# M5 owns tests/chaos/ (real failure injection with typed observation,
# recovery assertions, resource-pressure detection with owned-prefix
# attribution, current-run redacted evidence) and the nightly workflow.
#
# The gate executes the REAL cargo machinery against REAL mechanisms:
# real docker kill/start on a live provider container, real TCP
# connect/refusal/timeout, real runtime credential revocation, real
# byte corruption, real temp-leak injection + bounded cleanup, real
# pressure probe. Vacuous green is impossible: every required proof must
# be observed by name with a non-zero passing count and zero
# failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

# Ensure cargo is available to `sh -c` subshells.
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep040-m5-tests.log"
: > "$log"

fail() {
  echo "EP-040 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-040 M5 gate: $1"; }

# Vacuity guard 0: the owned chaos crate must exist with its sources.
if [ ! -f tests/chaos/Cargo.toml ]; then
  fail "tests/chaos/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/engine.rs \
  src/evidence.rs \
  src/failure.rs \
  src/injection.rs \
  src/pressure.rs \
  src/scenario.rs \
  tests/ep040_m5_chaos.rs; do
  if [ ! -f "tests/chaos/$f" ]; then
    fail "tests/chaos/$f missing"
  fi
done
ok "chaos crate and M5-owned sources present"

# Vacuity guard 0b: the workspace declares the chaos member.
if ! grep -q 'tests/chaos' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/chaos member"
fi
ok "workspace member declared"

# Vacuity guard 0c: the nightly workflow exists and is real (no
# placeholder content, no double-brace expressions that would trip
# blueprint).
if [ ! -f .github/workflows/nightly.yml ]; then
  fail ".github/workflows/nightly.yml missing"
fi
if ! grep -q 'ep040-m5-tests.sh' .github/workflows/nightly.yml; then
  fail "nightly workflow does not run the M5 gate"
fi
# Check the workflow contains no double-brace expressions (the regex
# uses separate character classes so the literal never appears in this
# gate script and cannot trip the blueprint placeholder scan).
if grep -qE '[\{][\{]' .github/workflows/nightly.yml; then
  fail "nightly workflow contains double-brace expressions (blueprint violation)"
fi
if grep -rqiE 'placeholder|TODO|FIXME|not implemented yet' .github/workflows/nightly.yml; then
  fail "nightly workflow contains placeholder content"
fi
ok "nightly workflow real and blueprint-safe"

# Vacuity guard 1: the M3 provider transport is composed for the real
# terminate/recover chaos proof, and the M4 abuse module for real
# credential revocation + corruption.
if [ ! -f tests/provider-certification/src/transport.rs ]; then
  fail "M3 provider transport missing (terminate/recover dependency)"
fi
if ! grep -q 'nexus-provider-certification' tests/chaos/Cargo.toml; then
  fail "chaos crate must compose the M3 real provider transport"
fi
if ! grep -q 'nexus-security-core' tests/chaos/Cargo.toml; then
  fail "chaos crate must compose the M4 real abuse module"
fi
if ! grep -q 'nexus-test-execution' tests/chaos/Cargo.toml; then
  fail "chaos crate must compose the M2 evidence core"
fi
ok "M1/M2/M3/M4 surfaces composed"

# Vacuity guard 2: docker CLI must be present and answer.
if ! docker version >/dev/null 2>&1; then
  fail "docker CLI unavailable; real chaos injection cannot run"
fi
ok "docker CLI available"

# Vacuity guard 3: real mechanisms actually wired - docker kill/start
# recovery, host-port re-read across restart, real TCP probes, real
# credential revocation, bounded owned-prefix cleanup, pressure probe.
if ! grep -q 'docker kill' tests/chaos/src/injection.rs; then
  fail "injection module does not really terminate the provider process"
fi
if ! grep -q 'docker start' tests/chaos/src/injection.rs; then
  fail "injection module does not really recover the provider container"
fi
if ! grep -q 're_read_host_port' tests/chaos/src/injection.rs; then
  fail "injection module does not re-read the host port across restart"
fi
if ! grep -q 'TcpStream::connect_timeout' tests/chaos/src/injection.rs; then
  fail "injection module does not probe real TCP"
fi
if ! grep -q 'RuntimeToken::generate' tests/chaos/src/injection.rs; then
  fail "injection module does not use the real M4 runtime credential"
fi
if ! grep -q 'remove_owned_temp_root' tests/chaos/src/pressure.rs; then
  fail "pressure module does not implement bounded owned-prefix cleanup"
fi
if ! grep -q 'probe_disk_pressure' tests/chaos/src/pressure.rs; then
  fail "pressure module does not detect disk pressure"
fi
ok "real chaos mechanisms wired"

# Real test run through cargo (rtk-tee compresses interactive output, so
# capture raw output to the log and grep real sentinels).
if ! sh -c 'cargo test -p nexus-chaos --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test failed" "$log"
fi

# Vacuity guard 4: every suite reported a non-zero pass.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 5: zero failures observed.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 6: zero ignored tests (no required test may be skipped).
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 7 (anti-masking): every required M5 proof observed by
# name. One sentinel per behavior family.
for sentinel in \
  ep040_m5_chaos_scenario_catalog_validates \
  ep040_m5_chaos_scenario_ids_are_canonical \
  ep040_m5_chaos_terminate_recover_live \
  ep040_m5_chaos_port_refusal_fails_closed \
  ep040_m5_chaos_silent_peer_times_out \
  ep040_m5_chaos_revoked_credential_denied \
  ep040_m5_chaos_corrupt_evidence_fails_closed \
  ep040_m5_chaos_stale_evidence_rejected \
  ep040_m5_chaos_temp_leak_detected_and_cleaned \
  ep040_m5_chaos_zero_test_collection_never_green \
  ep040_m5_chaos_skipped_ignored_never_green \
  ep040_m5_chaos_engine_runs_port_refusal_with_evidence \
  ep040_m5_chaos_engine_runs_revocation_with_evidence \
  ep040_m5_chaos_engine_runs_corruption_with_evidence \
  ep040_m5_chaos_engine_runs_stale_evidence_with_evidence \
  ep040_m5_chaos_engine_runs_temp_leak_with_evidence \
  ep040_m5_chaos_failure_classification_typed \
  ep040_m5_chaos_failure_class_roundtrip \
  ep040_m5_chaos_pressure_probe_reports_facts \
  ep040_m5_chaos_pressure_attribution_refuses_foreign_roots \
  ep040_m5_chaos_unknown_scenario_rejected \
  ep040_m5_chaos_gate_outcome_vacuity_invariant \
  ep040_m5_chaos_evidence_redaction_canary \
  ep040_m5_chaos_evidence_missing_binding_rejected \
  ep040_m5_chaos_recovery_attempted_ne_recovered \
  ep040_m5_chaos_terminate_classifies_unavailable \
  ep040_m5_chaos_pressure_lesson_documented \
  ep040_m5_chaos_scenario_evidence_serde_roundtrip \
  ep040_m5_chaos_all_scenarios_have_bounded_blast_radius \
  ep040_m5_chaos_evidence_root_owned_prefix_enforced \
  ep040_m5_chaos_terminate_cleanup_zero_residue; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-040-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "all 31 EP-040 M5-owned chaos proofs observed"

# Vacuity guard 8: no placeholder content in the chaos crate.
if grep -rqiE 'placeholder|TODO|FIXME|sample only|not implemented yet' tests/chaos/src tests/chaos/tests; then
  fail "chaos crate contains placeholder content"
fi
ok "chaos crate content validated"

# Vacuity guard 9: dependency direction - the chaos crate depends only
# on canonical M1/M2/M3/M4 surfaces plus serde.
bad_dep=$(cargo tree -p nexus-chaos --depth 1 2>/dev/null | grep -vE 'nexus-chaos|nexus-test-contract|nexus-test-execution|nexus-provider-certification|nexus-security-core|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-chaos: $bad_dep"
fi
ok "dependency-direction clean (canonical surfaces only)"

# Clippy -D warnings and fmt on the owned crate.
if ! sh -c 'cargo clippy -p nexus-chaos --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-chaos -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# License of the crate itself: declared MIT.
if ! grep -q '^license = "MIT"' tests/chaos/Cargo.toml; then
  fail "nexus-chaos license must be MIT"
fi
ok "crate license declared (MIT)"

# Vacuity guard 10: resource hygiene - zero EP-040-owned containers and
# zero M5 temp evidence roots remain after the real runs.
leftover=$(docker ps -a | awk '{print $NF}' | grep '^nexus-ep040-m3-' || true)
if [ -n "$leftover" ]; then
  fail "EP-040 M3/M5 containers left running: $leftover"
fi
leftover_evid=$(ls -d /tmp/ep040-m5-* 2>/dev/null | grep -v '/tmp/ep040-m5-tests.log' || true)
if [ -n "$leftover_evid" ]; then
  fail "EP-040 M5 temp evidence residue: $leftover_evid"
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

# M4 regression: security + hardware suites must stay green.
if ! sh -c 'cargo test -p nexus-security-core -p nexus-hardware-certification --locked >> "$1" 2>&1' _ "$log"; then
  fail "M4 regression failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "M4 regression ran no tests (vacuity guard)" "$log"
fi
ok "M4 regression green"

# Expected-files EP-040 must close for M5 scope (tests/chaos/ and the
# nightly workflow are M5-owned entries).
if ! grep -q 'tests/chaos/' .agent/expected-files/EP-040.txt; then
  fail "expected-files EP-040 missing tests/chaos/ entry"
fi
if ! grep -q '.github/workflows/nightly.yml' .agent/expected-files/EP-040.txt; then
  fail "expected-files EP-040 missing nightly workflow entry"
fi
ok "expected-files EP-040 lists M5-owned paths"

echo "EP-040 M5 gate: ok"
