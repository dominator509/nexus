#!/usr/bin/env sh
# EP-031 M1 gate: run the nexus-sentinel-advanced contract suite and
# the Suricata connector package-boundary suite through the REAL cargo
# test machinery with vacuity guards.
#
# The M1 changed-file fence is infra/sentinel/advanced/ (contract
# crate) plus connectors/suricata/ (package boundary) and the node
# script and plan files, so the authoritative gate is the advanced
# contract cargo suite (unit + dependency-direction) plus the Suricata
# connector crate suite plus fmt/clippy on the crates. Vacuity guards
# are required: `cargo test <filter>` exits 0 on a zero-match filter
# (EP-001 gate-masking class), so a green M1 must observe a real
# non-zero passing count, the dependency-direction test, an
# EP-031-owned test name, and zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep031-m1-tests.log"
: > "$log"

fail() {
  echo "EP-031 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-031 M1 gate: $1"; }

# Vacuity guard 0: the contract crate must exist.
if [ ! -f infra/sentinel/advanced/Cargo.toml ]; then
  fail "infra/sentinel/advanced/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/error.rs src/vocabulary.rs src/model.rs \
         src/provider.rs tests/dependency_direction.rs; do
  if [ ! -f "infra/sentinel/advanced/$f" ]; then
    fail "infra/sentinel/advanced/$f missing"
  fi
done
ok "advanced contract crate and sources present"

# Real build + full crate suite (all targets: unit + dependency
# direction). `--all-targets` ensures the integration test binary is
# compiled and run, not silently skipped. Use the real cargo binary
# directly (the shell alias wraps rust-rtk-tee which collapses output).
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-sentinel-advanced --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-sentinel-advanced --all-targets failed" "$log"
fi

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: a passing result with a non-zero count and zero
# failures is observed in the run output.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed; 0 failed' "$log"; then
  fail "no passing non-vacuous result (vacuity guard)" "$log"
fi

# Vacuity guard 3: the dependency-direction test ran and passed.
if ! grep -q 'ep031_unit_advanced_dependency_direction .* ok' "$log"; then
  fail "dependency-direction test did not pass" "$log"
fi

# Vacuity guard 4 (anti-masking): an EP-031-owned advanced sentinel
# test must be observed. This fails if the gate accidentally executes
# only a prior node's tests.
if ! grep -q 'ep031_unit_sensor_profiles_model_all_optional_classes .* ok' "$log"; then
  fail "EP-031-owned advanced sentinel test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 5: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
if grep -q 'filtered out' "$log" && ! grep -q '0 filtered out' "$log"; then
  fail "required tests were filtered (vacuity guard)" "$log"
fi
ok "real suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# The Suricata connector package-boundary crate must compile and its
# own ep031_unit_* tests must pass.
if ! "${CARGO_BIN:-$HOME/.cargo/bin/cargo}" test --offline -p nexus-suricata-connector --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-suricata-connector --all-targets failed" "$log"
fi
if ! grep -q 'ep031_unit_suricata_alert_severity_is_bounded .* ok' "$log"; then
  fail "Suricata connector EP-031-owned test did not run (anti-masking guard)" "$log"
fi
ok "Suricata connector package-boundary suite passed"

# Milestone artifact/fence checks: M1 fence paths exist.
for f in .agent/milestone-files/EP-031-M1.txt .agent/node-contracts/EP-031.md \
         .agent/execplans/EP-031-sentinel-advanced-detection-and-endpoints.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-031 M1: ok"
