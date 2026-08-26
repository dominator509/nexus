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
  M1) sh scripts/ep043-m1-tests.sh ;;
  M2) sh scripts/ep043-m2-tests.sh ;;
  M3) sh scripts/ep043-m3-tests.sh ;;
  M4) sh scripts/ep043-m4-tests.sh ;;
  M5) sh scripts/ep043-m5-tests.sh ;;
  verify)
      # Gate-composition guard (same defect class as EP-042 M5): the
      # canonical node verify runs verify.sh twice; LF-029 (EP-044's
      # live-fire proof) starts the control plane, asserts it, and shuts
      # it down gracefully. When EP-044 is at-least the runtime smoke is
      # mandatory, so the second verify.sh would fail closed with the
      # plane down. Provision the runtime through canonical local-start
      # when unhealthy, then re-smoke. Fails closed if it cannot be
      # brought healthy; this mirrors the fixture-provisioning pattern
      # EP-037/EP-038 gates use for MinIO/GlitchTip.
      if sh scripts/stage.sh at-least EP-044 >/dev/null 2>&1; then
        export NEXUS_SMOKE_URL="${NEXUS_SMOKE_URL:-http://127.0.0.1:8443}"
        if ! sh scripts/smoke/runtime.sh >/dev/null 2>&1; then
          echo "control plane not running - bringing up core profile (canonical local-start)"
          sh scripts/local-start.sh core >/dev/null 2>&1 || true
        fi
        if ! sh scripts/smoke/runtime.sh >/dev/null 2>&1; then
          echo "EP-043: FAIL - control plane not healthy after local-start (restart core before node verify)" >&2
          exit 4
        fi
      fi
      sh scripts/ep043-m5-tests.sh
      sh scripts/verify.sh
      :
      ;;
  *) echo "EP-043: FAIL - unknown mode $mode" >&2; exit 2;;
esac
echo "EP-043 $mode: ok"
