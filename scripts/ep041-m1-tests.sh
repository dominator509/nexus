#!/usr/bin/env sh
# EP-041 M1 gate: Microbrain contract, vocabulary, and package boundary.
#
# Non-vacuous: material presence, real pytest proof count, anti-masking
# sentinels, dependency-direction proof, no-placeholder scan, ruff lint
# + format on the owned surface. The node M1 branch must call this gate
# (no artifact-check-only masking).
set -eu
. scripts/env.sh
export NO_COLOR=1

log="/tmp/ep041-m1-tests.log"
: > "$log"

fail() {
  echo "EP-041 M1 gate: FAIL - $1" >&2
  exit 1
}

# --- material presence ---------------------------------------------------
for path in \
  python/nexus_microbrain/__init__.py \
  python/nexus_microbrain/errors.py \
  python/nexus_microbrain/models.py \
  python/nexus_microbrain/vocabulary.py \
  tests/microbrain/conftest.py \
  tests/microbrain/test_ep041_m1_contracts.py \
  microbrain/datasets/README.md \
  microbrain/evals/README.md \
  microbrain/training/README.md \
  microbrain/artifacts/README.md; do
  [ -f "$path" ] || fail "missing owned path: $path"
done

# --- workspace membership (pyproject wheel package) -----------------------
grep -q 'python/nexus_microbrain' pyproject.toml || fail "package not registered in pyproject.toml"
grep -q 'ep041_unit_\*' pyproject.toml || fail "ep041_unit pytest functions not registered"

# --- anti-masking sentinels ----------------------------------------------
grep -q 'ep041-m1-tests.sh' scripts/nodes/EP-041.sh || fail "node M1 branch not wired to gate"
if grep -q 'node-artifact-check.py EP-041 M1' scripts/nodes/EP-041.sh; then
  fail "node M1 still uses artifact-check masking"
fi

# --- real pytest with vacuity guard --------------------------------------
if ! uv run --frozen pytest tests/microbrain -q --tb=short \
  -o python_functions="ep041_unit_*" >>"$log" 2>&1; then
  echo "EP-041 M1 gate: FAIL - pytest ep041_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi
if ! grep -Eq '^[0-9]+ passed' "$log"; then
  fail "no tests ran (vacuity guard)"
fi
count=$(grep -Eo '^[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 50 ]; then
  fail "too few proofs passed: ${count:-0} (need >= 50)"
fi
if grep -Eq '[1-9][0-9]* failed|[1-9][0-9]* error' "$log"; then
  fail "failures/errors present in pytest output"
fi

# --- dependency-direction: contract crate is provider-neutral -------------
forbidden='import requests|import httpx|import boto3|import torch|import transformers|import openai|import anthropic|import numpy|import pandas|nexus_connector_sdk'
if grep -rEn --exclude-dir=__pycache__ "$forbidden" python/nexus_microbrain; then
  fail "forbidden provider dependency import in contract crate"
fi

# --- no-placeholder scan ---------------------------------------------------
if grep -rEn --exclude-dir=__pycache__ --include='*.py' 'TODO|FIXME|not implemented|placeholder' \
  python/nexus_microbrain tests/microbrain/test_ep041_m1_contracts.py; then
  fail "placeholder marker in owned sources"
fi

# --- ruff lint + format on owned surface -----------------------------------
if ! uv run --frozen ruff check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M1 gate: FAIL - ruff check" >&2
  tail -20 "$log" >&2
  exit 1
fi
if ! uv run --frozen ruff format --check python/nexus_microbrain tests/microbrain >>"$log" 2>&1; then
  echo "EP-041 M1 gate: FAIL - ruff format check" >&2
  tail -20 "$log" >&2
  exit 1
fi

tail -3 "$log"
echo "EP-041 M1 gate: ok"
