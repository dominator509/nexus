#!/usr/bin/env sh
# LF-007 conditional-home-workflow (EP-020 M5; SPEC-011; ADR-027).
#
# Real proof: condition-based automations in the REAL Home Assistant
# durable registry - creation, persistence across container restart,
# conditional execution, and conditional cancellation. Evidence JSON
# asserted by the Python driver. Temporal persistence machinery is
# proven by the EP-019 workflow suite; a Temporal-hosted home workflow
# is owned by the Temporal-owning/deployment nodes (recorded boundary).
set -eu
export CI=true
export CARGO_TERM_COLOR=never
export NO_COLOR=1

cargo build --locked -p nexus-home-assistant --examples
python3 -m pytest tests/home/test_ep020_livefire.py -k lf007 -q --tb=native
echo "LF-007: ok"
