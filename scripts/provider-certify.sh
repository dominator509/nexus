#!/usr/bin/env sh
# AUD-086: provider certification - honest fail-closed. The old script
# invoked the phantom target/release/nexusctl binary (no implementation
# exists). The REAL provider certification surface is the checked-in
# provider-certification/RESULTS.md rows read by the release-evidence
# CLI (certification-rows) and validated by scripts/certification_validate.py.
# This command fails closed with an explicit message instead of
# referencing a non-existent executable.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

provider="${1:-}"
echo "provider certification: FAIL - no real nexusctl executable exists (phantom removed; AUD-086). Provider ${provider:-} certification is recorded in provider-certification/RESULTS.md and validated by certification_validate.py." >&2
exit 1
