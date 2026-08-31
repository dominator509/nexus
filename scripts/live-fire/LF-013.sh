#!/usr/bin/env sh
# AUD-086: LF-013 live-fire proof now calls the REAL owning-node gate
# instead of the phantom proof-runner/nexus-cli. The authoritative
# live-fire for the fax fabric (EP-027) is the EP-027 M5 gate, which
# drives the real nexus-fax-e2e fixture and writes current-run evidence.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
sh "$REPO_ROOT/scripts/ep027-m5-tests.sh"
echo "LF-013: ok"
