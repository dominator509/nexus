#!/usr/bin/env sh
# GraphLock V2 scheduler - the only scheduling authority in generation 2.
#
# Node truth is recomputed independently via node-status-v2.sh (12 conditions).
# The ledger NODE_DONE event is NEVER accepted as DONE input. A forged ledger
# or tag state cannot cause NEXT, ALL_DONE, or release readiness.
#
# Usage: graph-next-v2.sh [ROOT]
set -eu
. "$(dirname "$0")/v2_common.sh"
ROOT="$(v2_root "${1:-}")"
SELF_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"

DAG="$ROOT/.agent/remediation/REMEDIATION_DAG.txt"
REGISTER="$ROOT/.agent/remediation/AUDIT_FINDINGS.tsv"
[ -f "$DAG" ] || { echo "graph-next-v2: FAIL - missing $DAG" >&2; exit 1; }

# Blocked node takes priority.
blocked=""
while read -r id _ deps; do
  case "$id" in \#*) continue;; esac
  [ "$id" = "RX-000" ] && continue
  if [ -f "$ROOT/.agent/state/closures/$id.json" ]; then
    continue
  fi
  if grep -E "\\| $id \\| NODE_BLOCKED \\|" "$ROOT/.agent/state/LEDGER.md" 2>/dev/null | tail -n 1 | grep -q NODE_BLOCKED; then
    blocked="$id"
    break
  fi
done < "$DAG"

status() {
  if sh "$SELF_DIR/node-status-v2.sh" "$1" "$ROOT" --quiet >/dev/null 2>&1; then
    echo DONE
  elif grep -E "\\| $1 \\| LEASE(_TAKEOVER)? \\|" "$ROOT/.agent/state/LEDGER.md" 2>/dev/null | tail -n 1 | grep -q "| $1 |"; then
    echo IN_PROGRESS
  else
    echo PENDING
  fi
}

if [ -n "$blocked" ]; then
  echo "BLOCKED $blocked"
  exit 0
fi

# Leased-but-unclosed node resumes first.
while read -r id _ deps; do
  case "$id" in \#*) continue;; esac
  st=$(status "$id")
  if [ "$st" = "IN_PROGRESS" ]; then
    echo "RESUME $id"
    exit 0
  fi
done < "$DAG"

# First schedulable pending node (all V2-DONE deps).
while read -r id _ deps; do
  case "$id" in \#*) continue;; esac
  st=$(status "$id")
  [ "$st" = "PENDING" ] || continue
  ok=1
  for d in $(echo "$deps" | tr ',' ' '); do
    [ "$d" = "-" ] && continue
    ds=$(status "$d")
    [ "$ds" = "DONE" ] || { ok=0; break; }
  done
  if [ "$ok" -eq 1 ]; then
    # RX-020 gate: no P0/P1 finding may remain open.
    if [ "$id" = "RX-020" ]; then
      open_p1=$(python3 - "$REGISTER" <<'PY'
import csv, sys
n = 0
with open(sys.argv[1], newline="") as fh:
    for row in csv.DictReader(fh, delimiter="\t"):
        if row["severity"] in ("P0", "P1") and row["status"] != "VERIFIED_FIXED":
            n += 1
print(n)
PY
      )
      if [ "$open_p1" != "0" ]; then
        echo "BLOCKED RX-020 (P0/P1 findings not verified fixed: $open_p1)"
        exit 0
      fi
    fi
    echo "NEXT $id"
    exit 0
  fi
done < "$DAG"

undone=""
while read -r id _ deps; do
  case "$id" in \#*) continue;; esac
  st=$(status "$id")
  [ "$st" = "DONE" ] || { undone="$id"; break; }
done < "$DAG"
if [ -z "$undone" ]; then
  echo "ALL_DONE_V2"
else
  echo "STALL $undone"
fi
