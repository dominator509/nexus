#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
[ ! -f .env ] || git ls-files --error-unmatch .env >/dev/null 2>&1 && { echo "security check: FAIL - .env is tracked" >&2; exit 1; } || true
patterns='AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----|(^|[^A-Za-z0-9_-])sk-[A-Za-z0-9_-]{24,}|ghp_[A-Za-z0-9]{30,}|xox[baprs]-[A-Za-z0-9-]{10,}|AGE-SECRET-KEY-1[A-Z0-9]{20,}'
tracked=$(git ls-files 2>/dev/null || true)
if [ -n "$tracked" ]; then
  hits=$(printf '%s
' "$tracked" | xargs grep -nE "$patterns" 2>/dev/null || true)
  [ -z "$hits" ] || { printf '%s
' "$hits"; echo "security check: FAIL - secret pattern" >&2; exit 1; }
fi
if [ -f Cargo.lock ]; then cargo audit --deny warnings; fi
if [ -f pnpm-lock.yaml ]; then pnpm audit --audit-level=high --prod; fi
if [ -f uv.lock ] && [ -f scripts/python_security_audit.py ]; then uv run --frozen python scripts/python_security_audit.py; fi
sh scripts/license-gate.sh >/dev/null
sh scripts/reality-gate.sh >/dev/null
echo "security check: ok"
