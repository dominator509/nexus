#!/usr/bin/env sh
# EP-037 M2 gate: run the real local-filesystem adapter behavior suite
# with vacuity guards.
#
# The M2 changed-file fence is connectors/storage-local/ (real
# local-filesystem ArtifactStore adapter) + tests/artifacts/ (test
# material umbrella). The authoritative gate is the adapter suite
# (cargo test -p nexus-provider-storage-local) plus clippy -D warnings
# and fmt, with M1 regression through the M1 gate.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep037-m2-tests.log"
: > "$log"

fail() {
  echo "EP-037 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-037 M2 gate: $1"; }

# Vacuity guard 0: adapter crate and test material must exist.
if [ ! -f connectors/storage-local/Cargo.toml ]; then
  fail "connectors/storage-local/Cargo.toml missing"
fi
for f in src/lib.rs tests/ep037_m2_local.rs; do
  if [ ! -f "connectors/storage-local/$f" ]; then
    fail "connectors/storage-local/$f missing"
  fi
done
if [ ! -f tests/artifacts/README.md ]; then
  fail "tests/artifacts/README.md missing"
fi
ok "storage-local adapter and tests/artifacts material present"

# Real test run through cargo.
if ! sh -c 'cargo test -p nexus-provider-storage-local --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test failed" "$log"
fi

# Vacuity guard 1: non-zero passing observed.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: zero failures observed.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 3: zero ignored tests.
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 4 (anti-masking): EP-037-owned M2 behavior tests observed.
for sentinel in \
  ep037_unit_local_put_get_roundtrip_content_addressed \
  ep037_unit_local_put_rejects_hash_mismatch \
  ep037_unit_local_get_detects_corruption \
  ep037_unit_local_delete_verifies_absence \
  ep037_unit_local_create_backup_verifies_hashes_and_writes_manifest \
  ep037_unit_local_restore_validates_when_all_hashes_present \
  ep037_unit_local_migrate_verifies_objects_on_target \
  ep037_unit_local_list_pages_artifacts; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-037-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-037-owned M2 behavior tests observed"

# Clippy -D warnings and fmt.
if ! sh -c 'cargo clippy -p nexus-provider-storage-local --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-provider-storage-local -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# M1 regression: the contract suite must stay green.
if ! sh scripts/ep037-m1-tests.sh > /tmp/ep037-m2-m1regress.log 2>&1; then
  fail "M1 regression failed" /tmp/ep037-m2-m1regress.log
fi
ok "M1 regression green"

echo "EP-037 M2 gate: ok"
