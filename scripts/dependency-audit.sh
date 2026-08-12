#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
python3 scripts/blueprint_validate.py >/dev/null
if [ -f Cargo.lock ]; then cargo audit --deny warnings; cargo deny check; fi
if [ -f pnpm-lock.yaml ]; then pnpm audit --audit-level=high --prod; fi
if [ -f uv.lock ] && [ -f scripts/python_dependency_audit.py ]; then uv run --frozen python scripts/python_dependency_audit.py; fi
if [ -f apps/mobile/pubspec.lock ] && [ -f scripts/flutter_dependency_audit.py ]; then python3 scripts/flutter_dependency_audit.py; fi
echo "dependency audit: ok"
