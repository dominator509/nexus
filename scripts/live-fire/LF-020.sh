#!/usr/bin/env sh
# LF-020 storage-backend-portability live-fire (EP-037 M5).
#
# Write versioned artifacts, migrate between local and one S3-compatible
# backend, verify hashes and metadata, and remove the old copy only
# after approval. The REAL journey is the EP-037 M5 gate: versioned
# artifacts through the production local ArtifactStore, migrated to a
# REAL digest-pinned MinIO backend through the production storage-s3
# adapter, destination hash/metadata verified, approval-before-delete
# proven (source remains without approval; after approval, source
# absence independently verified, destination intact). Evidence is
# current-run bound (.agent/state/evidence/LF-020-ep037-m5.json - stale
# evidence never satisfies). The historical phantom runner delegation is
# gone.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep037-m5-tests.sh

echo "LF-020: ok"
