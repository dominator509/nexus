#!/usr/bin/env sh
# EP-027 M2 gate: run the nexus-ictfax adapter suite through the REAL
# cargo test machinery with vacuity guards.
#
# The M2 changed-file fence is connectors/ictfax/ (adapter crate), so
# the authoritative gate is the nexus-ictfax cargo suite plus the M1
# contract regression. Vacuity guards are required: `cargo test
# <filter>` exits 0 on a zero-match filter (EP-001 gate-masking
# class), so a green M2 must observe a real non-zero passing count,
# the M1 contract regression, and zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep027-m2-tests.log"
: > "$log"

# Vacuity guard 0: the adapter crate must exist.
if [ ! -f connectors/ictfax/Cargo.toml ]; then
  echo "EP-027 M2: FAIL - connectors/ictfax/Cargo.toml missing" >&2
  exit 1
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/adapter.rs src/transport.rs src/observability.rs src/lib.rs; do
  if [ ! -f "connectors/ictfax/$f" ]; then
    echo "EP-027 M2: FAIL - connectors/ictfax/$f missing" >&2
    exit 1
  fi
done

# Real build + full adapter suite (all targets).
if ! cargo test --locked -p nexus-ictfax --all-targets >>"$log" 2>&1; then
  echo "EP-027 M2: FAIL - cargo test -p nexus-ictfax --all-targets failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# M1 contract regression (the M2 adapter depends on the M1 crate).
if ! cargo test --locked -p nexus-fax --all-targets >>"$log" 2>&1; then
  echo "EP-027 M2: FAIL - cargo test -p nexus-fax (M1 regression) failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-027 M2: FAIL - no tests ran (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 2: a passing result with a non-zero count and zero
# failures is observed in the run output.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$log"; then
  echo "EP-027 M2: FAIL - no passing non-vacuous result (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 3: the M2 floor is met. The owned adapter suite is 11
# tests as of the M2 commit; shrinking the floor requires a Decision
# Log entry. Passed counts are summed across all test binaries.
total_passed=$(awk '/test result: ok\. [0-9]+ passed; 0 failed/ {
  for (i = 1; i <= NF; i++) if ($i ~ /^[0-9]+$/) { sum += $i; break }
} END { print sum+0 }' "$log")
if [ "$total_passed" -lt 11 ]; then
  echo "EP-027 M2: FAIL - M2 adapter test floor (11) not met (got $total_passed)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 4: the adapter suite actually ran its named tests
# (anti-masking: the M2 gate must not pass on the M1 crate alone).
if ! grep -q 'ep027_unit_ictfax_' "$log"; then
  echo "EP-027 M2: FAIL - no nexus-ictfax ep027_unit tests ran (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# Vacuity guard 5: no ignored or filtered tests hide gaps.
if grep -qE '[1-9][0-9]* ignored' "$log"; then
  echo "EP-027 M2: FAIL - ignored tests present (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

# fmt and clippy on the crate (milestone convention).
if ! cargo fmt -p nexus-ictfax --check >>"$log" 2>&1; then
  echo "EP-027 M2: FAIL - cargo fmt --check failed" >&2
  tail -20 "$log" >&2
  exit 1
fi

if ! cargo clippy -p nexus-ictfax --all-targets -- -D warnings >>"$log" 2>&1; then
  echo "EP-027 M2: FAIL - cargo clippy -D warnings failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

tail -8 "$log"
echo "EP-027 M2: ok"
