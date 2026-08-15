# EP-016 Context and Memory Failure Tests

The EP-016 M4 failure and abuse suite lives in the owning crate:

- `crates/nexus-memory-workers/tests/ep016_failure_workers.rs` - 16
  `ep016_failure_*` tests exercising REAL failure mechanisms against
  the production workers (no mocking of the component under proof;
  injected ports script the failure):
  - `ep016_failure_vector_unavailable_renormalizes_without_synthetic_score`
  - `ep016_failure_provider_unavailable_fails_closed`
  - `ep016_failure_malformed_request_validation_fails_closed`
  - `ep016_failure_zero_budget_rejected_fails_closed`
  - `ep016_failure_cross_tenant_excluded_before_scoring`
  - `ep016_failure_shared_room_denies_sensitive_above_ceiling`
  - `ep016_failure_routing_decision_recorded_delivery_not_owned`
  - `ep016_failure_budget_flood_required_exact_not_crowded_out`
  - `ep016_failure_duplicate_consolidation_idempotent`
  - `ep016_failure_consolidation_partial_sources_conservative_merge`
  - `ep016_failure_semantic_unavailable_deterministic_fallback`
  - `ep016_failure_semantic_error_fails_closed`
  - `ep016_failure_consolidation_missing_sources_not_found`
  - `ep016_failure_graph_cycle_bounded_no_infinite_loop`
  - `ep016_failure_error_redacted_no_memory_content`
  - `ep016_failure_telemetry_redacted_no_content`

Run the suite with the M4 gate:

```sh
sh scripts/nodes/EP-016.sh M4
```

which executes `cargo test --locked -p nexus-memory-workers ep016_failure`
(after artifact check and a vacuity guard). Expected: `EP-016 M4: ok`.

Companion gates required by the M4 milestone:

```sh
sh scripts/security-check.sh   # security check: ok
sh scripts/license-gate.sh     # license gate: ok
```
