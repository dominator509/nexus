#!/usr/bin/env sh
# EP-028 M1 gate: run the nexus-hydra contract suite through the REAL
# cargo test machinery with vacuity guards.
#
# The M1 changed-file fence is crates/nexus-hydra/ (contract crate) plus
# the node script and plan files, so the authoritative gate is the
# nexus-hydra cargo suite (unit + dependency-direction) plus fmt/clippy
# on the crate. Vacuity guards are required: `cargo test <filter>`
# exits 0 on a zero-match filter (EP-001 gate-masking class), so a
# green M1 must observe a real non-zero passing count, the
# dependency-direction test, an EP-028-owned sentinel test name, and
# zero ignored/filtered tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep028-m1-tests.log"
: > "$log"

fail() {
  echo "EP-028 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-028 M1 gate: $1"; }

# Vacuity guard 0: the contract crate must exist.
if [ ! -f crates/nexus-hydra/Cargo.toml ]; then
  fail "crates/nexus-hydra/Cargo.toml missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in src/lib.rs src/error.rs src/vocabulary.rs src/model.rs \
         src/capability.rs src/action.rs src/context.rs src/events.rs \
         src/provider.rs tests/dependency_direction.rs; do
  if [ ! -f "crates/nexus-hydra/$f" ]; then
    fail "crates/nexus-hydra/$f missing"
  fi
done
ok "contract crate and sources present"

# Real build + full crate suite (all targets: unit + dependency
# direction). `--all-targets` ensures the integration test binary is
# compiled and run, not silently skipped.
if ! cargo test --locked -p nexus-hydra --all-targets >>"$log" 2>&1; then
  fail "cargo test -p nexus-hydra --all-targets failed" "$log"
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
if ! grep -q 'ep028_unit_dependency_direction .* ok' "$log"; then
  fail "dependency-direction test did not pass" "$log"
fi

# Vacuity guard 4 (anti-masking): an EP-028-owned sentinel test must be
# observed. This fails if the gate accidentally executes only a prior
# node's tests.
if ! grep -q 'ep028_unit_binding_explicit_tenant_and_business .* ok' "$log"; then
  fail "EP-028-owned sentinel test did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 5: no required test was ignored or filtered out.
if grep -qE 'test result: ok\. [0-9]+ passed; [0-9]+ ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi
ok "real suite passed ($(grep -oE 'test result: ok\. [0-9]+ passed' "$log" | awk '{s+=$4} END {print s}') tests total)"

# Milestone artifact/fence checks: M1 fence paths exist.
for f in .agent/milestone-files/EP-028-M1.txt .agent/node-contracts/EP-028.md \
         .agent/execplans/EP-028-hydra-business-control-plane.md; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-028 M1: ok"
