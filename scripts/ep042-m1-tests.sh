#!/usr/bin/env sh
# EP-042 M1 gate: run the nexus-release contract suite through the REAL
# cargo machinery with vacuity guards (EP-001 gate-masking class).
#
# M1 owns crates/nexus-release/ (provider-neutral deployment/release/
# update/rollback contract crate) and .github/workflows/release.yml
# (release CI surface). The authoritative gate is the crate suite plus
# clippy/fmt, deny-unknown vocabulary proofs, fail-closed invariant
# proofs for every public interface, dependency-direction proof, and a
# no-placeholder scan of the workflow.
#
# Vacuous green is impossible: `cargo test -t <filter>` exits 0 on a
# zero-match filter, so a green M1 must observe real non-zero passing
# counts, EP-042-owned test names, and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

# Ensure cargo is available to `sh -c` subshells (the interactive alias
# is not inherited). ~/.cargo/env appends cargo's bin dir to PATH.
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep042-m1-tests.log"
: > "$log"

fail() {
  echo "EP-042 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-042 M1 gate: $1"; }

# Vacuity guard 0: the crate must exist with its owned sources.
if [ ! -f crates/nexus-release/Cargo.toml ]; then
  fail "crates/nexus-release/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/error.rs \
  src/vocabulary.rs \
  src/model.rs; do
  if [ ! -f "crates/nexus-release/$f" ]; then
    fail "crates/nexus-release/$f missing"
  fi
done
if [ ! -f .github/workflows/release.yml ]; then
  fail ".github/workflows/release.yml missing"
fi
if [ ! -f references/ADR-028-deployment-release-update-vocabulary.md ]; then
  fail "references/ADR-028-deployment-release-update-vocabulary.md missing"
fi
ok "nexus-release crate and release.yml present"

# Workspace membership.
grep -q '"crates/nexus-release"' Cargo.toml || fail "nexus-release not registered in workspace members"

# Real test run through cargo, captured to the log for raw sentinels
# (rtk-tee compresses interactive cargo output).
if ! sh -c 'cargo test -p nexus-release --locked >> "$1" 2>&1' _ "$log"; then
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

# Vacuity guard 4 (anti-masking): EP-042-owned contract tests observed.
# One sentinel per public interface plus the cross-cutting invariants.
for sentinel in \
  ep042_unit_digest_accepts_real_sha256_hex \
  ep042_unit_signature_present_not_valid \
  ep042_unit_component_construction_and_signature_state \
  ep042_unit_matrix_accepts_components_in_range \
  ep042_unit_matrix_rejects_unknown_component \
  ep042_unit_matrix_supports_all_profiles \
  ep042_unit_manifest_construction_roundtrip \
  ep042_unit_manifest_exists_not_verified \
  ep042_unit_plan_requires_backup_first_step \
  ep042_unit_plan_exists_not_executed \
  ep042_unit_canary_observing_never_promoted \
  ep042_unit_canary_ready_requires_evidence \
  ep042_unit_rollback_receipt_requires_backup_ref \
  ep042_unit_bundle_requires_image_model_license_sbom \
  ep042_unit_bundle_exists_not_verified \
  ep042_unit_promotion_requires_human_approval \
  ep042_unit_promotion_never_deploys \
  ep042_unit_promotion_gate_never_deploys \
  ep042_unit_vocabulary_update_step_kind_has_no_promote \
  ep042_unit_vocabulary_canary_never_promotes \
  ep042_unit_vocabulary_serde_rejects_unknown_wire_value; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-042-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-042-owned contract tests observed (all 8 interfaces)"

# Vacuity guard 5: dependency direction - the contract crate must depend
# only on nexus-domain, serde, serde_json, and sha2. No provider SDKs or
# installer/update engines in M1.
bad_dep=$(cargo tree -p nexus-release --depth 1 2>/dev/null | grep -vE 'nexus-release|nexus-domain|serde|serde_json|sha2' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-release: $bad_dep"
fi
for forbidden in prometheus grafana opentelemetry datadog honeycomb sentry loki tempo jaeger aws-sdk azure google-cloud minio seaweedfs docker k8s kubernetes tonic axum tokio; do
  if cargo tree -p nexus-release 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider/engine dependency forbidden in M1: $forbidden"
  fi
done
ok "dependency-direction clean (nexus-domain + serde + serde_json + sha2 only)"

# Vacuity guard 6: no-placeholder scan on the owned surface.
for path in crates/nexus-release/src .github/workflows/release.yml; do
  if grep -rniE 'placeholder|TODO|FIXME|XXX|not implemented|unimplemented!' "$path" 2>/dev/null; then
    fail "placeholder content in $path"
  fi
done
ok "no-placeholder scan clean"

# Vacuity guard 7: the workflow is a real CI surface and never deploys.
if ! grep -q 'ep042-m1-tests.sh' .github/workflows/release.yml; then
  fail "release.yml does not run the M1 gate"
fi
# Scan actual workflow steps (run:/uses: lines), not prose comments.
if grep -qiE '^\s*(run|uses):.*(deploy|apply -f|kubectl|ssh |helm |terraform apply)' .github/workflows/release.yml; then
  fail "release.yml must not contain deploy steps (production deploy not authorized)"
fi
ok "release.yml validates and contains no deploy step"

# Clippy -D warnings and fmt on the owned crate.
if ! sh -c 'cargo clippy -p nexus-release --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-release -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

echo "EP-042 M1 gate: ok"
