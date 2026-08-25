#!/usr/bin/env sh
# EP-042 M4 operations diagnostic + bounded recovery (SPEC-016; ExecPlan
# M4 CONTENT 6 - every new service/provider gets a diagnostic + bounded
# recovery command).
#
# Reads the installer journal (real state transitions), quarantines any
# staged state from an interrupted update, and reports the journal state
# honestly. Recovery is bounded: it never touches foreign roots and
# never claims byte restoration it did not perform.
#
# Usage:
#   installer-recover.sh <install-root> <release-id> <install-id>
set -eu

INSTALL_ROOT="${1:?install root required}"
RELEASE_ID="${2:?release id required}"
INSTALL_ID="${3:?install id required}"

BASE="$(dirname "$INSTALL_ROOT")"
NAME="$(basename "$INSTALL_ROOT")"
STAGING_ROOT="$BASE/$NAME.staging"
BACKUP_ROOT="$BASE/$NAME.backup"
QUARANTINE_ROOT="$BASE/$NAME.quarantine"
JOURNAL_ROOT="$BASE/$NAME.journal"

node --experimental-transform-types --experimental-loader "$(cd "$(dirname "$0")" && pwd)/ts-resolve-loader.mjs" \
  "$(cd "$(dirname "$0")/.." && pwd)/src/cli.ts" recover \
  --install-root "$INSTALL_ROOT" \
  --staging-root "$STAGING_ROOT" \
  --backup-root "$BACKUP_ROOT" \
  --quarantine-root "$QUARANTINE_ROOT" \
  --journal-root "$JOURNAL_ROOT" \
  --release "$RELEASE_ID" \
  --install "$INSTALL_ID"
