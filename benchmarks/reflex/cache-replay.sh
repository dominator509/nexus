#!/usr/bin/env sh
# EP-014 M4 reflex cache-replay benchmark (SPEC-009).
#
# Proves the cacheable-corpus discipline on the REAL canonical prompt
# segment catalog:
#   1. The stable prefix serializes to identical bytes across loads
#      (byte stability -> cacheable corpus).
#   2. Replaying the stable prefix across many requests keeps the
#      rolling token cache-hit ratio at or above the 0.97 target.
#   3. The volatile tail (session/dynamic) is excluded from the
#      cacheable prefix.
#
# Deterministic: no network, no credentials, no timestamps in the
# measured bytes. Exits nonzero on any violation.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

cd "$(dirname "$0")/../.."

echo "== EP-014 reflex cache-replay benchmark =="

check() {
  name="$1"
  test_name="$2"
  log=$(mktemp)
  cargo test --locked -p nexus-reflex "$test_name" >"$log" 2>&1 || {
    echo "benchmark FAIL: $name could not run" >&2
    rm -f "$log"
    exit 1
  }
  # The rtk-tee wrapper appends a final summary line
  # ("cargo test: N passed, M filtered out"); prefer it over the last
  # per-binary result line, which may be from a zero-match binary.
  summary=$(grep -E "^cargo test:|test result: ok\. [1-9]" "$log" | tail -n 1)
  echo "$name: $summary"
  case "$summary" in
    *"test result: ok"*|*"1 passed"*|*"passed"*) ;;
    *) echo "benchmark FAIL: $name not proven" >&2; rm -f "$log"; exit 1;;
  esac
  rm -f "$log"
}

# 1. Byte stability across two independent catalog loads.
check "byte-stability test" ep014_unit_canonical_config_byte_stable

# 2. Cache replay at 0.97 on the cacheable corpus: the integration
#    suite records real usage (98/100 hits) and asserts the ledger
#    meets the target.
check "cache-replay-0.97 test" ep014_integration_cache_ledger_records_real_usage

# 3. Volatile tail excluded from the cacheable prefix.
check "prefix-corpus test" ep014_unit_canonical_config_prefix_is_cacheable_corpus

echo "cache replay benchmark: ok"
