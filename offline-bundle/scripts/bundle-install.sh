#!/usr/bin/env sh
# EP-042 M5 real OFFLINE installation (SPEC-016 behavior 5; ExecPlan
# M5 fence I/J). Installs from a verified LOCAL bundle only - no
# release transport, no S3, no network. The bundle IS the artifact
# source; offline actually means offline.
#
# Usage:
#   bundle-install.sh <bundle-dir> <install-root> <release-id> <install-id> \
#     <componentId=relPath[,componentId=relPath...]> <run-id> <git-commit>
set -eu

BUNDLE_DIR="${1:?bundle dir required}"
INSTALL_ROOT="${2:?install root required}"
RELEASE_ID="${3:?release id required}"
INSTALL_ID="${4:?install id required}"
COMPONENTS="${5:?components csv required}"
RUN_ID="${6:?run id required}"
GIT_COMMIT="${7:?git commit required}"

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
  --bundle-dir "$BUNDLE_DIR" \
  --install-root "$INSTALL_ROOT" \
  --staging-root "$STAGING_ROOT" \
  --backup-root "$BACKUP_ROOT" \
  --quarantine-root "$QUARANTINE_ROOT" \
  --journal-root "$JOURNAL_ROOT" \
  --release "$RELEASE_ID" \
  --install "$INSTALL_ID" \
  --components "$COMPONENTS" \
  --run-id "$RUN_ID" \
  --git-commit "$GIT_COMMIT"
