#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
python3 scripts/blueprint_validate.py >/dev/null
if [ -f Cargo.toml ] && find crates -path '*/tests/*.rs' -type f 2>/dev/null | grep -q .; then cargo test --workspace --tests --locked; fi
if [ -f pnpm-lock.yaml ]; then pnpm -r test:integration; fi
if [ -f pyproject.toml ] && [ -d tests/integration ]; then uv run --frozen pytest tests/integration -q; fi
echo "integration tests: ok"
