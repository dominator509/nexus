# EP-014 Reflex Benchmarks (SPEC-009; ADR-021)

This directory owns the cacheable-corpus benchmark for the reflex
plane. `cache-replay.sh` is the deterministic benchmark gate:

- Byte stability: the canonical stable prefix serializes identically
  across independent loads (`ep014_unit_canonical_config_byte_stable`).
- Cache replay at 0.97: real recorded usage (98/100 cache hits per
  request) keeps the rolling `CacheLedger` ratio at or above the
  SPEC-009 target (`ep014_integration_cache_ledger_records_real_usage`).
- Corpus boundary: the volatile tail (session context, dynamic request)
  is never part of the cacheable prefix
  (`ep014_unit_canonical_config_prefix_is_cacheable_corpus`).

The benchmark is deterministic: no network, no credentials, no volatile
bytes in the measured serialization. It exits nonzero on any violation.
