#!/usr/bin/env sh
# 6LAYER deterministic scheduler.
set -eu
# Generation 2 (remediation): the ledger-derived DONE path is never authority
# (AUD-085 root cause). Scheduling delegates to GraphLock V2, which recomputes
# node truth from closure attestations, not NODE_DONE.
if [ -f ".agent/remediation/REMEDIATION_STATE.env" ] && \
   grep -q '^REMEDIATION_GENERATION=2' .agent/remediation/REMEDIATION_STATE.env 2>/dev/null; then
  _self=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
  exec sh "$_self/graph-next-v2.sh" "$@"
fi
GRAPH=".agent/GRAPH.md"
[ -f "$GRAPH" ] || { echo "graph-next.sh: missing $GRAPH" >&2; exit 1; }
tmp=$(mktemp)
status_file=$(mktemp)
trap 'rm -f "$tmp" "$status_file"' EXIT
awk '
  /^GRAPH-TABLE-BEGIN$/ { t=1; next }
  /^GRAPH-TABLE-END$/ { t=0 }
  t && $1=="NODE" { print $2, $4 }
' "$GRAPH" > "$tmp"
[ -s "$tmp" ] || { echo "graph-next.sh: GRAPH-TABLE empty or missing" >&2; exit 1; }
: > "$status_file"
while read -r id deps; do
  st=$(sh scripts/ledger.sh status "$id")
  printf '%s %s %s\n' "$id" "$st" "$deps" >> "$status_file"
done < "$tmp"
blocked=$(awk '$2=="BLOCKED"{print $1; exit}' "$status_file")
if [ -n "$blocked" ]; then echo "BLOCKED $blocked"; exit 0; fi
resume=$(awk '$2=="IN_PROGRESS"{print $1; exit}' "$status_file")
if [ -n "$resume" ]; then echo "RESUME $resume"; exit 0; fi
next=$(awk '
  { st[$1]=$2; ord[NR]=$1; dep[$1]=$3; n=NR }
  END {
    for (i=1; i<=n; i++) {
      id=ord[i]
      if (st[id]=="PENDING") {
        ok=1
        m=split(dep[id], a, ",")
        for (j=1; j<=m; j++) {
          d=a[j]
          if (d!="-" && st[d]!="DONE") { ok=0; break }
        }
        if (ok) { print id; exit }
      }
    }
  }
' "$status_file")
if [ -n "$next" ]; then
  echo "NEXT $next"
else
  undone=$(awk '$2!="DONE"{print $1; exit}' "$status_file")
  if [ -z "$undone" ]; then echo ALL_DONE; else echo "STALL $undone"; fi
fi
