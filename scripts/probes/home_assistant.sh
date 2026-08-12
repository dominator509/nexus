#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
curl -fsS --max-time 20 -H "Authorization: Bearer $HOME_ASSISTANT_TOKEN" -H "Content-Type: application/json" "$HOME_ASSISTANT_URL/api/" | grep -q "API running"
