#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
curl -fsS --max-time 20 -u "$BACKBLAZE_KEY_ID:$BACKBLAZE_APPLICATION_KEY" https://api.backblazeb2.com/b2api/v3/b2_authorize_account | grep -q "authorizationToken"
