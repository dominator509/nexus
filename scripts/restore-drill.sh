#!/usr/bin/env sh
# AUD-086: real restore drill surface. The old script invoked the
# phantom nexus-cli package. The REAL restore drill is the canonical
# rollback drill (scripts/ep043-rollback-drill.sh) which restores the
# exact prior committed state A and verifies restoration byte-for-byte.
# This wrapper delegates to the real drill.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
sh "$REPO_ROOT/scripts/ep043-rollback-drill.sh" "$@"
echo "restore drill: ok"
