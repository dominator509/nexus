#!/usr/bin/env sh
# EP-040 M1 gate: run the nexus-test-contract + nexus-test-performance
# contract suites through the REAL cargo machinery with vacuity guards
# (EP-001 gate-masking class).
#
# M1 owns tests/contract/ (provider-neutral testing/hardening/chaos
# contract crate: TestMatrix, ChaosScenario, ProviderCertificationSuite,
# HardwareCertificationSuite, PerformanceBudget, AccessibilityAudit,
# FlakyTestPolicy) and tests/performance/ (deterministic PerformanceBudget
# evaluation root) plus Cargo.toml/Cargo.lock workspace membership.
#
# Vacuous green is impossible: `cargo test -t <filter>` exits 0 on a
# zero-match filter, so a green M1 must observe real non-zero passing
# counts, EP-040-owned test names, and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

# Ensure cargo is available to `sh -c` subshells (the interactive alias
# is not inherited). ~/.cargo/env appends cargo's bin dir to PATH.
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep040-m1-tests.log"
: > "$log"

fail() {
  echo "EP-040 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-040 M1 gate: $1"; }

# Vacuity guard 0: the owned crates must exist with their owned sources.
if [ ! -f tests/contract/Cargo.toml ]; then
  fail "tests/contract/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/error.rs \
  src/vocabulary.rs \
  src/model.rs \
  src/port.rs \
  tests/ep040_m1_contract.rs; do
  if [ ! -f "tests/contract/$f" ]; then
    fail "tests/contract/$f missing"
  fi
done
if [ ! -f tests/performance/Cargo.toml ]; then
  fail "tests/performance/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  tests/ep040_m1_performance.rs; do
  if [ ! -f "tests/performance/$f" ]; then
    fail "tests/performance/$f missing"
  fi
done
ok "nexus-test-contract + nexus-test-performance crates and M1-owned sources present"

# Vacuity guard 0b: the workspace declares both crate members.
if ! grep -q 'tests/contract' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/contract member"
fi
if ! grep -q 'tests/performance' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/performance member"
fi
ok "workspace members declared"

# Real test run through cargo, captured to the log for raw sentinels
# (rtk-tee compresses interactive cargo output).
if ! sh -c 'cargo test -p nexus-test-contract -p nexus-test-performance --locked >> "$1" 2>&1' _ "$log"; then
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

# Vacuity guard 4 (anti-masking): EP-040-owned contract tests observed.
# One sentinel per public interface + the cross-cutting invariants.
for sentinel in \
  ep040_unit_vocabulary_deny_unknown_test_layer \
  ep040_unit_vocabulary_deny_unknown_test_outcome \
  ep040_unit_vocabulary_deny_unknown_flake_classification \
  ep040_unit_vocabulary_deny_unknown_failure_injection \
  ep040_unit_vocabulary_deny_unknown_blast_radius \
  ep040_unit_vocabulary_deny_unknown_resource_kind \
  ep040_unit_vocabulary_deny_unknown_hardening_state \
  ep040_unit_vocabulary_serde_rejects_unknown_wire_value \
  ep040_unit_test_evidence_test_exists_not_test_ran \
  ep040_unit_test_evidence_test_ran_not_behavior_verified \
  ep040_unit_test_evidence_mock_passed_not_production_path_verified \
  ep040_unit_gate_result_zero_tests_collected_not_green \
  ep040_unit_gate_result_skipped_required_test_not_green \
  ep040_unit_gate_result_ignored_required_test_not_green \
  ep040_unit_test_matrix_zero_test_guard_fails_closed \
  ep040_unit_chaos_scenario_requires_bounded_blast_radius \
  ep040_unit_chaos_scenario_requires_rollback_path \
  ep040_unit_chaos_scenario_requires_cleanup_assertion \
  ep040_unit_hardening_control_defined_not_applied \
  ep040_unit_hardening_control_verify_requires_evidence \
  ep040_unit_fixture_ownership_requires_owned_prefix \
  ep040_unit_resource_residue_cleanup_attempted_not_clean \
  ep040_unit_flake_retried_green_not_fixed \
  ep040_unit_flake_fix_requires_root_cause \
  ep040_unit_regression_requirement_requires_gate \
  ep040_unit_provider_certification_requires_real_evidence \
  ep040_unit_hardware_certification_requires_model_firmware_evidence \
  ep040_unit_accessibility_audit_requires_target_and_standard \
  ep040_unit_performance_budget_build_passed_not_runtime_safe \
  ep040_unit_error_codes_are_canonical \
  ep040_unit_error_messages_never_contain_secret_shaped_values \
  ep040_unit_port_traits_implementable \
  ep040_unit_dependency_direction \
  ep040_unit_performance_budget_unobserved_fails_closed \
  ep040_unit_performance_budget_within_bound_passes \
  ep040_unit_performance_budget_exceeded_fails_policy \
  ep040_unit_performance_budget_deterministic \
  ep040_unit_performance_budget_port_object_safe; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-040-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-040-owned contract tests observed (all 7 interfaces + invariants)"

# Vacuity guard 5: dependency direction - both crates must depend only on
# nexus-test-contract/nexus-domain + serde + serde_json. No test runner,
# chaos injector, certification harness, or performance framework in M1.
bad_dep=$(cargo tree -p nexus-test-contract --depth 1 2>/dev/null | grep -vE 'nexus-test-contract|nexus-domain|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-test-contract: $bad_dep"
fi
bad_dep=$(cargo tree -p nexus-test-performance --depth 1 2>/dev/null | grep -vE 'nexus-test-performance|nexus-test-contract|nexus-domain|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-test-performance: $bad_dep"
fi
for forbidden in tokio axum actix rocket reqwest hyper tonic tower serde_yaml proptest quickcheck criterion iai testcontainers docker-compose k6 locust gatling selenium playwright puppet cypress junit testng pytest; do
  if cargo tree -p nexus-test-contract -p nexus-test-performance 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider SDK/test framework dependency forbidden in M1: $forbidden"
  fi
done
ok "dependency-direction clean (nexus-domain + serde + serde_json only)"

# Vacuity guard 6: no placeholder content in the contract crates.
if grep -rqiE 'placeholder|TODO|fake|sample only' tests/contract/src tests/performance/src; then
  fail "contract crate contains placeholder content"
fi
ok "contract crate content validated"

# Clippy -D warnings and fmt on the owned crates.
if ! sh -c 'cargo clippy -p nexus-test-contract -p nexus-test-performance --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-test-contract -p nexus-test-performance -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# License/security of the crates themselves: declared MIT and no
# dependency outside the allowed surface was introduced.
if ! grep -q '^license = "MIT"' tests/contract/Cargo.toml; then
  fail "nexus-test-contract license must be MIT"
fi
if ! grep -q '^license = "MIT"' tests/performance/Cargo.toml; then
  fail "nexus-test-performance license must be MIT"
fi
ok "crate licenses declared (MIT)"

echo "EP-040 M1 gate: ok"
