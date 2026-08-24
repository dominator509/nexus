#!/usr/bin/env sh
# EP-041 M2 gate: deterministic Microbrain dataset policy behavior.
#
# Non-vacuous: M1 regression, real pytest proof count, real manifest
# fixtures exercised, anti-masking sentinels, dependency-direction,
# no-placeholder scan, ruff lint + format on the owned surface.
set -eu
. scripts/env.sh
export NO_COLOR=1

log="/tmp/ep041-m2-tests.log"
: > "$log"

fail() {
  echo "EP-041 M2 gate: FAIL - $1" >&2
  exit 1
}

# --- M1 regression first ---------------------------------------------------
if ! sh scripts/ep041-m1-tests.sh >"$log" 2>&1; then
  echo "EP-041 M2 gate: FAIL - M1 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi

# --- material presence -----------------------------------------------------
for path in \
  python/nexus_microbrain/dataset_policy.py \
  microbrain/datasets/manifests/nexus-synthetic-role-ops-v1.manifest.json \
  microbrain/datasets/manifests/nexus-teacher-consensus-v1.manifest.json \
  tests/microbrain/test_ep041_m2_dataset_policy.py; do
  [ -f "$path" ] || fail "missing owned path: $path"
done

# --- real manifest fixtures are valid JSON ---------------------------------
for manifest in \
  microbrain/datasets/manifests/nexus-synthetic-role-ops-v1.manifest.json \
  microbrain/datasets/manifests/nexus-teacher-consensus-v1.manifest.json; do
  python3 -m json.tool "$manifest" >/dev/null 2>&1 || fail "invalid JSON manifest: $manifest"
done

# --- anti-masking sentinels ------------------------------------------------
grep -q 'ep041-m2-tests.sh' scripts/nodes/EP-041.sh || fail "node M2 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-041 M2' scripts/nodes/EP-041.sh; then
  fail "node M2 still uses artifact-check masking"
fi

# --- real pytest with vacuity guard ----------------------------------------
if ! uv run --frozen pytest tests/microbrain -q --tb=short \
  -o python_functions="ep041_unit_*" >>"$log" 2>&1; then
  echo "EP-041 M2 gate: FAIL - pytest ep041_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi
if ! grep -Eq '^[0-9]+ passed' "$log"; then
  fail "no tests ran (vacuity guard)"
fi
count=$(grep -Eo '^[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 75 ]; then
  fail "too few proofs passed: ${count:-0} (need >= 75)"
fi
if grep -Eq '[1-9][0-9]* failed|[1-9][0-9]* error' "$log"; then
  fail "failures/errors present in pytest output"
fi

# --- dataset negative proofs are present -----------------------------------
collected=$(grep -cE 'def ep041_unit_m2_.*(denied|fails_closed)\(' \
  tests/microbrain/test_ep041_m2_dataset_policy.py || true)
if [ "${collected:-0}" -lt 6 ]; then
  fail "M2 dataset fail-closed negative proofs not present: ${collected:-0}"
fi

# --- dependency-direction: contract crate is provider-neutral --------------
forbidden='import requests|import httpx|import boto3|import torch|import transformers|import openai|import anthropic|import numpy|import pandas|nexus_connector_sdk'
if grep -rEn --exclude-dir=__pycache__ "$forbidden" python/nexus_microbrain; then
  fail "forbidden provider dependency import in contract crate"
fi

# --- no-placeholder scan ----------------------------------------------------
if grep -rEn --exclude-dir=__pycache__ --include='*.py' 'TODO|FIXME|not implemented|placeholder' \
  python/nexus_microbrain tests/microbrain; then
  fail "placeholder marker in owned sources"
fi

# --- ruff lint + format on owned surface ------------------------------------
if ! uv run --frozen ruff check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M2 gate: FAIL - ruff check" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! uv run --frozen ruff format --check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M2 gate: FAIL - ruff format check" >&2
  tail -20 "$log" >&2
  exit 1
fi

tail -2 "$log"
echo "EP-041 M2 gate: ok"
