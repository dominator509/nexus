#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
curl -fsS --max-time 20 "https://api.phaxio.com/v2/account/status?api_key=$PHAXIO_API_KEY&api_secret=$PHAXIO_API_SECRET" | grep -q "success"
