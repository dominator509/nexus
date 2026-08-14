#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
mode="${1:-verify}"
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-009 M1 ;;
  M2)
      python3 scripts/node-artifact-check.py EP-009 M2 \
      && cargo test --locked -p nexus-trust ep009_unit \
      && cargo test --locked -p nexus-openbao ep009_unit \
      && uv run --frozen pytest tests/trust -q --tb=native -o python_functions="ep009_integration_*" \
      && uv run --frozen pytest tests/trust -q --tb=native -o python_functions="ep009_failure_*" \
      && sh scripts/ep009-orphan-audit.sh
      ;;
  M3)
      python3 scripts/node-artifact-check.py EP-009 M3 \
      && cargo test --locked -p nexus-trust ep009_unit \
      && cargo test --locked -p nexus-headscale ep009_unit \
      && uv run --frozen pytest tests/trust -q --tb=native -o python_functions="ep009_integration_headscale_*" \
      && sh scripts/ep009-orphan-audit.sh
      ;;
  M4)
      python3 scripts/node-artifact-check.py EP-009 M4 \
      && cargo test --locked -p nexus-trust ep009_unit \
      && cargo test --locked -p nexus-pki ep009_unit \
      && uv run --frozen pytest tests/trust -q --tb=native -o python_functions="ep009_integration_pki_*" \
      && uv run --frozen pytest tests/trust -q --tb=native -o python_functions="ep009_failure_pki_*" \
      && sh scripts/ep009-orphan-audit.sh
      ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-009 M5
      cargo test --locked -p nexus-trust
      :
      ;;
  *) echo "EP-009: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-009 $mode: ok"
