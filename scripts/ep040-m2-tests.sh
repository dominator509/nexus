#!/usr/bin/env sh
# EP-040 M2 gate: run the nexus-test-execution + nexus-accessibility-audit
# deterministic behavior suites through the REAL cargo machinery with
# vacuity guards (EP-001 gate-masking class).
#
# M2 owns tests/integration/ (deterministic test execution core:
# real subprocess runner, cargo-output parser, GateResult aggregation,
# flake policy, consecutive-verify policy, evidence store) and
# tests/accessibility/audit-core/ (deterministic WCAG audit verdict
# engine) plus Cargo.toml/Cargo.lock workspace membership.
#
# Vacuous green is impossible: `cargo test -t <filter>` exits 0 on a
# zero-match filter, so a green M2 must observe real non-zero passing
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

log="/tmp/ep040-m2-tests.log"
: > "$log"

fail() {
  echo "EP-040 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-040 M2 gate: $1"; }

# Vacuity guard 0: the owned crates must exist with their owned sources.
if [ ! -f tests/integration/Cargo.toml ]; then
  fail "tests/integration/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/runner.rs \
  src/policy.rs \
  src/evidence.rs \
  tests/ep040_m2_execution.rs; do
  if [ ! -f "tests/integration/$f" ]; then
    fail "tests/integration/$f missing"
  fi
done
if [ ! -f tests/accessibility/audit-core/Cargo.toml ]; then
  fail "tests/accessibility/audit-core/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  tests/ep040_m2_accessibility.rs; do
  if [ ! -f "tests/accessibility/audit-core/$f" ]; then
    fail "tests/accessibility/audit-core/$f missing"
  fi
done
ok "nexus-test-execution + nexus-accessibility-audit crates and M2-owned sources present"

# Vacuity guard 0b: the workspace declares both crate members.
if ! grep -q 'tests/integration' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/integration member"
fi
if ! grep -q 'tests/accessibility/audit-core' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/accessibility/audit-core member"
fi
ok "workspace members declared"

# Real test run through cargo, captured to the log for raw sentinels
# (rtk-tee compresses interactive cargo output).
if ! sh -c 'cargo test -p nexus-test-execution -p nexus-accessibility-audit --locked >> "$1" 2>&1' _ "$log"; then
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

# Vacuity guard 4 (anti-masking): EP-040-owned behavior tests observed.
# One sentinel per deterministic behavior family.
for sentinel in \
  ep040_unit_parser_recognizes_passed_line \
  ep040_unit_parser_recognizes_failed_line \
  ep040_unit_parser_recognizes_ignored_and_skipped_lines \
  ep040_unit_parser_recognizes_summary_line \
  ep040_unit_parser_output_without_summary_fails_closed \
  ep040_unit_parse_output_zero_tests_collected_not_green \
  ep040_unit_parse_output_skipped_required_test_not_green \
  ep040_unit_parse_output_ignored_required_test_not_green \
  ep040_unit_parse_output_failed_test_not_green \
  ep040_unit_parse_output_all_passed_evidence_bound_green \
  ep040_unit_parse_output_evidence_bound_required_for_green \
  ep040_unit_parse_output_evidence_ran_but_not_behavior_verified \
  ep040_unit_run_tests_executes_real_command \
  ep040_unit_run_tests_real_failing_command_not_green \
  ep040_unit_run_tests_missing_program_fails_closed \
  ep040_unit_matrix_validator_rejects_vacuous \
  ep040_unit_matrix_validator_accepts_required_tests \
  ep040_unit_flake_policy_classifies_known_classes \
  ep040_unit_flake_policy_rejects_empty_test_id \
  ep040_unit_flake_policy_port_object_safe \
  ep040_unit_consecutive_verify_requires_three_green \
  ep040_unit_consecutive_verify_resets_on_failure \
  ep040_unit_consecutive_verify_fix_requires_root_cause \
  ep040_unit_evidence_store_requires_run_context \
  ep040_unit_evidence_store_roundtrip_redacted \
  ep040_unit_evidence_store_port_object_safe \
  ep040_unit_accessibility_wcag_level_deny_unknown \
  ep040_unit_accessibility_parse_violation_canonical_shape \
  ep040_unit_accessibility_parse_violation_fails_closed_on_bad_shape \
  ep040_unit_accessibility_evaluate_clean_passes \
  ep040_unit_accessibility_evaluate_a_blocks_all_findings \
  ep040_unit_accessibility_evaluate_aa_blocks_a_and_aa_not_aaa \
  ep040_unit_accessibility_evaluate_aaa_blocks_everything \
  ep040_unit_accessibility_audit_port_implementable \
  ep040_unit_accessibility_dependency_direction \
  ep040_unit_dependency_direction_execution_core; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-040-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-040-owned behavior tests observed (execution core + accessibility)"

# Vacuity guard 5: dependency direction - both crates must depend only on
# nexus-test-contract/nexus-domain + serde + serde_json. No test runner,
# chaos injector, certification harness, or performance framework in M2.
bad_dep=$(cargo tree -p nexus-test-execution --depth 1 2>/dev/null | grep -vE 'nexus-test-execution|nexus-test-contract|nexus-domain|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-test-execution: $bad_dep"
fi
bad_dep=$(cargo tree -p nexus-accessibility-audit --depth 1 2>/dev/null | grep -vE 'nexus-accessibility-audit|nexus-test-contract|nexus-domain|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-accessibility-audit: $bad_dep"
fi
for forbidden in tokio axum actix rocket reqwest hyper tonic tower serde_yaml proptest quickcheck criterion iai testcontainers docker-compose k6 locust gatling selenium playwright puppet cypress junit testng pytest axe-core; do
  if cargo tree -p nexus-test-execution -p nexus-accessibility-audit 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider SDK/test framework dependency forbidden in M2: $forbidden"
  fi
done
ok "dependency-direction clean (nexus-domain + serde + serde_json only)"

# Vacuity guard 6: no placeholder content in the behavior crates.
if grep -rqiE 'placeholder|TODO|fake|sample only' tests/integration/src tests/accessibility/audit-core/src; then
  fail "behavior crate contains placeholder content"
fi
ok "behavior crate content validated"

# Clippy -D warnings and fmt on the owned crates.
if ! sh -c 'cargo clippy -p nexus-test-execution -p nexus-accessibility-audit --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-test-execution -p nexus-accessibility-audit -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# License/security of the crates themselves: declared MIT.
if ! grep -q '^license = "MIT"' tests/integration/Cargo.toml; then
  fail "nexus-test-execution license must be MIT"
fi
if ! grep -q '^license = "MIT"' tests/accessibility/audit-core/Cargo.toml; then
  fail "nexus-accessibility-audit license must be MIT"
fi
ok "crate licenses declared (MIT)"

# M1 regression: the contract + performance suites must stay green.
if ! sh -c 'cargo test -p nexus-test-contract -p nexus-test-performance --locked >> "$1" 2>&1' _ "$log"; then
  fail "M1 regression failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "M1 regression ran no tests (vacuity guard)" "$log"
fi
ok "M1 regression green"

echo "EP-040 M2 gate: ok"
