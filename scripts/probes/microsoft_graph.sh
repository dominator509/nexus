#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
curl -fsS --max-time 20 "https://login.microsoftonline.com/$MICROSOFT_TENANT_ID/v2.0/.well-known/openid-configuration" | grep -q "token_endpoint"
