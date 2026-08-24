#!/usr/bin/env sh
# EP-039 M4 gate: forced failures, abuse cases, and observability for
# the supply-chain / SBOM surface (SPEC-019; EP-039 M4 fence
# `scripts/sbom/` + the authorized `policies/licenses/` crate).
#
# M4 owns:
# - scripts/sbom/ (real SBOM evidence generator, verifier, redacted
#   observability, forced-failure runner, README)
# - policies/licenses/examples/sbom_generate.rs (real generator
#   adapter over the certified M3 transport)
# - policies/licenses/tests/ep039_failure_sbom.rs (26 ep039_failure_*
#   proofs using REAL failure mechanisms - isolated temp fixtures,
#   real registry cache, real policy files, no mocked component)
#
# Vacuous green is impossible: the gate requires real cargo test runs
# with non-zero pass counts, every ep039_failure_* sentinel observed,
# real script execution (generate/verify/observability/forced-failures),
# clippy, fmt, redaction proof, and M1+M2+M3 regressions.
set -eu
export CI=true
export CARGO_TERM_COLOR=never
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep039-m4-tests.log"
: > "$log"

fail() {
  echo "EP-039 M4 gate: FAIL - $1" >&2
  tail -60 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-039 M4 gate: $1"; }

# Vacuity guard 0: M4-owned material presence.
for f in \
  scripts/sbom/generate.sh \
  scripts/sbom/verify.sh \
  scripts/sbom/observability.sh \
  scripts/sbom/forced-failures.sh \
  scripts/sbom/README.md \
  policies/licenses/examples/sbom_generate.rs \
  policies/licenses/tests/ep039_failure_sbom.rs; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "M4-owned scripts, example, and failure suite present"

# Vacuity guard 0b: scripts are executable.
for f in scripts/sbom/generate.sh scripts/sbom/verify.sh scripts/sbom/observability.sh scripts/sbom/forced-failures.sh; do
  if [ ! -x "$f" ]; then
    fail "$f not executable"
  fi
done
ok "scripts/sbom/ executables"

# Vacuity guard 0c: workspace still declares the transport crate.
if ! grep -q '"policies/licenses"' Cargo.toml; then
  fail "workspace Cargo.toml missing policies/licenses member"
fi
ok "workspace member declared"

# Vacuity guard 1: no placeholder content in M4-owned sources.
if grep -rqiE 'placeholder|TODO|fake|sample only' scripts/sbom policies/licenses/examples policies/licenses/tests/ep039_failure_sbom.rs; then
  fail "M4-owned sources contain placeholder content"
fi
ok "M4-owned content validated"

# Vacuity guard 2 (anti-masking): no secret-shaped literals in tracked
# M4-owned sources (canaries are runtime-constructed).
if grep -rniE 'sk-[A-Za-z0-9]|ghp_[A-Za-z0-9]|AKIA[0-9A-Z]|Bearer [A-Za-z0-9]' \
  scripts/sbom policies/licenses/examples policies/licenses/tests/ep039_failure_sbom.rs; then
  fail "secret-shaped literal in tracked M4 sources"
fi
ok "no secret literals in tracked M4 sources"

# Real test run through cargo, captured to the log for raw sentinels.
if ! sh -c 'cargo test -p nexus-supply-chain-policy-io --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test failed" "$log"
fi

# Vacuity guard 3: a real non-zero pass was observed.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 4: zero failures observed.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 5: zero ignored tests.
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 6 (anti-masking): every required forced-failure case
# actually ran. One sentinel per owned abuse case.
for sentinel in \
  ep039_failure_missing_lockfile_fails_closed \
  ep039_failure_malformed_lockfile_fails_closed \
  ep039_failure_empty_lockfile_refused \
  ep039_failure_generate_inventory_missing_lockfile_fails_closed \
  ep039_failure_unknown_license_fails_closed \
  ep039_failure_missing_license_field_fails_closed_on_real_workspace \
  ep039_failure_fuzzy_license_alias_fails_closed \
  ep039_failure_prohibited_license_fails_closed \
  ep039_failure_transitive_dependency_with_denied_license_fails_closed \
  ep039_failure_duplicate_package_ambiguity_fails \
  ep039_failure_same_package_version_different_source_fails \
  ep039_failure_image_tag_without_digest_fails \
  ep039_failure_stale_sbom_evidence_fails \
  ep039_failure_empty_sbom_evidence_fails \
  ep039_failure_tampered_sbom_binding_fails \
  ep039_failure_mismatched_run_id_fails \
  ep039_failure_waiver_wrong_scope_fails \
  ep039_failure_waiver_expired_fails \
  ep039_failure_waiver_revoked_fails \
  ep039_failure_advisory_source_not_queried_fails \
  ep039_failure_advisory_critical_unmitigated_blocks \
  ep039_failure_secret_canary_redacted_in_evidence \
  ep039_failure_observability_evidence_bound_to_real_inventory \
  ep039_failure_real_inventory_denied_finding_preserved \
  ep039_failure_license_engine_denied_without_approval_fails \
  ep039_failure_unverified_component_never_releasable; do
  if ! grep -q "$sentinel" "$log"; then
    fail "forced-failure test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "all 26 ep039_failure_* forced-failure proofs observed"

# Vacuity guard 7: redaction canary proof must have run.
if ! grep -q 'ep039_failure_secret_canary_redacted_in_evidence' "$log"; then
  fail "redaction canary proof did not run (anti-masking)" "$log"
fi
ok "redaction proof observed"

# Dependency direction: the transport crate may depend only on the
# certified surface. The example/failure tests add no new dependencies.
bad_dep=$(cargo tree -p nexus-supply-chain-policy-io --depth 1 2>/dev/null | grep -vE 'nexus-supply-chain-policy-io|nexus-supply-chain-policy|nexus-supply-chain|nexus-domain|serde|serde_json|toml' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-supply-chain-policy-io: $bad_dep"
fi
for forbidden in cyclonedx spdx-tools syft grype cosign sigstore slsa in-toto trivy osv-scanner aquasec anchore quay docker-registry npm pypi pip cargo-registry; do
  if cargo tree -p nexus-supply-chain-policy-io 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider SDK dependency forbidden in M4: $forbidden"
  fi
done
ok "dependency-direction clean"

# Clippy -D warnings (all targets, includes the example) and fmt.
if ! sh -c 'cargo clippy -p nexus-supply-chain-policy-io --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean (incl. example)"

if ! sh -c 'cargo fmt -p nexus-supply-chain-policy-io -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# Real script execution: the gate must exercise scripts/sbom/ itself.
WORK=$(mktemp -d /tmp/ep039-m4-gate.XXXXXX)
trap 'rm -rf "$WORK"' EXIT INT TERM

if ! sh scripts/sbom/generate.sh "$WORK/evidence" >"$WORK/generate.log" 2>&1; then
  fail "scripts/sbom/generate.sh failed" "$WORK/generate.log"
fi
ok "generate.sh produced bound evidence"

if ! sh scripts/sbom/verify.sh "$WORK/evidence" >"$WORK/verify.log" 2>&1; then
  fail "scripts/sbom/verify.sh rejected fresh evidence" "$WORK/verify.log"
fi
ok "verify.sh verified evidence against current repository state"

if ! sh scripts/sbom/observability.sh "$WORK/evidence" >"$WORK/observability.log" 2>&1; then
  fail "scripts/sbom/observability.sh failed" "$WORK/observability.log"
fi
for field in run_id git_commit lockfile_fingerprint policy_fingerprint package_count \
  resolved_count green_count denied_count unknown_count missing_license_count \
  policy_verdict verification_state provenance_state advisory_source_status redaction; do
  if ! grep -q "^$field:" "$WORK/observability.log"; then
    fail "observability missing field $field" "$WORK/observability.log"
  fi
done
ok "observability.sh emitted all required redacted fields"

if ! sh scripts/sbom/forced-failures.sh >"$WORK/forced.log" 2>&1; then
  fail "scripts/sbom/forced-failures.sh failed" "$WORK/forced.log"
fi
for proof in \
  "missing Cargo.lock fails closed" \
  "malformed Cargo.lock fails closed" \
  "fresh evidence verifies against current repository state" \
  "tampered evidence rejected (TAMPERED_EVIDENCE)" \
  "stale evidence rejected (STALE_EVIDENCE)" \
  "mismatched run_id/git_commit rejected (MISMATCHED_RUN_ID)" \
  "empty evidence rejected (EMPTY_EVIDENCE)" \
  "generated evidence redacted (no secret-shaped content)"; do
  if ! grep -qF "$proof" "$WORK/forced.log"; then
    fail "forced-failures proof missing: $proof" "$WORK/forced.log"
  fi
done
ok "forced-failures.sh proved every shell-level abuse case"

# The evidence must reflect the honest non-green verdict while the
# denied finding stands (SBOM GENERATED != POLICY PASSED).
if ! grep -q '"policy_verdict":"NON_GREEN"' "$WORK/evidence/evidence.json"; then
  fail "evidence does not carry the honest NON_GREEN verdict"
fi
if ! grep -q '"policy_passed":false' "$WORK/evidence/evidence.json"; then
  fail "evidence claims policy passed (must stay false)"
fi
ok "evidence honest verdict preserved (16-denied finding not erased)"

# M1 + M2 + M3 regressions: the failure suite must not break the
# contract crate, the deterministic engine, or the real transport.
for crate in nexus-supply-chain nexus-supply-chain-policy nexus-supply-chain-policy-io; do
  mlog="/tmp/ep039-m4-regression-$crate.log"
  : > "$mlog"
  if ! sh -c 'cargo test -p "$1" --locked >> "$2" 2>&1' _ "$crate" "$mlog"; then
    fail "regression: cargo test -p $crate failed" "$mlog"
  fi
  if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$mlog"; then
    fail "regression: no tests ran for $crate (vacuity guard)" "$mlog"
  fi
  if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$mlog"; then
    fail "regression: observed failed tests for $crate" "$mlog"
  fi
done
ok "M1 + M2 + M3 regression green"

echo "EP-039 M4 gate: ok"
