#!/usr/bin/env sh
# LF-024 offline-degraded-operation (EP-020 M5; SPEC-011 req 7;
# ADR-027).
#
# Real proof: with the REAL HA provider stopped (cloud/public-internet
# analog unreachable), command execution fails closed, the bounded
# idempotent offline queue retains authorized commands, low-risk local
# capability is retained offline (deterministic fast path, no model
# call), and reconnect drains the queue through the real path with
# exact-target verification. Evidence JSON asserted by the Python
# driver.
set -eu
export CI=true
export CARGO_TERM_COLOR=never
export NO_COLOR=1

cargo build --locked -p nexus-home-assistant --examples
python3 -m pytest tests/home/test_ep020_livefire.py -k lf024 -q --tb=native
echo "LF-024: ok"
