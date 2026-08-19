#!/usr/bin/env sh
# EP-027 M1 gate: run the nexus-fax contract suite through the REAL
# cargo test machinery with vacuity guards.
#
# The M1 changed-file fence is crates/nexus-fax/ (contract crate) plus
# the node script and plan files, so the authoritative gate is the
# nexus-fax cargo suite (unit + dependency-direction) plus fmt/clippy
# on the crate. Vacuity guards are required: `cargo test <filter>`
# exits 0 on a zero-match filter (EP-001 gate-masking class), so a
# green M1 must observe a real non-zero passing count, the
# dependency-direction test, and zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep027-m1-tests.log"
: > "$log"

# Vacuity guard 0: the contract crate must exist.
if [ ! -f crates/nexus-fax/Cargo.toml ]; then
  echo "EP-027 M1: FAIL - crates/nexus-fax/Cargo.toml missing" >&2
  exit 1
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/error.rs src/vocabulary.rs src/provider.rs src/lib.rs; do
  if [ ! -f "crates/nexus-fax/$f" ]; then
    echo "EP-027 M1: FAIL - crates/nexus-fax/$f missing" >&2
    exit 1
  fi
done

# Real build + full crate suite (all targets: unit + dependency
# direction). `--all-targets` ensures the integration test binary is
# compiled and run, not silently skipped.
if ! cargo test --locked -p nexus-fax --all-targets >>"$log" 2>&1; then
  echo "EP-027 M1: FAIL - cargo test -p nexus-fax --all-targets failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-027 M1: FAIL - no tests ran (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 2: a passing result with a non-zero count and zero
# failures is observed in the run output.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$log"; then
  echo "EP-027 M1: FAIL - no passing non-vacuous result (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 3: the dependency-direction test ran and passed.
if ! grep -q 'ep027_unit_dependency_direction .* ok' "$log"; then
  echo "EP-027 M1: FAIL - dependency-direction test did not pass" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 4: the M1 contract floor is met. The owned suite is
# 16 tests (15 unit + 1 dependency direction) as of the M1 commit;
# shrinking the floor requires a Decision Log entry. Passed counts are
# summed across all test binaries in the run.
total_passed=$(awk '/test result: ok\. [0-9]+ passed; 0 failed/ {
  for (i = 1; i <= NF; i++) if ($i ~ /^[0-9]+$/) { sum += $i; break }
} END { print sum+0 }' "$log")
if [ "$total_passed" -lt 16 ]; then
  echo "EP-027 M1: FAIL - M1 contract test floor (16) not met (got $total_passed)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 5: no ignored or filtered tests hide gaps.
if grep -qE '[1-9][0-9]* ignored' "$log"; then
  echo "EP-027 M1: FAIL - ignored tests present (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# fmt and clippy on the crate (milestone convention).
if ! cargo fmt -p nexus-fax --check >>"$log" 2>&1; then
  echo "EP-027 M1: FAIL - cargo fmt --check failed" >&2
  tail -20 "$log" >&2
  exit 1
fi

if ! cargo clippy -p nexus-fax --all-targets -- -D warnings >>"$log" 2>&1; then
  echo "EP-027 M1: FAIL - cargo clippy -D warnings failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

tail -8 "$log"
echo "EP-027 M1: ok"
