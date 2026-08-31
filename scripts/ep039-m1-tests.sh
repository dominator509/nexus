#!/usr/bin/env sh
# EP-039 M1 gate: run the nexus-supply-chain contract suite through the
# REAL cargo machinery with vacuity guards (EP-001 gate-masking class).
#
# M1 owns crates/nexus-supply-chain/ (provider-neutral supply-chain
# contract crate: LicenseClassifier, ComponentBoundary, SbomGenerator,
# ArtifactSigner, ProvenanceAttestation, AdvisoryMonitor,
# DependencyWaiver) and Cargo.toml/Cargo.lock workspace membership.
#
# Vacuous green is impossible: `cargo test -t <filter>` exits 0 on a
# zero-match filter, so a green M1 must observe real non-zero passing
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

log="/tmp/ep039-m1-tests.log"
: > "$log"

fail() {
  echo "EP-039 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-039 M1 gate: $1"; }

# Vacuity guard 0: the crate must exist with its owned sources.
if [ ! -f crates/nexus-supply-chain/Cargo.toml ]; then
  fail "crates/nexus-supply-chain/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/error.rs \
  src/vocabulary.rs \
  src/model.rs \
  src/port.rs \
  tests/ep039_m1_contract.rs; do
  if [ ! -f "crates/nexus-supply-chain/$f" ]; then
    fail "crates/nexus-supply-chain/$f missing"
  fi
done
ok "nexus-supply-chain crate and M1-owned sources present"

# Vacuity guard 0b: the workspace declares the crate member.
if ! grep -q 'crates/nexus-supply-chain' Cargo.toml; then
  fail "workspace Cargo.toml missing nexus-supply-chain member"
fi
ok "workspace member declared"

# Real test run through cargo, captured to the log for raw sentinels
# (rtk-tee compresses interactive cargo output).
if ! sh -c 'cargo test -p nexus-supply-chain --locked >> "$1" 2>&1' _ "$log"; then
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

# Vacuity guard 4 (anti-masking): EP-039-owned contract tests observed.
# One sentinel per public interface + the cross-cutting invariants.
for sentinel in \
  ep039_unit_vocabulary_deny_unknown_license_class \
  ep039_unit_vocabulary_serde_rejects_unknown_wire_value \
  ep039_unit_license_classify_green_permissive \
  ep039_unit_license_unknown_fails_closed \
  ep039_unit_license_missing_fails_closed \
  ep039_unit_license_present_not_verified \
  ep039_unit_dependency_exists_not_approved \
  ep039_unit_allowlist_entry_not_legal_approval_for_all_uses \
  ep039_unit_transitive_dependency_never_out_of_scope \
  ep039_unit_package_name_match_not_same_artifact \
  ep039_unit_image_tag_not_digest \
  ep039_unit_sbom_generated_not_verified \
  ep039_unit_sbom_build_passed_not_complete \
  ep039_unit_sbom_lockfile_exists_not_accounted \
  ep039_unit_sbom_stale_fails_closed \
  ep039_unit_sbom_transitive_included_in_scope \
  ep039_unit_provenance_unsigned_not_trusted \
  ep039_unit_waiver_expired_fails_closed \
  ep039_unit_component_boundary_sidecar_source_offer \
  ep039_unit_advisory_critical_without_mitigation_blocks \
  ep039_unit_error_codes_are_canonical \
  ep039_unit_error_serializes_without_secrets \
  ep039_unit_error_messages_never_contain_secret_shaped_values \
  ep039_unit_component_fail_closed_defaults_never_releasable \
  ep039_unit_component_requires_full_review_ladder \
  ep039_unit_component_serializes_roundtrip \
  ep039_unit_sbom_serializes_without_secrets \
  ep039_unit_port_traits_implementable \
  ep039_unit_dependency_direction; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-039-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-039-owned contract tests observed (all 7 interfaces)"

# Vacuity guard 5: dependency direction - the contract crate must depend
# only on nexus-domain, serde, serde_json, and sha2. No OCI registry,
# package manager, scanner, signature provider, or vendor SDK in M1.
bad_dep=$(cargo tree -p nexus-supply-chain --depth 1 2>/dev/null | grep -vE 'nexus-supply-chain|nexus-domain|serde|serde_json|sha2|ring|build-dependencies' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-supply-chain: $bad_dep"
fi
for forbidden in cyclonedx spdx-tools syft grype cosign sigstore slsa in-toto trivy osv-scanner aquasec anchore quay docker-registry npm pypi pip cargo-registry; do
  if cargo tree -p nexus-supply-chain 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider SDK dependency forbidden in M1: $forbidden"
  fi
done
ok "dependency-direction clean (nexus-domain + serde + sha2 + ring only)"

# Vacuity guard 6: no placeholder content in the contract crate.
if grep -rqiE 'placeholder|TODO|fake|sample only' crates/nexus-supply-chain/src; then
  fail "contract crate contains placeholder content"
fi
ok "contract crate content validated"

# Clippy -D warnings and fmt on the owned crate.
if ! sh -c 'cargo clippy -p nexus-supply-chain --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-supply-chain -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# License/security of the crate itself: the declared license is MIT and
# no dependency outside the allowed surface was introduced.
if ! grep -q '^license = "MIT"' crates/nexus-supply-chain/Cargo.toml; then
  fail "nexus-supply-chain license must be MIT"
fi
ok "crate license declared (MIT)"

echo "EP-039 M1 gate: ok"
