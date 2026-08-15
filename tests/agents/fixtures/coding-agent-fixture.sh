#!/usr/bin/env sh
# CONTROLLED_TEST_FIXTURE: simulated coding-agent harness for the
# EP-017 M5 / LF-016 live-fire proof.
#
# This script is a REAL executable spawned by the production
# ProcessRunner transport. It simulates a coding agent's observable
# CLI surface so the LF-016 proof exercises the real process boundary
# (spawn, stdin/stdout, exit status, cancellation) without claiming a
# real external provider (Codex/Claude Code are NOT installed here;
# external provider certification is deferred and recorded as such).
#
# Usage: coding-agent-fixture.sh <COMMAND_KIND> [args...]
#   START           print "started", exit 0
#   MESSAGE         print "ack <text>", exit 0
#   CANCEL          print "cancelled", exit 0
#   ARTIFACTS       print "artifact: fixture.patch sha256:<hash>", exit 0
#   TESTS           print "tests: 12 passed", exit 0
#   REVIEW          print "review: APPROVE", exit 0
#   FAIL            exit 3 (forced failure)
#   KILL            exit 137 (forced signal-style death)
#   MALFORMED       print "garbage not json", exit 0
#   SLEEP           sleep 5 then exit 0 (forced timeout)
set -eu

kind="${1:-START}"
shift || true

case "$kind" in
  START)
    # A brief requesting failure forces the fixture to exit non-zero,
    # proving the orchestrator fails closed through the real process.
    case "$*" in
      *FAIL*) exit 3 ;;
    esac
    echo "started" ;;
  MESSAGE)
    case "$*" in
      *FAIL*) exit 3 ;;
    esac
    echo "ack $*" ;;
  CANCEL) echo "cancelled" ;;
  ARTIFACTS) echo "artifact: fixture.patch sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" ;;
  TESTS) echo "tests: 12 passed" ;;
  REVIEW) echo "review: APPROVE" ;;
  FAIL) exit 3 ;;
  KILL) exit 137 ;;
  MALFORMED) echo "garbage not json" ;;
  SLEEP) sleep 5; echo "woke" ;;
  *) echo "unknown command: $kind" >&2; exit 2 ;;
esac
exit 0
