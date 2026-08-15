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

# LF-021 model-provider-failover (EP-015 M5; live-fire/REGISTRY.tsv).
#
# Proof: the REAL EP-015 live-fire tests in
# crates/nexus-model-router/tests/lf021.rs (8 tests) exercise the
# production failover surface (DeterministicModelRouter::route_with_failover
# + ProviderFailoverPolicy, config-driven from the canonical
# config/models/router/policy.json failover section) through the REAL
# EP-014 DeepSeekFlashProvider and DeepSeekReflexTransport against real
# controlled HTTP endpoints:
#   - primary baseline success (known-good before any failover);
#   - primary UNAVAILABLE (connection refused) -> secondary failover
#     with trace/budget/schema preservation and the ordered audit chain;
#   - primary TIMEOUT (real 30s read timeout on a silent peer) ->
#     secondary failover (typed eligible failure);
#   - contract-invalid payload is NOT failover-eligible (no provider
#     hopping; fail closed);
#   - secondary failure fails closed (unavailable and contract-invalid),
#     bounded attempts, never a fabricated control object;
#   - security override dominates availability (prohibited secondary
#     never used);
#   - budget carry-forward enforces fail-closed (never reset for
#     failover);
#   - policy denial (R4) never attempts a provider; DisabledMicrobrain
#     is never selected.
#
# This script was rewritten from a stub that delegated to a nonexistent
# `nexus-cli` proof runner (EP-006/EP-008 precedent: LF-017.sh/LF-003.sh).
# EP-015 owns no CLI (the workspace has no nexus-cli crate); the REAL
# proof is the committed Rust test suite below - no mocks, no stubs.
#
# External provider certification boundary: no external DeepSeek account
# is required by this node and no DeepSeek credential is present in the
# environment. The production adapter + real transport are exercised
# against real controlled HTTP endpoints; external DeepSeek/secondary
# vendor certification is NOT ASSERTED.

log="/tmp/lf021-cargo.log"
: > "$log"

cargo test --locked -p nexus-model-router --test lf021 >>"$log" 2>&1 || {
  echo "LF-021: FAIL - live-fire cargo test run failed" >&2
  tail -40 "$log" >&2
  exit 1
}

# Vacuity guard: exactly the committed live-fire tests must run (cargo
# exits 0 even when the filter matches nothing - the EP-001 gate-masking
# class).
if ! grep -q 'test result: ok. 8 passed' "$log"; then
  echo "LF-021: FAIL - expected 8 passing live-fire tests (vacuity guard)" >&2
  tail -20 "$log" >&2
  exit 1
fi

evidence=".agent/state/evidence/LF-021-ep015-m5.md"
if [ ! -f "$evidence" ]; then
  {
    echo "# LF-021 Model Provider Failover (EP-015 M5)"
    echo
    echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "Node: EP-015"
    echo "Command: sh scripts/live-fire/LF-021.sh"
    echo
    echo "## Real proof (no mocks, no miniature router)"
    echo "- crates/nexus-model-router/tests/lf021.rs (8 tests) exercises the"
    echo "  production failover surface through the REAL EP-014"
    echo "  DeepSeekFlashProvider and DeepSeekReflexTransport (pinned ureq)"
    echo "  against real controlled HTTP endpoints."
    echo "- DeterministicModelRouter::route_with_failover + ProviderFailoverPolicy"
    echo "  (config-driven from config/models/router/policy.json failover section)."
    echo
    echo "## Primary baseline"
    echo "PASS"
    echo
    echo "## Primary attempted before failover"
    echo "PASS (real transport attempt; connection-refused -> UNAVAILABLE,"
    echo "silent peer -> TIMEOUT/UNAVAILABLE)"
    echo
    echo "## Primary failure type"
    echo "UNAVAILABLE (connection refused); TIMEOUT/UNAVAILABLE (real 30s read timeout)"
    echo
    echo "## Failover policy"
    echo "PASS (only UNAVAILABLE/TIMEOUT are failover-eligible; typed lock)"
    echo
    echo "## Configured secondary selected"
    echo "PASS (production DeepSeekFlashProvider adapter instance at a real"
    echo "isolated HTTP secondary endpoint; instance label"
    echo "deepseek-v4-flash-secondary)"
    echo
    echo "## Secondary real transport"
    echo "PASS (real HTTP transport to the secondary endpoint)"
    echo
    echo "## Final NexusControlObject validation"
    echo "PASS (canonical NexusControlObjectValidator; schema_version 1.0.0)"
    echo
    echo "## Schema preserved"
    echo "PASS (final object retains schema_version 1.0.0; malformed secondary"
    echo "response fails closed)"
    echo
    echo "## Trace ID preserved"
    echo "PASS (one logical correlation id c-lf021 across decision, primary"
    echo "attempt, failure audit, failover decision, secondary attempt, final"
    echo "result, and every audit record)"
    echo
    echo "## Budget preserved / not reset"
    echo "PASS (primary attempt consumes 100 milli-cost + 100 ms per policy;"
    echo "secondary receives the remaining 900, never a fresh cap; final route"
    echo "within max_provider_attempts=2)"
    echo
    echo "## Bounded attempt count"
    echo "PASS (2 attempts max; no provider cycling)"
    echo
    echo "## Security override"
    echo "PASS (SECRET privacy prohibits a CHEAP_API secondary; fail closed"
    echo "without using the prohibited provider)"
    echo
    echo "## Double-provider failure fail-closed"
    echo "PASS (primary + secondary unavailable/invalid -> typed failure;"
    echo "no fabricated control object)"
    echo
    echo "## Audit chain"
    echo "PASS (ordered RouteAuditRecord stages: PRIMARY_SELECTED,"
    echo "PRIMARY_ATTEMPTED, PRIMARY_FAILED:<typed>, FAILOVER_ELIGIBLE,"
    echo "SECONDARY_SELECTED, SECONDARY_ATTEMPTED, SECONDARY_VALIDATED,"
    echo "ROUTE_COMPLETED; FAILED_CLOSED on fail-closed paths)"
    echo
    echo "## Credential/prompt redaction"
    echo "PASS (audit serialization contains no credential, no prompt body,"
    echo "no feature domain)"
    echo
    echo "## External DeepSeek provider"
    echo "NOT ASSERTED (no DeepSeek credential in environment; transport"
    echo "exercised against real controlled endpoints)"
    echo
    echo "## External secondary vendor"
    echo "NOT ASSERTED (registry preferred secondary is the bifrost gateway,"
    echo "not implemented; production adapter used at a controlled endpoint)"
    echo
    echo "## Microbrain status"
    echo "NOT PROMOTED (DisabledMicrobrain is the V1 default; never selected)"
    echo
    echo "## Model output authority"
    echo "NONE (a valid final control object grants no authorization, no"
    echo "capability grant, no approval, and cannot override OPA/OpenFGA)"
    echo
    echo "## Observed sentinel"
    echo "LF-021: ok"
  } > "$evidence"
fi

echo "LF-021: ok"
