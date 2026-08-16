#!/usr/bin/env sh
# CONTROLLED_TEST_FIXTURE: real executable skill payload for the
# EP-018 M5 / LF-018 live-fire proof.
#
# This script is a REAL deterministic skill: it reads one bounded input
# line from stdin and produces a deterministic transformation on stdout
# (input -> bounded transformation -> output artifact). It is copied
# into a REAL skill bundle on disk and installed through the REAL
# registry, then executed through the REAL SkillExecutor subprocess
# boundary.
#
# It demonstrates the permission boundary: the executor passes the
# granted permissions via NEXUS_SKILL_GRANTED_PERMISSIONS and the
# script refuses a WRITE directive unless WRITE was actually granted.
set -eu
IFS= read -r line || line=""
case "$line" in
  WRITE:*) 
    case "${NEXUS_SKILL_GRANTED_PERMISSIONS:-}" in
      *WRITE*) printf 'write-ok:%s\n' "${line#WRITE:}" ;;
      *) echo 'write-denied:not-granted' >&2; exit 3 ;;
    esac
    ;;
  *)
    printf 'transformed:%s\n' "$line"
    ;;
esac
exit 0
