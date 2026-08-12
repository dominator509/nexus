#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
path="${NEXUS_BOOTSTRAP_AGE_KEY_FILE:-${NEXUS_EXISTING_SSH_KEY_FILE:-}}"; [ -n "$path" ] && [ -f "$path" ] && [ ! -r "$path" ] && exit 1; [ -n "$path" ] && [ -f "$path" ]
