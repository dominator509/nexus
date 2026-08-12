#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
dispatch=$(sh scripts/graph-next.sh)
[ "$dispatch" = ALL_DONE ] || { echo "production readiness: FAIL - graph status is $dispatch" >&2; exit 1; }
NEXUS_REQUIRE_ALL_PROOFS=1 sh scripts/verify.sh
python3 scripts/certification_validate.py
sh scripts/restore-drill.sh
sh scripts/rollback-drill.sh
[ -f dist/release/RELEASE_MANIFEST.json ] || { echo "production readiness: FAIL - release manifest missing" >&2; exit 1; }
echo "production readiness: ok"
