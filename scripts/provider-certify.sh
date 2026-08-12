#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
provider="${1:?provider}"
profile="${2:?profile}"
if [ -x target/release/nexusctl ]; then
  target/release/nexusctl provider certify "$provider" --profile "$profile" --evidence-dir .agent/state/evidence
else
  echo "provider certification: FAIL - nexusctl unavailable" >&2
  exit 1
fi
echo "provider certification: ok"
