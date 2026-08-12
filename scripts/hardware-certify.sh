#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
target="${1:?hardware target}"
if [ -x target/release/nexusctl ]; then
  target/release/nexusctl hardware certify "$target" --inventory hardware/LAB_INVENTORY.yaml --evidence-dir .agent/state/evidence
else
  echo "hardware certification: FAIL - nexusctl unavailable" >&2
  exit 1
fi
echo "hardware certification: ok"
