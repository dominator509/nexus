#!/usr/bin/env sh
# EP-037 M1 gate: run the nexus-artifacts contract suite through the REAL
# cargo machinery with vacuity guards.
#
# The M1 changed-file fence is crates/nexus-artifacts/ (artifact storage
# contract crate) + infra/storage/ (storage topology root), so the
# authoritative gate is the crate suite (cargo test -p nexus-artifacts)
# plus dependency-direction proof and clippy -D warnings. Vacuity guards
# are required: `cargo test -t <filter>` exits 0 on a zero-match filter
# (EP-001 gate-masking class), so a green M1 must observe real non-zero
# passing counts, EP-037-owned test names, and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep037-m1-tests.log"
: > "$log"

fail() {
  echo "EP-037 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-037 M1 gate: $1"; }

# Vacuity guard 0: the crate must exist with its owned sources.
if [ ! -f crates/nexus-artifacts/Cargo.toml ]; then
  fail "crates/nexus-artifacts/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/error.rs \
  src/vocabulary.rs \
  src/model.rs \
  src/port.rs \
  tests/ep037_m1_contract.rs; do
  if [ ! -f "crates/nexus-artifacts/$f" ]; then
    fail "crates/nexus-artifacts/$f missing"
  fi
done
if [ ! -f infra/storage/README.md ]; then
  fail "infra/storage/README.md missing"
fi
ok "nexus-artifacts crate and infra/storage root present"

# Real test run through cargo, captured to the log for raw sentinels
# (rtk-tee compresses interactive cargo output).
if ! sh -c 'cargo test -p nexus-artifacts --locked >> "$1" 2>&1' _ "$log"; then
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

# Vacuity guard 4 (anti-masking): EP-037-owned contract tests observed.
for sentinel in \
  ep037_unit_vocabulary_rejects_unknown_storage_backend \
  ep037_unit_vocabulary_minio_is_compatibility_only \
  ep037_unit_artifact_hash_content_identity_is_digest_not_name \
  ep037_unit_metadata_sensitive_on_remote_backend_requires_encryption \
  ep037_unit_backup_set_advances_ladder_exactly \
  ep037_unit_restore_plan_verifies_all_hashes_before_validation \
  ep037_unit_migration_requires_verify_before_delete_approval \
  ep037_unit_dependency_direction_no_storage_sdk_or_transport; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-037-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-037-owned contract tests observed"

# Vacuity guard 5: dependency direction - the contract crate must depend
# only on nexus-domain and serde/serde_json (no storage SDK or transport).
bad_dep=$(cargo tree -p nexus-artifacts --depth 1 2>/dev/null | grep -vE 'nexus-artifacts|nexus-domain|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-artifacts: $bad_dep"
fi
ok "dependency-direction clean (nexus-domain + serde only)"

# Clippy -D warnings and fmt on the owned crate.
if ! sh -c 'cargo clippy -p nexus-artifacts --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-artifacts -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

echo "EP-037 M1 gate: ok"
