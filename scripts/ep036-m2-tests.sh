#!/usr/bin/env sh
# EP-036 M2 gate: run the AWS provider binding suite + OpenTofu module
# validation through the REAL cargo and OpenTofu machinery with vacuity
# guards.
#
# The M2 changed-file fence is infra/opentofu/ (OpenTofu module root) +
# providers/aws/ (AWS provider binding crate), so the authoritative gate
# is the AWS binding crate suite (cargo test -p nexus-provider-aws),
# the OpenTofu module proof (tofu validate + tofu fmt --check against
# the real OpenTofu binary), and M1 regressions. Vacuity guards are
# required: a green M2 must observe real non-zero passing counts,
# EP-036-owned test names, and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep036-m2-tests.log"
: > "$log"

fail() {
  echo "EP-036 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-036 M2 gate: $1"; }

# Vacuity guard 0: the owned roots must exist.
if [ ! -f providers/aws/Cargo.toml ]; then
  fail "providers/aws/Cargo.toml missing"
fi
if [ ! -f providers/aws/src/lib.rs ]; then
  fail "providers/aws/src/lib.rs missing"
fi
for f in README.md main.tf variables.tf; do
  if [ ! -f "infra/opentofu/modules/aws/$f" ]; then
    fail "infra/opentofu/modules/aws/$f missing"
  fi
done
ok "providers/aws + infra/opentofu roots present"

# Real OpenTofu binary must be available.
if ! command -v tofu >/dev/null 2>&1; then
  fail "tofu binary not found (OpenTofu required by SPEC-016)"
fi
ok "OpenTofu binary present"

# OpenTofu module proof: validate + fmt --check on the real module.
# A plan against a live cloud account is NOT ASSERTED at M2 (no real
# cloud account; provider certification is owned by later milestones).
if ! (cd infra/opentofu/modules/aws && tofu init -backend=false -input=false >>"$log" 2>&1); then
  fail "tofu init failed" "$log"
fi
if ! (cd infra/opentofu/modules/aws && tofu validate >>"$log" 2>&1); then
  fail "tofu validate failed" "$log"
fi
if ! (cd infra/opentofu/modules/aws && tofu fmt -check . >>"$log" 2>&1); then
  fail "tofu fmt --check failed" "$log"
fi
# Remove OpenTofu scratch (init artifacts must never be committed).
rm -rf infra/opentofu/modules/aws/.terraform infra/opentofu/modules/aws/.terraform.lock.hcl
ok "OpenTofu module validate + fmt clean"

# Real AWS binding suite through cargo.
if ! sh -c 'cargo test -p nexus-provider-aws --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test -p nexus-provider-aws failed" "$log"
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

# Vacuity guard 4 (anti-masking): EP-036-owned AWS binding proofs observed.
for sentinel in \
  ep036_unit_aws_region_slug_shape \
  ep036_unit_aws_binding_rejects_bad_region \
  ep036_unit_aws_binding_is_provider_kind_aws; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-036-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all EP-036-owned M2 proofs observed"

total=$(grep -oE 'test result: ok\. [1-9][0-9]* passed' "$log" | awk '{s+=$4} END {print s}')
ok "real AWS binding suite passed (${total} tests total)"

# Native compile/typecheck + format for the new crate.
if ! sh -c 'cargo clippy -p nexus-provider-aws --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"
if ! cargo fmt -p nexus-provider-aws -- --check >>"$log" 2>&1; then
  fail "cargo fmt --check failed" "$log"
fi
ok "cargo fmt clean"

# M1 regression: the compute fabric contract must remain green.
sh scripts/ep036-m1-tests.sh >>"$log" 2>&1 || fail "M1 regression failed" "$log"
ok "EP-036 M1 regression green"

echo "EP-036 M2 gate: ok"
