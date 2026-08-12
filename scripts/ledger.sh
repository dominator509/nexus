#!/usr/bin/env sh
# 6LAYER ledger helper. Append-only event writer and status reader.
set -eu
LEDGER=".agent/state/LEDGER.md"
[ -f "$LEDGER" ] || { echo "ledger.sh: missing $LEDGER" >&2; exit 1; }
cmd="${1:-}"
[ -n "$cmd" ] && shift
case "$cmd" in
  append)
    agent="${1:?agent id}"; node="${2:?node id or -}"; event="${3:?event}"; shift 3
    detail="${*:-}"
    case "$detail" in *" | "*) echo "ledger.sh: detail contains reserved delimiter" >&2; exit 1;; esac
    ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    printf '%s | %s | %s | %s | %s\n' "$ts" "$agent" "$node" "$event" "$detail" >> "$LEDGER"
    ;;
  status)
    node="${1:?node id}"
    line=$(grep -E "\| $node \| (NODE_DONE|NODE_BLOCKED|LEASE_RELEASE|LEASE|LEASE_TAKEOVER) \|" "$LEDGER" | tail -n 1 || true)
    case "$line" in
      *"| NODE_DONE |"*) echo DONE ;;
      *"| NODE_BLOCKED |"*) echo BLOCKED ;;
      *"| LEASE_RELEASE |"*) echo PENDING ;;
      *"| LEASE |"*|*"| LEASE_TAKEOVER |"*) echo IN_PROGRESS ;;
      *) echo PENDING ;;
    esac
    ;;
  tail)
    n="${1:-30}"
    tail -n "$n" "$LEDGER"
    ;;
  *)
    echo "usage: ledger.sh append|status|tail" >&2
    exit 2
    ;;
esac
