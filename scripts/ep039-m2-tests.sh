#!/usr/bin/env sh
# EP-039 M2 gate: run the nexus-supply-chain-policy deterministic engine
# suite through the REAL cargo machinery with vacuity guards (EP-001
# gate-masking class) plus the M1 regression.
#
# M2 owns supply-chain/ (root-level policy engine crate: license
# classification behavior, component boundary evaluation, SBOM
# verification, deterministic provenance, waiver validation, advisory
# evaluation, redacted evidence) and Cargo.toml/Cargo.lock workspace
# membership.
#
# Vacuous green is impossible: `cargo test -t <filter>` exits 0 on a
# zero-match filter, so a green M2 must observe real non-zero passing
# counts, EP-039-owned test names, and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

# Ensure cargo is available to `sh -c` subshells (the interactive alias
# is not inherited). ~/.cargo/env appends cargo's bin dir to PATH.
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep039-m2-tests.log"
: > "$log"

fail() {
  echo "EP-039 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-039 M2 gate: $1"; }

# Vacuity guard 0: the crate must exist with its owned sources.
if [ ! -f supply-chain/Cargo.toml ]; then
  fail "supply-chain/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/license.rs \
  src/boundary.rs \
  src/sbom.rs \
  src/provenance.rs \
  src/waiver.rs \
  src/advisory.rs \
  src/evidence.rs \
  tests/ep039_m2_policy.rs; do
  if [ ! -f "supply-chain/$f" ]; then
    fail "supply-chain/$f missing"
  fi
done
ok "nexus-supply-chain-policy crate and M2-owned sources present"

# Vacuity guard 0b: the workspace declares the crate member.
if ! grep -q '"supply-chain"' Cargo.toml; then
  fail "workspace Cargo.toml missing supply-chain member"
fi
ok "workspace member declared"

# Real test run through cargo, captured to the log for raw sentinels.
if ! sh -c 'cargo test -p nexus-supply-chain-policy --locked >> "$1" 2>&1' _ "$log"; then
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

# Vacuity guard 4 (anti-masking): EP-039-owned M2 behavior tests observed.
# One sentinel per behavior family + cross-cutting invariants.
for sentinel in \
  ep039_unit_m2_license_green_permitted_only_under_exact_policy \
  ep039_unit_m2_license_green_allowlist_entry_not_approval \
  ep039_unit_m2_license_review_requires_review_state \
  ep039_unit_m2_license_external_never_auto_approved \
  ep039_unit_m2_license_prohibited_fails_closed \
  ep039_unit_m2_license_unknown_fails_closed \
  ep039_unit_m2_license_missing_fails_closed \
  ep039_unit_m2_license_fuzzy_string_never_bypasses_policy \
  ep039_unit_m2_boundary_sidecar_requires_process_separation \
  ep039_unit_m2_boundary_sidecar_requires_declared_boundary \
  ep039_unit_m2_boundary_sidecar_requires_api_contract \
  ep039_unit_m2_boundary_sidecar_requires_source_offer \
  ep039_unit_m2_boundary_external_must_be_provider_integration \
  ep039_unit_m2_boundary_transitive_never_out_of_scope \
  ep039_unit_m2_sbom_empty_fails \
  ep039_unit_m2_sbom_stale_fails \
  ep039_unit_m2_sbom_generated_not_verified_fails \
  ep039_unit_m2_sbom_missing_component_fails \
  ep039_unit_m2_sbom_duplicate_ambiguity_fails \
  ep039_unit_m2_sbom_package_name_collision_fails \
  ep039_unit_m2_sbom_image_tag_without_digest_fails \
  ep039_unit_m2_provenance_unsigned_not_trusted \
  ep039_unit_m2_provenance_verified_binds_deterministically \
  ep039_unit_m2_provenance_different_digest_different_binding \
  ep039_unit_m2_waiver_absent_denied \
  ep039_unit_m2_waiver_expired_denied \
  ep039_unit_m2_waiver_wrong_package_denied \
  ep039_unit_m2_waiver_wrong_version_denied \
  ep039_unit_m2_waiver_wrong_scope_denied \
  ep039_unit_m2_waiver_wildcard_denied \
  ep039_unit_m2_waiver_valid_permits_exact_bounded_decision \
  ep039_unit_m2_advisory_source_not_queried_not_safe \
  ep039_unit_m2_advisory_critical_without_mitigation_blocks \
  ep039_unit_m2_advisory_critical_with_expired_mitigation_blocks \
  ep039_unit_m2_advisory_critical_with_bounded_mitigation_passes \
  ep039_unit_m2_advisory_fixed_version_not_affected \
  ep039_unit_m2_redaction_never_leaks_sk_token \
  ep039_unit_m2_redaction_never_leaks_aws_key \
  ep039_unit_m2_redaction_never_leaks_credential_url \
  ep039_unit_m2_evidence_document_redacts_all_fields \
  ep039_unit_m2_policy_engine_idempotent_and_deterministic; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-039-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-039-owned M2 behavior tests observed (all 7 policy families)"

# Vacuity guard 5: dependency direction - the policy engine must depend
# only on nexus-domain, nexus-supply-chain, serde, serde_json. No OCI
# registry, package manager, scanner, signature provider, or vendor SDK.
bad_dep=$(cargo tree -p nexus-supply-chain-policy --depth 1 2>/dev/null | grep -vE 'nexus-supply-chain-policy|nexus-supply-chain|nexus-domain|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-supply-chain-policy: $bad_dep"
fi
for forbidden in cyclonedx spdx-tools syft grype cosign sigstore slsa in-toto trivy osv-scanner aquasec anchore quay docker-registry npm pypi pip cargo-registry; do
  if cargo tree -p nexus-supply-chain-policy 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider SDK dependency forbidden in M2: $forbidden"
  fi
done
ok "dependency-direction clean (nexus-supply-chain + nexus-domain + serde only)"

# Vacuity guard 6: no placeholder content in the policy engine.
if grep -rqiE 'placeholder|TODO|fake|sample only' supply-chain/src; then
  fail "policy engine contains placeholder content"
fi
ok "policy engine content validated"

# Clippy -D warnings and fmt on the owned crate.
if ! sh -c 'cargo clippy -p nexus-supply-chain-policy --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-supply-chain-policy -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# License/security of the crate itself: declared license MIT.
if ! grep -q '^license = "MIT"' supply-chain/Cargo.toml; then
  fail "nexus-supply-chain-policy license must be MIT"
fi
ok "crate license declared (MIT)"

# Vacuity guard 7 (anti-masking): redaction canary proof must have run.
if ! grep -q 'ep039_unit_m2_redaction_never_leaks_bearer_token' "$log"; then
  fail "redaction canary proof did not run (anti-masking)" "$log"
fi
ok "redaction proof observed"

# M1 regression: the M2 crate must not break the M1 contract crate.
m1log="/tmp/ep039-m2-m1-regression.log"
: > "$m1log"
if ! sh -c 'cargo test -p nexus-supply-chain --locked >> "$1" 2>&1' _ "$m1log"; then
  fail "M1 regression: cargo test -p nexus-supply-chain failed" "$m1log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$m1log"; then
  fail "M1 regression: no tests ran (vacuity guard)" "$m1log"
fi
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$m1log"; then
  fail "M1 regression: observed failed tests" "$m1log"
fi
ok "M1 regression green"

echo "EP-039 M2 gate: ok"
