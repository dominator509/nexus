#!/usr/bin/env sh
# AUD-086: seed test tenant - honest fail-closed. The old script invoked
# the phantom nexus-cli package. No real test-data seeding executable
# exists in the workspace, so this command fails closed with an explicit
# message instead of referencing a non-existent package.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

echo "seed test tenant: FAIL - no real test-data seeding executable exists in the workspace (phantom nexus-cli removed; AUD-086)" >&2
exit 1
