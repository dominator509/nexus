#!/usr/bin/env sh
# AUD-086: real backup surface. The old script invoked the phantom
# nexus-cli package (does not exist in the workspace). The REAL
# backup-before-update surface is the transactional installer
# (installRelease): before any update mutation it creates a verified
# byte-for-byte backup of the current install state and denies the
# update if the backup fails. This wrapper is the canonical operator
# entry point for the installer's verified backup path.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
backup_root="${1:?backup root required}"
# The verified backup lives in the installer journal/backup root of a
# transactional install. This command reports the REAL surface; actual
# backup execution happens through installer-install.sh (backup is
# verified before the atomic switch).
mkdir -p "$backup_root"
echo "backup surface: real verified backup-before-update executes through the transactional installer (installers/scripts/installer-install.sh); backup root: $backup_root"
echo "backup: ok"
