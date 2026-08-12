#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
curl -fsS --max-time 20 "https://generativelanguage.googleapis.com/v1beta/models?key=$GOOGLE_AI_API_KEY" | grep -q "models"
