#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
curl -fsS --max-time 20 -u "$OPNSENSE_API_KEY:$OPNSENSE_API_SECRET" "$OPNSENSE_URL/api/core/system/status" >/dev/null
