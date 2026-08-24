#!/usr/bin/env sh
# EP-041 M4 gate: training candidate eligibility and plan behavior.
#
# Non-vacuous: M1 + M2 + M3 regressions, real pytest proof count, real
# training fixtures exercised, anti-masking sentinels, dependency-
# direction, no-placeholder scan, ruff lint + format on owned surface.
set -eu
. scripts/env.sh
export NO_COLOR=1

log="/tmp/ep041-m4-tests.log"
: > "$log"

fail() {
  echo "EP-041 M4 gate: FAIL - $1" >&2
  exit 1
}

# --- M1 + M2 + M3 regressions first ----------------------------------------
if ! sh scripts/ep041-m1-tests.sh >"$log" 2>&1; then
  echo "EP-041 M4 gate: FAIL - M1 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! sh scripts/ep041-m2-tests.sh >"$log" 2>&1; then
  echo "EP-041 M4 gate: FAIL - M2 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! sh scripts/ep041-m3-tests.sh >"$log" 2>&1; then
  echo "EP-041 M4 gate: FAIL - M3 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi

# --- material presence ------------------------------------------------------
for path in \
  python/nexus_microbrain/training_policy.py \
  microbrain/training/plans/nexus-candidate-v1.candidate.json \
  microbrain/training/plans/nexus-training-plan-v1.plan.json \
  tests/microbrain/test_ep041_m4_training_policy.py; do
  [ -f "$path" ] || fail "missing owned path: $path"
done

# --- real training fixtures are valid JSON ----------------------------------
for fixture in \
  microbrain/training/plans/nexus-candidate-v1.candidate.json \
  microbrain/training/plans/nexus-training-plan-v1.plan.json; do
  python3 -m json.tool "$fixture" >/dev/null 2>&1 || fail "invalid JSON fixture: $fixture"
done

# --- anti-masking sentinels --------------------------------------------------
grep -q 'ep041-m4-tests.sh' scripts/nodes/EP-041.sh || fail "node M4 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-041 M4' scripts/nodes/EP-041.sh; then
  fail "node M4 still uses artifact-check masking"
fi

# --- real pytest with vacuity guard ------------------------------------------
if ! uv run --frozen pytest tests/microbrain -q --tb=short \
  -o python_functions="ep041_unit_*" >>"$log" 2>&1; then
  echo "EP-041 M4 gate: FAIL - pytest ep041_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi
if ! grep -Eq '^[0-9]+ passed' "$log"; then
  fail "no tests ran (vacuity guard)"
fi
count=$(grep -Eo '^[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 160 ]; then
  fail "too few proofs passed: ${count:-0} (need >= 160)"
fi
if grep -Eq '[1-9][0-9]* failed|[1-9][0-9]* error' "$log"; then
  fail "failures/errors present in pytest output"
fi

# --- candidate eligibility negative proofs are present ------------------------
negatives=$(grep -cE 'def ep041_unit_m4_.*(denied|rejected|fails_closed|never_certify)\(' \
  tests/microbrain/test_ep041_m4_training_policy.py || true)
if [ "${negatives:-0}" -lt 15 ]; then
  fail "M4 fail-closed negative proofs not present: ${negatives:-0}"
fi

# --- training-plan-not-executed proofs are present ----------------------------
plan_not_executed=$(grep -cE 'def ep041_unit_m4_(plan_ready_only_never_executed|plan_does_not_create_run_or_promotion|qlora_declared_not_executed|qlora_metrics_alone_never_certify)\(' \
  tests/microbrain/test_ep041_m4_training_policy.py || true)
if [ "${plan_not_executed:-0}" -lt 4 ]; then
  fail "training-plan-not-executed proofs not present: ${plan_not_executed:-0}"
fi

# --- leakage negative proofs are present --------------------------------------
leakage=$(grep -cE 'def ep041_unit_m4_leakage_.*(denied|fails_closed)\(' \
  tests/microbrain/test_ep041_m4_training_policy.py || true)
if [ "${leakage:-0}" -lt 3 ]; then
  fail "leakage negative proofs not present: ${leakage:-0}"
fi

# --- dependency-direction: contract crate is provider-neutral ------------------
forbidden='import requests|import httpx|import boto3|import torch|import transformers|import openai|import anthropic|import numpy|import pandas|nexus_connector_sdk'
if grep -rEn --exclude-dir=__pycache__ "$forbidden" python/nexus_microbrain; then
  fail "forbidden provider dependency import in contract crate"
fi

# --- no-placeholder scan -------------------------------------------------------
if grep -rEn --exclude-dir=__pycache__ --include='*.py' 'TODO|FIXME|not implemented|placeholder' \
  python/nexus_microbrain tests/microbrain; then
  fail "placeholder marker in owned sources"
fi

# --- ruff lint + format on owned surface ----------------------------------------
if ! uv run --frozen ruff check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M4 gate: FAIL - ruff check" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! uv run --frozen ruff format --check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M4 gate: FAIL - ruff format check" >&2
  tail -20 "$log" >&2
  exit 1
fi

tail -2 "$log"
echo "EP-041 M4 gate: ok"
