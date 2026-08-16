#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

# LF-018 skill-install-and-run (EP-018 M5; SPEC-010 behaviors 6-8;
# ADR-025; LIVE_FIRE_PROOFS.md).
#
# Proof: inspect, scan, approve, sign, install, discover, execute, and
# roll back a skill without granting undeclared capabilities. The REAL
# EP-018 production composition is exercised end to end:
#   - real skill bundle written on disk (tests/skills/fixtures/
#     livefire-transform.sh payload, CONTROLLED_TEST_FIXTURE);
#   - real SkillBundleLoader (filesystem I/O) + real SHA-256 content
#     hash + manifest validation;
#   - REAL ring Ed25519 signature verification (fresh keypair signs the
#     canonical identity digest; tampered / wrong-signer / bad
#     signature all FAIL);
#   - proposal -> frozen-eval evaluation -> distinct human approval
#     (a model/agent cannot self-approve);
#   - installation through the real durable JSON registry;
#   - dependency composition boundary (present resolves, missing fails
#     closed, effective authority is the intersection);
#   - resolve_for_execution (fail closed);
#   - REAL subprocess execution boundary (SkillExecutor spawns the
#     payload with a scrubbed environment, grants only declared
#     permissions, WRITE at runtime is denied with exit 3);
#   - revoke -> execution denied afterward, durable across reload.
#
# The payload is CONTROLLED_TEST_FIXTURE: it is a real deterministic
# executable (input -> bounded transformation -> output artifact), not
# an external provider. Skill execution is REAL_INTERNAL_PROCESS;
# OS-level sandbox isolation and external/public skill registry
# certification are DEFERRED and recorded in the certification registry.

cargo test --locked -p nexus-skills --test lf018
echo "LF-018: ok"
