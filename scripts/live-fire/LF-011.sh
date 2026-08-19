#!/usr/bin/env sh
# LF-011 email-lifecycle live-fire (EP-026 M5).
#
# REAL proof (replaces the pre-created proof-runner placeholder): the
# full email lifecycle - receive, search, summarize, draft, approve,
# send, verify - through the REAL production EmailProvider adapter
# (nexus-imap-smtp) over REAL sockets against the certified controlled
# mail provider (GreenMail 2.1.0, pinned digest). Owned by
# scripts/ep026-m5-tests.sh (the M5 gate), which provisions the real
# fixture, runs the lifecycle suite, verifies current-run machine-
# readable evidence under .agent/state/evidence/, tears the fixture
# down, and audits zero-orphan state; this wrapper records the
# canonical sentinel.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep026-m5-tests.sh
echo "LF-011: ok"
