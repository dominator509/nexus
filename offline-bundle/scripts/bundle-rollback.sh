#!/usr/bin/env sh
# EP-042 M5 real rollback drill (ExecPlan M5 fence M). Restores the
# exact prior bytes through the M4 rollback surface and writes the
# rollback receipt ONLY after restoration is verified.
#
# Usage:
#   bundle-rollback.sh <install-root> <release-id> <install-id> \
#     <backup-digest> <absPath=priorBytes[,absPath=priorBytes...]> \
#     <run-id> <git-commit>
set -eu

INSTALL_ROOT="${1:?install root required}"
RELEASE_ID="${2:?release id required}"
INSTALL_ID="${3:?install id required}"
BACKUP_DIGEST="${4:?backup digest required}"
EXPECTED_PRIOR="${5:?expected prior bytes csv required}"
RUN_ID="${6:?run id required}"
GIT_COMMIT="${7:?git commit required}"

node --experimental-transform-types --experimental-loader "$(cd "$(dirname "$0")" && pwd)/ts-resolve-loader.mjs" \
  "$(cd "$(dirname "$0")/.." && pwd)/src/cli.ts" rollback-drill \
  --install-root "$INSTALL_ROOT" \
  --staging-root "$INSTALL_ROOT.staging" \
  --backup-root "$INSTALL_ROOT.backup" \
  --quarantine-root "$INSTALL_ROOT.quarantine" \
  --journal-root "$INSTALL_ROOT.journal" \
  --release "$RELEASE_ID" \
  --install "$INSTALL_ID" \
  --backup-digest "$BACKUP_DIGEST" \
  --expected-prior "$EXPECTED_PRIOR" \
  --run-id "$RUN_ID" \
  --git-commit "$GIT_COMMIT"
