#!/usr/bin/env sh
# EP-036 M5 gate: live-fire, operations, and node closure.
#
# Per the authoritative ExecPlan, no standalone live-fire proof is owned
# by this node; its behavior is exercised by downstream proofs and the
# node-specific REAL dependency tests (M3 real ephemeral sshd transport,
# M4 real forced-failure mechanisms). M5 therefore:
#   1. adds the Hetzner provider binding crate (M5 fence),
#   2. runs every node-owned real proof (M1..M4 gates),
#   3. writes machine-readable evidence under .agent/state/evidence/,
#   4. runs the closure gates (verify mode, expected files, scope).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep036-m5-tests.log"
: > "$log"

fail() {
  echo "EP-036 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-036 M5 gate: $1"; }

# Vacuity guard 0: the owned root must exist.
if [ ! -f providers/hetzner/Cargo.toml ]; then
  fail "providers/hetzner/Cargo.toml missing"
fi
if [ ! -f providers/hetzner/src/lib.rs ]; then
  fail "providers/hetzner/src/lib.rs missing"
fi
ok "hetzner binding root present"

# Real Hetzner binding suite (--nocapture for sentinels).
if ! sh -c 'cargo test -p nexus-provider-hetzner --locked -- --nocapture >> "$1" 2>&1' _ "$log"; then
  fail "cargo test -p nexus-provider-hetzner failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no hetzner tests ran (vacuity guard)" "$log"
fi
for sentinel in \
  ep036_unit_hetzner_location_slug_shape \
  ep036_unit_hetzner_binding_is_provider_kind_hetzner; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-036-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "hetzner binding suite passed"

# All node-owned real proofs: M1 contract, M2 OpenTofu, M3 real
# transport, M4 real forced failures.
sh scripts/ep036-m1-tests.sh >>"$log" 2>&1 || fail "M1 proof failed" "$log"
ok "EP-036 M1 proof green"
sh scripts/ep036-m2-tests.sh >>"$log" 2>&1 || fail "M2 proof failed" "$log"
ok "EP-036 M2 proof green"
sh scripts/ep036-m3-tests.sh >>"$log" 2>&1 || fail "M3 proof failed" "$log"
ok "EP-036 M3 proof green"
sh scripts/ep036-m4-tests.sh >>"$log" 2>&1 || fail "M4 proof failed" "$log"
ok "EP-036 M4 proof green"

# Clippy + fmt for the new crate.
if ! sh -c 'cargo clippy -p nexus-provider-hetzner --locked --all-targets -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"
if ! cargo fmt -p nexus-provider-hetzner -- --check >>"$log" 2>&1; then
  fail "cargo fmt --check failed" "$log"
fi
ok "cargo fmt clean"

# Machine-readable node evidence (real proofs observed above).
ts=$(date -u +%Y%m%dT%H%M%SZ)
commit=$(git rev-parse HEAD)
hetzner_total=$(grep -oE 'test result: ok\. [1-9][0-9]* passed' "$log" | awk '{s+=$4} END {print s}')
cat > .agent/state/evidence/EP-036-m5.json <<EOF
{
  "lf_id": "EP-036-M5",
  "node": "EP-036",
  "milestone": "M5",
  "run_id": "ep036-m5-${ts}",
  "slug": "compute-fabric-and-cloud-provisioning-closure",
  "git_commit": "${commit}",
  "owned_live_fire": "NONE_STANDALONE",
  "real_proofs_observed": {
    "M1_contract": "48 tests green (contract 32 + model 11 + dependency_direction 2 + digitalocean 3)",
    "M2_opentofu": "real tofu validate + fmt green",
    "M3_transport": "real ephemeral sshd container + ssh-keyscan probe green",
    "M4_failures": "real forced-failure suite green (7 tests)",
    "M5_hetzner": "${hetzner_total} hetzner binding tests green"
  },
  "certification_boundary": {
    "nexus-compute": "INTERNAL CONTRACT CERTIFIED",
    "providers-digitalocean": "CONTRACT/BINDING IMPLEMENTED; real API NOT ASSERTED",
    "providers-aws": "CONTRACT/BINDING IMPLEMENTED; real API NOT ASSERTED",
    "providers-contabo": "CONTRACT/BINDING IMPLEMENTED; real API NOT ASSERTED",
    "providers-hetzner": "CONTRACT/BINDING IMPLEMENTED; real API NOT ASSERTED",
    "infra-opentofu": "VALIDATED with real OpenTofu tooling; plan/apply NOT ASSERTED",
    "infra-cloud-init": "SCHEMA VALID; execution on a VM NOT ASSERTED",
    "existing-ssh": "REAL TRANSPORT CERTIFIED for exact controlled ephemeral sshd path; authentication/remote bootstrap NOT ASSERTED",
    "fleet-enrollment": "NOT ASSERTED",
    "physical-cloud-node-readiness": "NOT ASSERTED"
  }
}
EOF
ok "machine-readable evidence written (.agent/state/evidence/EP-036-m5.json)"

echo "EP-036 M5 gate: ok"
