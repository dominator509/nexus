#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
case "${1:-}" in
  --dry-run)
    cargo run --locked -q -p nexus-setup-cli -- deploy plan --profile "${NEXUS_RELEASE_PROFILE:-core}" --redact
    echo "deploy dry run: ok"
    ;;
  *)
    echo "deploy: FAIL - production deployment is not authorized; use --dry-run" >&2
    exit 1
    ;;
esac
