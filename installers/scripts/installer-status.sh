#!/usr/bin/env sh
# EP-042 M4 installer journal status (SPEC-016; operations diagnostic).
#
# Usage:
#   installer-status.sh <install-root> <release-id> <install-id>
set -eu

INSTALL_ROOT="${1:?install root required}"
RELEASE_ID="${2:?release id required}"
INSTALL_ID="${3:?install id required}"

BASE="$(dirname "$INSTALL_ROOT")"
NAME="$(basename "$INSTALL_ROOT")"
JOURNAL_ROOT="$BASE/$NAME.journal"

node --experimental-transform-types --experimental-loader "$(cd "$(dirname "$0")" && pwd)/ts-resolve-loader.mjs" \
  "$(cd "$(dirname "$0")/.." && pwd)/src/cli.ts" status \
  --journal-root "$JOURNAL_ROOT" \
  --release "$RELEASE_ID" \
  --install "$INSTALL_ID"
