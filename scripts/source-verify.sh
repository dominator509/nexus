#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
fail() { echo "source verify: FAIL - $1" >&2; exit 1; }
[ -f references/SOURCE_VERIFICATION.json ] || fail "missing references/SOURCE_VERIFICATION.json"
[ -f references/source-evidence-raw.json ] || fail "missing references/source-evidence-raw.json"
python3 - <<'PY' || fail "verification logic"
import json, sys
from pathlib import Path
records = json.loads(Path("references/SOURCE_VERIFICATION.json").read_text())
unverified = [r["component"] for r in records if r.get("decision_status") == "UNVERIFIED"]
if unverified:
    print(f"unverified sources: {unverified}", file=sys.stderr)
    sys.exit(1)
required = {"component","url","authoritative_owner","version","license","retrieval_date","decision_status"}
for r in records:
    missing = required - set(r)
    if missing:
        print(f"{r['component']} missing {sorted(missing)}", file=sys.stderr)
        sys.exit(1)
    if not r["url"].startswith(("https://","http://")):
        print(f"{r['component']} bad url", file=sys.stderr)
        sys.exit(1)
print(f"source verify: {len(records)} records verified")
PY
echo "source verify: ok"
