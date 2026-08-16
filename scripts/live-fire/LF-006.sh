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

cargo build --locked -p nexus-home-assistant --examples
python3 -m pytest tests/home/test_ep020_livefire.py -k lf006 -q --tb=native
echo "LF-006: ok"
