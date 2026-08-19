#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
mode="${1:-verify}"
case "$mode" in
  M1) python3 scripts/node-artifact-check.py EP-027 M1 && sh scripts/ep027-m1-tests.sh ;;
  M2) python3 scripts/node-artifact-check.py EP-027 M2 && sh scripts/ep027-m2-tests.sh ;;
  M3) python3 scripts/node-artifact-check.py EP-027 M3 && sh scripts/ep027-m3-tests.sh ;;
  M4) python3 scripts/node-artifact-check.py EP-027 M4 && sh scripts/ep027-m4-tests.sh ;;
  M5|verify)
      python3 scripts/node-artifact-check.py EP-027 M5 && \
      sh scripts/ep027-m5-tests.sh && \
      sh scripts/live-fire/LF-030.sh
      ;;
  *) echo "EP-027: FAIL - unknown mode $mode" >&2; exit 2;;
esac
rc=$?
if [ "$rc" -eq 0 ]; then
  echo "EP-027 $mode: ok"
fi
exit "$rc"
