#!/usr/bin/env sh
# AUD-086: real migration surface. The old script invoked the phantom
# nexus-cli package (does not exist in the workspace). The REAL
# migrations live in migrations/*.sql; this wrapper applies them to a
# real PostgreSQL target via psql when DATABASE_URL is set, and fails
# closed otherwise. It never references a non-existent executable.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
db_url="${DATABASE_URL:-}"
[ -n "$db_url" ] || { echo "migrate: FAIL - DATABASE_URL not set; migrations are applied to the real PostgreSQL target by an operator" >&2; exit 1; }
command -v psql >/dev/null 2>&1 || { echo "migrate: FAIL - psql not installed" >&2; exit 1; }
for migration in "$REPO_ROOT"/migrations/*.sql; do
  [ -f "$migration" ] || continue
  echo "migrate: applying $(basename "$migration")"
  psql "$db_url" -v ON_ERROR_STOP=1 -f "$migration" >/dev/null
done
echo "migrate: ok"
