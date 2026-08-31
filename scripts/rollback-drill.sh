#!/usr/bin/env sh
# AUD-086: real rollback drill surface. The old script invoked the
# phantom nexus-cli package. The REAL rollback drill lives at
# scripts/ep043-rollback-drill.sh (SPEC-008): known state A -> bad state
# B -> rollback -> exact A verification -> evidence only after
# verification. This wrapper is the canonical operator entry point and
# delegates to the real drill.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
sh "$REPO_ROOT/scripts/ep043-rollback-drill.sh" "$@"
echo "rollback drill: ok"
