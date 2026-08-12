#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
cargo run --locked -q -p nexus-cli -- backup drill --scratch --evidence-dir .agent/state/evidence
echo "restore drill: ok"
