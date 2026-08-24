#!/usr/bin/env sh
# EP-041 M3 gate: deterministic frozen-eval behavior.
#
# Non-vacuous: M1 + M2 regressions, real pytest proof count, real eval
# fixtures exercised, anti-masking sentinels, dependency-direction,
# no-placeholder scan, ruff lint + format on the owned surface.
set -eu
. scripts/env.sh
export NO_COLOR=1

log="/tmp/ep041-m3-tests.log"
: > "$log"

fail() {
  echo "EP-041 M3 gate: FAIL - $1" >&2
  exit 1
}

# --- M1 + M2 regressions first ---------------------------------------------
if ! sh scripts/ep041-m1-tests.sh >"$log" 2>&1; then
  echo "EP-041 M3 gate: FAIL - M1 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! sh scripts/ep041-m2-tests.sh >"$log" 2>&1; then
  echo "EP-041 M3 gate: FAIL - M2 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi

# --- material presence ------------------------------------------------------
for path in \
  python/nexus_microbrain/eval_policy.py \
  microbrain/evals/suites/nexus-frozen-suite-v1.eval.json \
  microbrain/evals/suites/nexus-frozen-suite-v1.binding.json \
  tests/microbrain/test_ep041_m3_eval_policy.py; do
  [ -f "$path" ] || fail "missing owned path: $path"
done

# --- real eval fixtures are valid JSON --------------------------------------
for fixture in \
  microbrain/evals/suites/nexus-frozen-suite-v1.eval.json \
  microbrain/evals/suites/nexus-frozen-suite-v1.binding.json; do
  python3 -m json.tool "$fixture" >/dev/null 2>&1 || fail "invalid JSON fixture: $fixture"
done

# --- anti-masking sentinels --------------------------------------------------
grep -q 'ep041-m3-tests.sh' scripts/nodes/EP-041.sh || fail "node M3 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-041 M3' scripts/nodes/EP-041.sh; then
  fail "node M3 still uses artifact-check masking"
fi

# --- real pytest with vacuity guard ------------------------------------------
if ! uv run --frozen pytest tests/microbrain -q --tb=short \
  -o python_functions="ep041_unit_*" >>"$log" 2>&1; then
  echo "EP-041 M3 gate: FAIL - pytest ep041_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi
if ! grep -Eq '^[0-9]+ passed' "$log"; then
  fail "no tests ran (vacuity guard)"
fi
count=$(grep -Eo '^[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 115 ]; then
  fail "too few proofs passed: ${count:-0} (need >= 115)"
fi
if grep -Eq '[1-9][0-9]* failed|[1-9][0-9]* error' "$log"; then
  fail "failures/errors present in pytest output"
fi

# --- frozen-eval negative proofs are present ---------------------------------
negatives=$(grep -cE 'def ep041_unit_m3_.*(rejected|fails_closed|blocks)\(' \
  tests/microbrain/test_ep041_m3_eval_policy.py || true)
if [ "${negatives:-0}" -lt 10 ]; then
  fail "M3 fail-closed negative proofs not present: ${negatives:-0}"
fi

# --- deterministic scoring proofs are present --------------------------------
scoring=$(grep -cE 'def ep041_unit_m3_(scoring|suite_score|all_pass|any_fail)' \
  tests/microbrain/test_ep041_m3_eval_policy.py || true)
if [ "${scoring:-0}" -lt 3 ]; then
  fail "M3 deterministic scoring proofs not present: ${scoring:-0}"
fi

# --- dependency-direction: contract crate is provider-neutral ----------------
forbidden='import requests|import httpx|import boto3|import torch|import transformers|import openai|import anthropic|import numpy|import pandas|nexus_connector_sdk'
if grep -rEn --exclude-dir=__pycache__ "$forbidden" python/nexus_microbrain; then
  fail "forbidden provider dependency import in contract crate"
fi

# --- no-placeholder scan ------------------------------------------------------
if grep -rEn --exclude-dir=__pycache__ --include='*.py' 'TODO|FIXME|not implemented|placeholder' \
  python/nexus_microbrain tests/microbrain; then
  fail "placeholder marker in owned sources"
fi

# --- ruff lint + format on owned surface --------------------------------------
if ! uv run --frozen ruff check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M3 gate: FAIL - ruff check" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! uv run --frozen ruff format --check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M3 gate: FAIL - ruff format check" >&2
  tail -20 "$log" >&2
  exit 1
fi

tail -2 "$log"
echo "EP-041 M3 gate: ok"
