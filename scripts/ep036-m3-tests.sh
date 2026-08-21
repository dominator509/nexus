#!/usr/bin/env sh
# EP-036 M3 gate: run the cloud-init config validation + existing-SSH
# binding/transport suite through the REAL cloud-init and cargo
# machinery with vacuity guards.
#
# The M3 changed-file fence is infra/cloud-init/ (cloud-init config
# root) + providers/existing-ssh/ (generic SSH provider binding crate),
# so the authoritative gate is the cloud-init schema lint (real
# cloud-init binary), the existing-SSH crate suite (cargo test -p
# nexus-provider-existing-ssh, including the real ephemeral sshd
# transport integration), and M1+M2 regressions.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep036-m3-tests.log"
: > "$log"

fail() {
  echo "EP-036 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-036 M3 gate: $1"; }

# Vacuity guard 0: the owned roots must exist.
if [ ! -f infra/cloud-init/nexus-node.cfg ]; then
  fail "infra/cloud-init/nexus-node.cfg missing"
fi
if [ ! -f providers/existing-ssh/Cargo.toml ]; then
  fail "providers/existing-ssh/Cargo.toml missing"
fi
if [ ! -f providers/existing-ssh/src/lib.rs ]; then
  fail "providers/existing-ssh/src/lib.rs missing"
fi
if [ ! -f providers/existing-ssh/tests/ep036_integration_existing_ssh.rs ]; then
  fail "providers/existing-ssh/tests/ep036_integration_existing_ssh.rs missing"
fi
ok "cloud-init + existing-ssh roots present"

# Real cloud-init schema validation.
if ! command -v cloud-init >/dev/null 2>&1; then
  fail "cloud-init binary not found"
fi
if ! (cd infra/cloud-init && cloud-init schema --config-file nexus-node.cfg >>"$log" 2>&1); then
  fail "cloud-init schema validation failed" "$log"
fi
if ! grep -q "Valid schema nexus-node.cfg" "$log"; then
  fail "cloud-init schema did not print the Valid schema sentinel" "$log"
fi
ok "cloud-init schema lint clean"

# Real existing-SSH suite (unit + ephemeral sshd transport integration).
# --nocapture is REQUIRED: the real-transport sentinel is an eprintln
# proof that ssh-keyscan actually reached the ephemeral sshd; captured
# output would make the anti-masking guard vacuous.
if ! sh -c 'cargo test -p nexus-provider-existing-ssh --locked -- --nocapture >> "$1" 2>&1' _ "$log"; then
  fail "cargo test -p nexus-provider-existing-ssh failed" "$log"
fi

# Vacuity guard 1: non-zero pass observed.
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

# Vacuity guard 4 (anti-masking): EP-036-owned proofs observed,
# including the REAL transport integration. The positive transport
# sentinel is only printed when ssh-keyscan actually reached the
# ephemeral sshd; a docker-unavailable silent skip can never print it.
for sentinel in \
  ep036_unit_existing_ssh_binding_validates \
  ep036_unit_existing_ssh_binding_rejects_bad_host \
  ep036_unit_existing_ssh_binding_maps_to_generic_ssh \
  ep036_real_transport_ssh_keyscan_probe; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-036-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
if grep -q "docker unavailable; skipping" "$log"; then
  fail "integration silently skipped docker (vacuity guard)" "$log"
fi
ok "all EP-036-owned M3 proofs observed"

total=$(grep -oE 'test result: ok\. [1-9][0-9]* passed' "$log" | awk '{s+=$4} END {print s}')
ok "real existing-SSH suite passed (${total} tests total)"

# Native compile/typecheck + format for the new crate.
if ! sh -c 'cargo clippy -p nexus-provider-existing-ssh --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"
if ! cargo fmt -p nexus-provider-existing-ssh -- --check >>"$log" 2>&1; then
  fail "cargo fmt --check failed" "$log"
fi
ok "cargo fmt clean"

# M1 + M2 regressions: the compute fabric contract and AWS/OpenTofu work
# must remain green.
sh scripts/ep036-m1-tests.sh >>"$log" 2>&1 || fail "M1 regression failed" "$log"
ok "EP-036 M1 regression green"
sh scripts/ep036-m2-tests.sh >>"$log" 2>&1 || fail "M2 regression failed" "$log"
ok "EP-036 M2 regression green"

echo "EP-036 M3 gate: ok"
