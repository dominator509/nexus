#!/usr/bin/env sh
# RX-004 closure verify ladder: runs every gate and writes the evidence log.
set -eu
cd /root/nexus
EVID=.agent/remediation/evidence/RX-004
mkdir -p "$EVID"
LOG="$EVID/verify.log"
: > "$LOG"

echo "=== RX-004 build/test/reality regression battery ===" >> "$LOG"
. scripts/env.sh
sh scripts/rx004-build-test-reality-tests.sh >> "$LOG" || { echo "RX-004 verify: FAIL (battery)" >> "$LOG"; exit 1; }

echo "=== RX-004 remediation register ===" >> "$LOG"
bash .agent/remediation/verify-remediation-register.sh >> "$LOG" 2>&1 || { echo "RX-004 verify: FAIL (register)" >> "$LOG"; exit 1; }

echo "=== RX-004 EP-040 M5 gate (AUD-062 three consecutive verifies + AUD-063 real observation) ===" >> "$LOG"
set -a
. /tmp/ep038-m5-battery.env
. /tmp/ep038-verify-gt.env
set +a
sh scripts/ep040-m5-tests.sh >> "$LOG" 2>&1 || { echo "RX-004 verify: FAIL (M5 gate)" >> "$LOG"; exit 1; }

echo "=== RX-004 canonical verify ladder ===" >> "$LOG"
sh scripts/verify.sh >> "$LOG" 2>&1 || { echo "RX-004 verify: FAIL (canonical ladder)" >> "$LOG"; exit 1; }

echo "=== RX-004 expected-files ===" >> "$LOG"
sh scripts/expected-files.sh RX-004 >> "$LOG" 2>&1 || { echo "RX-004 verify: FAIL (expected-files)" >> "$LOG"; exit 1; }

# Scrub generated log to ASCII before pinning as closure proof.
python3 - "$LOG" <<'PY'
import sys
p = sys.argv[1]
data = open(p, encoding="utf-8", errors="replace").read()
clean = "".join(ch if ord(ch) < 128 else "?" for ch in data)
open(p, "w", encoding="ascii").write(clean)
PY

echo "verify: ok" >> "$LOG"
echo "RX-004 verify: ok" >> "$LOG"
echo "RX004_VERIFY_EXIT=0" >> "$LOG"
tail -3 "$LOG"
