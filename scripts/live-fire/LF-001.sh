#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
# LF-001 one-package-deployment live-fire. The real proof is the EP-035
# M5 gate: one-package bundle built from the current source tree, clean
# ephemeral deployment target, real runtime readiness, real owner
# bootstrap and verification, idempotent replay, and current-run
# evidence. The dangling proof-runner delegation is gone.
sh scripts/ep035-m5-tests.sh
echo "LF-001: ok"
