#!/usr/bin/env sh
set -eu
# Canonical environment: mise shims PATH + non-interactive exports.
. scripts/env.sh
sh scripts/preflight.sh
sh scripts/clean-shell-check.sh
sh scripts/lint.sh
sh scripts/format-check.sh
sh scripts/typecheck.sh
sh scripts/test-unit.sh
sh scripts/test-integration.sh
sh scripts/test-e2e.sh
sh scripts/build.sh
sh scripts/security-check.sh
sh scripts/dependency-audit.sh
sh scripts/license-gate.sh
sh scripts/reality-gate.sh
sh scripts/smoke-test.sh
sh scripts/live-fire.sh
echo "verify: ok"
