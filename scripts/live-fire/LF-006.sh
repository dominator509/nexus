#!/usr/bin/env sh
# LF-006 deterministic-home-control (EP-020 M5; SPEC-011; ADR-027).
#
# Real proof: the production nexus-home-assistant adapter drives the
# REAL pinned Home Assistant container. Evidence JSON asserted by the
# Python driver. No model call occurs: the proof process never
# constructs a model provider and the fast-path decision is
# deterministic (EXECUTE_LOCALLY) from policy + twin registry alone.
set -eu
export CI=true
export CARGO_TERM_COLOR=never
export NO_COLOR=1

# Resolve a python3 that has pytest + websocket-client (EP-011 sidecar
# precedent: `python3` runs repo test fixtures). Under scripts/env.sh
# (sourced by node-verify.sh) the mise shim python3 shadows PATH and
# lacks pytest, so probe explicitly instead of trusting PATH (EP-001
# gate-masking class; fail closed if none resolves).
_py=""
for _cand in /root/hermes-env/bin/python3 /usr/bin/python3 python3; do
  if command -v "$_cand" >/dev/null 2>&1 && "$_cand" -c 'import pytest, websocket' >/dev/null 2>&1; then
    _py="$_cand"
    break
  fi
done
[ -n "$_py" ] || { echo "LF-006: FAIL - no python3 with pytest+websocket-client" >&2; exit 1; }

cargo build --locked -p nexus-home-assistant --examples
"$_py" -m pytest tests/home/test_ep020_livefire.py -k lf006 -q --tb=native
echo "LF-006: ok"
