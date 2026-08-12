#!/usr/bin/env sh
set -eu
cmd="${1:-current}"
case "$cmd" in
  current)
    last="EP-000"
    seen=0
    awk '/^GRAPH-TABLE-BEGIN$/{t=1;next} /^GRAPH-TABLE-END$/{t=0} t && $1=="NODE"{print $2}' .agent/GRAPH.md |
    while read -r id; do
      st=$(sh scripts/ledger.sh status "$id")
      if [ "$st" = "DONE" ]; then
        printf '%s\n' "$id"
      fi
    done > .agent/state/.stage.tmp
    if [ -s .agent/state/.stage.tmp ]; then last=$(tail -n 1 .agent/state/.stage.tmp); seen=1; fi
    rm -f .agent/state/.stage.tmp
    if [ "$seen" -eq 0 ]; then echo "STAGE PRE-EP-000"; else echo "STAGE $last"; fi
    ;;
  at-least)
    target="${2:?node id}"
    st=$(sh scripts/ledger.sh status "$target")
    [ "$st" = "DONE" ]
    ;;
  *) echo "usage: stage.sh current|at-least EP-NNN" >&2; exit 2;;
esac
