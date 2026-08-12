#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
[ -f Cargo.toml ] || { echo "migrate: FAIL - workspace absent" >&2; exit 1; }
cargo run --locked -q -p nexus-cli -- database migrate
echo "migrate: ok"
