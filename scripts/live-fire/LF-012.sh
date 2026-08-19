#!/usr/bin/env sh
# LF-012 governed-phone-call live-fire (EP-025 M5).
#
# REAL proof (replaces the pre-created proof-runner placeholder): a
# real inbound governed call through the REAL pinned Asterisk 22.10.1
# container with REAL ARI control, REAL RTP, REAL whisper.cpp STT, REAL
# Kokoro TTS, production disclosure policy, and independent far-end
# readback. Owned by scripts/ep025-m5-tests.sh (the M5 gate), which
# drives all three scenarios (positive / negative-disclosure / hostile)
# and the production governance suite; this wrapper records the
# canonical sentinel.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep025-m5-tests.sh
echo "LF-012: ok"
