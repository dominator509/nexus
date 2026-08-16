#!/usr/bin/env sh
# LF-028 shared-room private response (EP-021 M5 live-fire).
#
# Proves with the real stack that a sensitive response requested in an
# occupied room is routed privately instead of being spoken aloud:
# real Kokoro synthesis of the would-be response, real AudioPrivacyPolicy
# shared-room state (SPEC-012 behavior 9), real router decision, and a
# private-zone control. Machine-readable evidence is written under
# .agent/state/evidence/.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

# Resolve a python3 with stdlib (interpreter-probe precedent; the mise
# shim under scripts/env.sh may differ, so probe explicitly).
_py=""
for _cand in /root/hermes-env/bin/python3 /usr/bin/python3 python3; do
  if command -v "$_cand" >/dev/null 2>&1; then
    _py="$_cand"
    break
  fi
done
[ -n "$_py" ] || { echo "LF-028: FAIL - no python3" >&2; exit 1; }

"$_py" benchmarks/voice/lf028_shared_room_private.py
echo "LF-028: ok"
