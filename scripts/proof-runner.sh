#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
proof="${1:?proof id}"
if [ -x target/release/nexusctl ]; then
  target/release/nexusctl proof run "$proof" --evidence-dir .agent/state/evidence
elif [ -f Cargo.toml ]; then
  cargo run --locked -q -p nexus-cli -- proof run "$proof" --evidence-dir .agent/state/evidence
else
  echo "$proof: FAIL - Nexus CLI is not built" >&2
  exit 1
fi
