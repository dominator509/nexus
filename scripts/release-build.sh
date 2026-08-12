#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
sh scripts/verify.sh
cargo run --locked -q -p nexus-cli -- release build --output dist/release --sbom --sign
[ -f dist/release/RELEASE_MANIFEST.json ] || { echo "release build: FAIL - manifest absent" >&2; exit 1; }
echo "release build: ok"
