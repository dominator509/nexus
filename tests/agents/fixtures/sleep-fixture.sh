#!/usr/bin/env sh
# CONTROLLED_TEST_FIXTURE: sleep harness for RX-007 AUD-022.
#
# Sleeps much longer than the invocation deadline the tests configure
# (500 ms) so the ProcessRunner must kill the child and surface
# HarnessExitStatus::Timeout instead of waiting forever.
#
# `exec` is deliberate: the fixture REPLACES itself with `sleep`, so
# the long-running process IS the direct child of ProcessRunner. The
# safe ProcessRunner contract kills the direct child (no unsafe code
# in nexus-harness-adapters); `exec` keeps the test honest by leaving
# no forked grandchild holding the pipes open after the kill.
set -eu
exec sleep 60
