#!/usr/bin/env sh
# AUD-086: hardware certification - honest fail-closed. The old script
# invoked the phantom target/release/nexusctl binary (no implementation
# exists). The REAL hardware certification surface is the checked-in
# hardware/CERTIFICATION_RESULTS.md rows read by the release-evidence
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

echo "hardware certification: FAIL - no real nexusctl executable exists (phantom removed; AUD-086). Hardware certification is recorded in hardware/CERTIFICATION_RESULTS.md and validated by certification_validate.py." >&2
exit 1
