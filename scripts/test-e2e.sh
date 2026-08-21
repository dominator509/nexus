#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
if sh scripts/stage.sh at-least EP-033; then
  [ -d tests/e2e/web ] || { echo "e2e tests: FAIL - missing web E2E suite" >&2; exit 1; }
  pnpm --filter @nexus/web-e2e test:unit
else
  python3 scripts/blueprint_validate.py >/dev/null
  test "$(sh scripts/graph-next.sh | wc -l | tr -d ' ')" = 1
fi
if sh scripts/stage.sh at-least EP-034; then
  [ -d tests/e2e/mobile ] || { echo "e2e tests: FAIL - missing mobile E2E suite" >&2; exit 1; }
  (cd tests/e2e/mobile && flutter test)
fi
echo "e2e tests: ok"
