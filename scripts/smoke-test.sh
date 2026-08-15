#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
python3 scripts/blueprint_validate.py >/dev/null
# Runtime smoke ownership (EP-044, owner GraphLock amendment 2026-08-14).
# The runtime smoke activates only once its real prerequisite owner is DONE.
# Before EP-044 is DONE the stage is explicitly not-applicable; it is NOT a
# PASS claim for runtime functionality. At/after EP-044 the smoke is mandatory
# and fails closed when the runtime is absent or unhealthy.
runtime_owner="EP-044"
if sh scripts/stage.sh at-least "$runtime_owner"; then
  sh scripts/smoke/runtime.sh
else
  echo "runtime smoke: not-applicable-before $runtime_owner"
fi
echo "smoke test: ok"
