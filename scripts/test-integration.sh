#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
python3 scripts/blueprint_validate.py >/dev/null
if [ -f Cargo.toml ] && find crates -path '*/tests/*.rs' -type f 2>/dev/null | grep -q .; then
  # EP-038 phase-gated exclusion (approved-scope convention): the
  # revoked-token proof requires NEXUS_GLITCHTIP_REVOKED=1 AND a truly
  # revoked token, which cannot coexist with the valid-token live
  # provider tests in one shared battery env. It is proven by the M4
  # gate's dedicated revoked phase (scripts/ep038-m4-tests.sh), which
  # revokes the token in the DB and asserts exactly 1/1. The blanket
  # battery therefore skips only that single phase-gated test; every
  # other fixture-driven test runs here with the battery env.
  cargo test --workspace --tests --locked -- --skip ep038_failure_revoked_token_authorization
fi
if [ -f pnpm-lock.yaml ]; then pnpm -r test:integration; fi
if [ -f pyproject.toml ] && [ -d tests/integration ]; then uv run --frozen pytest tests/integration -q; fi
if [ -f tests/accessibility/mobile/pubspec.yaml ]; then (cd tests/accessibility/mobile && flutter test); fi
echo "integration tests: ok"
