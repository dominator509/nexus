#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
. scripts/env.sh
export NO_COLOR=1

# LF-016 coding-agent-cowork (EP-017 M5; live-fire/REGISTRY.tsv).
#
# Proof: the REAL EP-017 production composition is exercised end to end
# through a REAL subprocess boundary. crates/nexus-harness-adapters/tests/lf016.rs
# (5 tests) drives the production ProcessRunner transport, which spawns
# the REAL executable tests/agents/fixtures/coding-agent-fixture.sh
# (CONTROLLED_TEST_FIXTURE) for every normalized command:
#   - lf016_real_process_full_cowork_chain: objective/task ->
#     capability-based selection -> budget-bound assignment -> REAL
#     harness execution (spawn) -> progress -> artifact exchange ->
#     bounded review -> completion + delegation COMPLETED;
#   - lf016_real_process_cancellation_terminates_owned_process: cancel
#     through the real transport, delegation REVOKED, no orphan;
#   - lf016_real_process_nonzero_exit_fails_closed: real subprocess
#     exits 3 -> typed UNAVAILABLE, task stays ASSIGNED;
#   - lf016_real_process_runner_maps_exit_codes: exit-status mapping
#     (Success / Failure(code)) and output capture on the real
#     transport;
#   - lf016_real_process_missing_executable_fails_closed: spawn failure
#     -> typed UNAVAILABLE.
#
# External provider certification boundary: real Codex / Claude Code /
# Hermes / OpenClaw CLIs are NOT installed in this environment and no
# provider credential is present. This proof does NOT certify an
# external coding-agent provider; the fixture is CONTROLLED_TEST_FIXTURE.
# External provider certification is DEFERRED (recorded in the
# certification registry with its owner).

cargo test --locked -p nexus-harness-adapters --test lf016
echo "LF-016: ok"
