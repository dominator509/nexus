#!/usr/bin/env sh
# EP-042 M4 real local installer (SPEC-016, SPEC-024).
#
# Performs a REAL transactional install into an isolated install root:
# manifest validation (canonical M1/M2 surface), backup-before-update,
# staged replacement, digest validation, atomic switch, verification.
#
# Usage:
#   installer-install.sh <install-root> <release-id> <install-id> \
#     <manifest.json> <artifacts-dir> <componentId=relPath>[,componentId=relPath...]
#
# All state lives under the caller-provided install root parent:
#   <install-root>.staging, .backup, .quarantine, .journal
# The host nexus tree is never touched (fence G).
set -eu

INSTALL_ROOT="${1:?install root required}"
RELEASE_ID="${2:?release id required}"
INSTALL_ID="${3:?install id required}"
MANIFEST="${4:?manifest required}"
ARTIFACTS="${5:?artifacts dir required}"
COMPONENTS="${6:?components csv required}"

BASE="$(dirname "$INSTALL_ROOT")"
NAME="$(basename "$INSTALL_ROOT")"
STAGING_ROOT="$BASE/$NAME.staging"
BACKUP_ROOT="$BASE/$NAME.backup"
QUARANTINE_ROOT="$BASE/$NAME.quarantine"
JOURNAL_ROOT="$BASE/$NAME.journal"

rm -rf "$STAGING_ROOT" "$BACKUP_ROOT" "$QUARANTINE_ROOT" "$JOURNAL_ROOT"
mkdir -p "$STAGING_ROOT" "$BACKUP_ROOT" "$QUARANTINE_ROOT" "$JOURNAL_ROOT"

node --experimental-transform-types --experimental-loader "$(cd "$(dirname "$0")" && pwd)/ts-resolve-loader.mjs" \
  "$(cd "$(dirname "$0")/.." && pwd)/src/cli.ts" install \
  --install-root "$INSTALL_ROOT" \
  --staging-root "$STAGING_ROOT" \
  --backup-root "$BACKUP_ROOT" \
  --quarantine-root "$QUARANTINE_ROOT" \
  --journal-root "$JOURNAL_ROOT" \
  --release "$RELEASE_ID" \
  --install "$INSTALL_ID" \
  --manifest "$MANIFEST" \
  --artifacts "$ARTIFACTS" \
  --components "$COMPONENTS"
