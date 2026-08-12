#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
[ -n "$AZURE_SPEECH_REGION" ] && curl -fsS --max-time 20 -X POST -H "Ocp-Apim-Subscription-Key: $AZURE_SPEECH_KEY" "https://$AZURE_SPEECH_REGION.api.cognitive.microsoft.com/sts/v1.0/issueToken" >/dev/null
