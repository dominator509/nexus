#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
curl -fsS --max-time 20 -X POST https://auth.contabo.com/auth/realms/contabo/protocol/openid-connect/token -H "Content-Type: application/x-www-form-urlencoded" --data-urlencode "client_id=$CONTABO_CLIENT_ID" --data-urlencode "client_secret=$CONTABO_CLIENT_SECRET" --data-urlencode "username=$CONTABO_API_USER" --data-urlencode "password=$CONTABO_API_PASSWORD" --data-urlencode "grant_type=password" | grep -q "access_token"
