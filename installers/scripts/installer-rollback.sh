#!/usr/bin/env sh
# EP-042 M4 real rollback (SPEC-016 behavior 6; SPEC-024 restore).
#
# Restores the prior install state from the backup root created by
# installer-install.sh, then verifies the restored bytes. A rollback
# receipt is only produced after verified restoration (fence K).
#
# Usage:
#   installer-rollback.sh <install-root> <release-id> <install-id> <backup-digest>
set -eu

INSTALL_ROOT="${1:?install root required}"
RELEASE_ID="${2:?release id required}"
INSTALL_ID="${3:?install id required}"
BACKUP_DIGEST="${4:?backup digest required}"

BASE="$(dirname "$INSTALL_ROOT")"
NAME="$(basename "$INSTALL_ROOT")"
STAGING_ROOT="$BASE/$NAME.staging"
BACKUP_ROOT="$BASE/$NAME.backup"
QUARANTINE_ROOT="$BASE/$NAME.quarantine"
JOURNAL_ROOT="$BASE/$NAME.journal"

node --experimental-transform-types --experimental-loader "$(cd "$(dirname "$0")" && pwd)/ts-resolve-loader.mjs" \
  "$(cd "$(dirname "$0")/.." && pwd)/src/cli.ts" rollback \
  --install-root "$INSTALL_ROOT" \
  --staging-root "$STAGING_ROOT" \
  --backup-root "$BACKUP_ROOT" \
  --quarantine-root "$QUARANTINE_ROOT" \
  --journal-root "$JOURNAL_ROOT" \
  --release "$RELEASE_ID" \
  --install "$INSTALL_ID" \
  --backup-digest "$BACKUP_DIGEST"
