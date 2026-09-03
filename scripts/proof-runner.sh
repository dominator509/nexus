#!/usr/bin/env sh
# AUD-086: proof runner - honest fail-closed. The old script invoked the
# phantom target/release/nexusctl binary or the phantom nexus-cli
# package (neither exists in the workspace). Every real live-fire proof
# calls its owning node's REAL gate directly (see scripts/live-fire/LF-*).
# This command fails closed with an explicit message instead of
# referencing a non-existent executable.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

proof="${1:-}"
echo "proof runner: FAIL - no real Nexus CLI executable exists (phantom nexusctl/nexus-cli removed; AUD-086). Live-fire proof ${proof:-} must call its owning node's real gate." >&2
exit 1
