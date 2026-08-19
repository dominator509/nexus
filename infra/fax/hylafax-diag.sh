#!/usr/bin/env sh
# EP-027 M4 operations diagnostic + bounded recovery for the HylaFAX
# controlled fixture (SPEC-014; M4 CHANGE: infra/fax/).
#
# Usage:
#   sh infra/fax/hylafax-diag.sh diagnose   - health/diagnostic summary
#   sh infra/fax/hylafax-diag.sh recover    - bounded recovery: restart
#                                             hfaxd if not healthy
#
# The diagnostic NEVER prints credentials or document content.
set -eu

FIXTURE="nexus-hylafax-fixture"
HF_PORT="${HYLAFAX_PORT:-4559}"
HF_HOST="${HYLAFAX_HOST:-172.17.0.3}"
MODE="${1:-diagnose}"

hf_pid() {
  docker exec "$FIXTURE" sh -c 'pgrep -x hfaxd 2>/dev/null | head -1' 2>/dev/null || true
}
faxq_pid() {
  docker exec "$FIXTURE" sh -c 'pgrep -x faxq 2>/dev/null | head -1' 2>/dev/null || true
}
hf_reachable() {
  docker exec "$FIXTURE" python3 -c "
import socket
s = socket.socket(); s.settimeout(3)
try:
    s.connect(('$HF_HOST', $HF_PORT)); s.close()
except Exception:
    raise SystemExit(1)
" 2>/dev/null
}
hf_version() {
  docker exec "$FIXTURE" python3 -c "
import socket
s = socket.socket(); s.settimeout(3)
s.connect(('$HF_HOST', $HF_PORT))
print(s.recv(256).decode('latin1').strip())
s.close()
" 2>/dev/null || echo "unreachable"
}

case "$MODE" in
  diagnose)
    echo "hylafax diag: container=$(docker inspect "$FIXTURE" 2>/dev/null | python3 -c 'import json,sys; print("up" if json.load(sys.stdin)[0]["State"]["Running"] else "down")' 2>/dev/null || echo missing)"
    echo "hylafax diag: hfaxd_pid=$(hf_pid || echo none)"
    echo "hylafax diag: faxq_pid=$(faxq_pid || echo none)"
    echo "hylafax diag: greeting=$(hf_version)"
    echo "hylafax diag: sendq_jobs=$(docker exec "$FIXTURE" sh -c 'ls /var/spool/hylafax/sendq/ 2>/dev/null | grep -c "^q" || true' 2>/dev/null || echo 0)"
    echo "hylafax diag: docq_docs=$(docker exec "$FIXTURE" sh -c 'ls /var/spool/hylafax/docq/ 2>/dev/null | wc -l' 2>/dev/null || echo 0)"
    ;;
  recover)
    if hf_reachable && [ -n "$(hf_pid)" ]; then
      echo "hylafax recover: already healthy"
      exit 0
    fi
    echo "hylafax recover: restarting hfaxd (bounded)"
    docker exec "$FIXTURE" sh -c "pkill -x hfaxd 2>/dev/null || true; sleep 1; nohup /usr/sbin/hfaxd -i $HF_PORT >/dev/null 2>&1 &"
    i=0
    until hf_reachable; do
      i=$((i + 1))
      [ "$i" -ge 30 ] && { echo "hylafax recover: FAIL - hfaxd not reachable after restart" >&2; exit 1; }
      sleep 1
    done
    echo "hylafax recover: ok ($(hf_version))"
    ;;
  *)
    echo "hylafax diag: unknown mode $MODE (diagnose|recover)" >&2
    exit 2
    ;;
esac
