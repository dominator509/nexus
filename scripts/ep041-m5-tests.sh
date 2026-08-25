#!/usr/bin/env sh
# EP-041 M5 gate: artifact, GGUF, shadow, promotion, and node closure.
#
# Non-vacuous: M1 + M2 + M3 + M4 regressions, real pytest proof count,
# real artifact fixtures exercised, anti-masking sentinels,
# dependency-direction, no-placeholder scan, ruff lint + format.
set -eu
. scripts/env.sh
export NO_COLOR=1

log="/tmp/ep041-m5-tests.log"
: > "$log"

fail() {
  echo "EP-041 M5 gate: FAIL - $1" >&2
  exit 1
}

# --- M1 + M2 + M3 + M4 regressions first ------------------------------------
if ! sh scripts/ep041-m1-tests.sh >"$log" 2>&1; then
  echo "EP-041 M5 gate: FAIL - M1 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! sh scripts/ep041-m2-tests.sh >"$log" 2>&1; then
  echo "EP-041 M5 gate: FAIL - M2 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! sh scripts/ep041-m3-tests.sh >"$log" 2>&1; then
  echo "EP-041 M5 gate: FAIL - M3 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! sh scripts/ep041-m4-tests.sh >"$log" 2>&1; then
  echo "EP-041 M5 gate: FAIL - M4 regression" >&2
  tail -20 "$log" >&2
  exit 1
fi

# --- material presence --------------------------------------------------------
for path in \
  python/nexus_microbrain/artifact_policy.py \
  microbrain/artifacts/fixtures/nexus-artifact-v1.artifact.json \
  microbrain/artifacts/fixtures/nexus-artifact-v1.gguf.marker \
  tests/microbrain/test_ep041_m5_artifact_policy.py; do
  [ -f "$path" ] || fail "missing owned path: $path"
done

# --- real artifact fixtures are valid JSON -------------------------------------
python3 -m json.tool microbrain/artifacts/fixtures/nexus-artifact-v1.artifact.json >/dev/null 2>&1 \
  || fail "invalid JSON artifact fixture"

# --- fixture-only marker is labeled -------------------------------------------
grep -q 'fixture-only' microbrain/artifacts/fixtures/nexus-artifact-v1.gguf.marker \
  || fail "GGUF marker not labeled fixture-only"

# --- anti-masking sentinels ------------------------------------------------------
grep -q 'ep041-m5-tests.sh' scripts/nodes/EP-041.sh || fail "node M5 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-041 M5' scripts/nodes/EP-041.sh; then
  fail "node M5 still uses artifact-check masking"
fi

# --- real pytest with vacuity guard ----------------------------------------------
if ! uv run --frozen pytest tests/microbrain -q --tb=short \
  -o python_functions="ep041_unit_*" >>"$log" 2>&1; then
  echo "EP-041 M5 gate: FAIL - pytest ep041_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi
if ! grep -Eq '^[0-9]+ passed' "$log"; then
  fail "no tests ran (vacuity guard)"
fi
count=$(grep -Eo '^[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 205 ]; then
  fail "too few proofs passed: ${count:-0} (need >= 205)"
fi
if grep -Eq '[1-9][0-9]* failed|[1-9][0-9]* error' "$log"; then
  fail "failures/errors present in pytest output"
fi

# --- artifact digest negative proofs are present ---------------------------------
negatives=$(grep -cE 'def ep041_unit_m5_.*(denied|fails_closed|blocks|never_certify|not_certified)\(' \
  tests/microbrain/test_ep041_m5_artifact_policy.py || true)
if [ "${negatives:-0}" -lt 12 ]; then
  fail "M5 fail-closed negative proofs not present: ${negatives:-0}"
fi

# --- shadow-not-promotion proofs are present --------------------------------------
shadow=$(grep -cE 'def ep041_unit_m5_(shadow_pass_advances_to_canary_not_promote|cannot_promote_directly_from_shadow|cannot_promote_with_false_positives|shadow_missing_evidence_fails_closed|shadow_false_positives_block)\(' \
  tests/microbrain/test_ep041_m5_artifact_policy.py || true)
if [ "${shadow:-0}" -lt 4 ]; then
  fail "shadow-not-promotion proofs not present: ${shadow:-0}"
fi

# --- promotion prerequisite negative proofs are present -----------------------------
promo=$(grep -cE 'def ep041_unit_m5_promotion_.*denies\(' \
  tests/microbrain/test_ep041_m5_artifact_policy.py || true)
if [ "${promo:-0}" -lt 6 ]; then
  fail "promotion prerequisite negative proofs not present: ${promo:-0}"
fi

# --- final live-fire composition proof is present ------------------------------------
livefire=$(grep -cE 'def ep041_unit_m5_final_live_fire_' \
  tests/microbrain/test_ep041_m5_artifact_policy.py || true)
if [ "${livefire:-0}" -lt 2 ]; then
  fail "final live-fire composition proofs not present: ${livefire:-0}"
fi

# --- dependency-direction: contract crate is provider-neutral --------------------------
forbidden='import requests|import httpx|import boto3|import torch|import transformers|import openai|import anthropic|import numpy|import pandas|nexus_connector_sdk'
if grep -rEn --exclude-dir=__pycache__ "$forbidden" python/nexus_microbrain; then
  fail "forbidden provider dependency import in contract crate"
fi

# --- no-placeholder scan ----------------------------------------------------------------
if grep -rEn --exclude-dir=__pycache__ --include='*.py' 'TODO|FIXME|not implemented|placeholder' \
  python/nexus_microbrain tests/microbrain; then
  fail "placeholder marker in owned sources"
fi

# --- ruff lint + format on owned surface --------------------------------------------------
if ! uv run --frozen ruff check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M5 gate: FAIL - ruff check" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! uv run --frozen ruff format --check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M5 gate: FAIL - ruff format check" >&2
  tail -20 "$log" >&2
  exit 1
fi

tail -2 "$log"
echo "EP-041 M5 gate: ok"
