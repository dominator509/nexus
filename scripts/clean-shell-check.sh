#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
# Regression (EP-003 M5, owner clarification): the canonical command
# environment must let a fresh noninteractive shell run the toolchain gate
# without a manual PATH preamble. Run the REAL toolchain check inside a
# scrubbed environment that mimics a brand-new noninteractive login: only
# HOME and the standard Ubuntu default PATH (which includes /usr/local/bin
# where system tools like sops live), no mise shims inherited. env.sh
# (sourced by toolchain-check.sh) must re-establish the locked toolchain.
out=$(env -i HOME="$HOME" PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin sh scripts/toolchain-check.sh 2>&1) || {
  printf '%s\n' "$out"
  echo "clean shell check: FAIL - toolchain check failed from a clean noninteractive shell" >&2
  exit 1
}
printf '%s\n' "$out" | grep -q 'toolchain check: ok' || {
  printf '%s\n' "$out"
  echo "clean shell check: FAIL - toolchain check sentinel missing" >&2
  exit 1
}
echo "clean shell check: ok"
