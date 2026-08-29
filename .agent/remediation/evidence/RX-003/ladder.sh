#!/usr/bin/env sh
# RX-003 closure verify ladder: runs every gate and writes the evidence log.
set -eu
cd /root/nexus
EVID=.agent/remediation/evidence/RX-003
mkdir -p "$EVID"
LOG="$EVID/verify.log"
: > "$LOG"

echo "=== RX-003 CI-authority regression battery ===" >> "$LOG"
sh scripts/rx003-ci-authority-tests.sh >> "$LOG" || { echo "RX-003 verify: FAIL (battery)" >> "$LOG"; exit 1; }

echo "=== RX-003 remediation register ===" >> "$LOG"
bash .agent/remediation/verify-remediation-register.sh >> "$LOG" 2>&1 || { echo "RX-003 verify: FAIL (register)" >> "$LOG"; exit 1; }

echo "=== RX-003 canonical verify ladder ===" >> "$LOG"
set -a
. /tmp/ep038-m5-battery.env
. /tmp/ep038-verify-gt.env
set +a
sh scripts/verify.sh >> "$LOG" 2>&1 || { echo "RX-003 verify: FAIL (canonical ladder)" >> "$LOG"; exit 1; }

echo "=== RX-003 expected-files ===" >> "$LOG"
sh scripts/expected-files.sh RX-003 >> "$LOG" 2>&1 || { echo "RX-003 verify: FAIL (expected-files)" >> "$LOG"; exit 1; }

# Scrub generated log to ASCII before pinning as closure proof.
python3 - "$LOG" <<'PY'
import sys
p = sys.argv[1]
data = open(p, encoding="utf-8", errors="replace").read()
clean = "".join(ch if ord(ch) < 128 else "?" for ch in data)
open(p, "w", encoding="ascii").write(clean)
PY

echo "verify: ok" >> "$LOG"
echo "RX-003 verify: ok" >> "$LOG"
echo "RX003_VERIFY_EXIT=0" >> "$LOG"
tail -3 "$LOG"
