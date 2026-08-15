#!/usr/bin/env sh
# EP-015 M3 frozen routing benchmark (SPEC-009 required test "Router
# policy table"; node contract fallback).
#
# The frozen corpus is the acceptance criterion for learned routing:
# deterministic weighted policy routing is the V1 default and a learned
# router (RouteLLM/LLMRouter) may replace it ONLY after beating this
# frozen benchmark on every case (node contract fallback). The policy
# engine can always override learned routing for security.
#
# Proves:
#   1. The deterministic policy selects the FROZEN expected route for
#      every corpus case (boundary + safety floors).
#   2. A learned proposal that violates a safety floor (SECRET privacy
#      -> CHEAP_API, R4 -> any model route, local-only -> remote) is
#      overridden by policy.
#   3. Replay stability: running the corpus twice produces identical
#      decisions (idempotency).
#
# Deterministic: no network, no credentials, no timestamps. Exits
# nonzero on any violation.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

cd "$(dirname "$0")/../.."

echo "== EP-015 frozen routing benchmark =="

check() {
  name="$1"
  test_name="$2"
  log=$(mktemp)
  cargo test --locked -p nexus-model-router "$test_name" >"$log" 2>&1 || {
    echo "benchmark FAIL: $name could not run" >&2
    rm -f "$log"
    exit 1
  }
  summary=$(grep -E "^cargo test:|test result: ok\. [1-9]" "$log" | tail -n 1)
  echo "$name: $summary"
  case "$summary" in
    *"test result: ok"*|*"passed"*) ;;
    *) echo "benchmark FAIL: $name not proven" >&2; rm -f "$log"; exit 1;;
  esac
  rm -f "$log"
}

# 1. Frozen corpus: every case must select the frozen expected route.
check "frozen-corpus test" ep015_unit_frozen_corpus_routes_match

# 2. Security override: learned proposals violating safety floors are
#    overridden by the policy engine.
check "security-override test" ep015_unit_frozen_corpus_security_override

# 3. Replay stability: identical features -> identical decisions.
check "replay-stability test" ep015_unit_router_is_idempotent

echo "frozen routing benchmark: ok"
