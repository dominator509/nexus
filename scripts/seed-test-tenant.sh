#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
cargo run --locked -q -p nexus-cli -- test-data seed --tenant nexus-live-fire --idempotent
echo "seed test tenant: ok"
