#!/usr/bin/env sh
# EP-021 M2 gate: run the wake model core suite through real pytest with
# a vacuity guard.
#
# The M2 changed-files fence is models/wake/ (Python wake core), so the
# authoritative gate is the ep021_unit pytest suite under models/wake/.
# The vacuity guard is required: pytest exits 0 on a zero-collected run
# (EP-001 gate-masking class), so a zero-match invocation must fail.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep021-m2-tests.log"
: > "$log"

# Resolve a python3 that has pytest (EP-011 sidecar precedent). Under
# scripts/env.sh the mise shim python3 shadows PATH and may lack pytest,
# so probe explicitly instead of trusting PATH (EP-001 gate-masking
# class; fail closed if none resolves).
_py=""
for _cand in /root/hermes-env/bin/python3 /usr/bin/python3 python3; do
  if command -v "$_cand" >/dev/null 2>&1 && "$_cand" -c 'import pytest' >/dev/null 2>&1; then
    _py="$_cand"
    break
  fi
done
[ -n "$_py" ] || { echo "EP-021 M2: FAIL - no python3 with pytest" >&2; exit 1; }

if ! "$_py" -m pytest models/wake/tests -q -o 'python_functions=ep021_unit_*' -k ep021_unit >>"$log" 2>&1; then
  echo "EP-021 M2: FAIL - pytest wake core suite failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# Vacuity guard: at least one ep021_unit test passed.
if ! grep -qE '^[0-9]+ passed' "$log"; then
  echo "EP-021 M2: FAIL - no ep021_unit tests passed (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-021 M2: ok"
