# EP-014 M5 live-fire and node closure evidence

Node: EP-014 (nexus-reflex, DeepSeek V4 Flash ReflexProvider)
Commit: 1fbe57b (M1-M4) + M5 additions below
Date: 2026-08-15
Owner: hermes-nexus-main

## 1. Runtime composition (A / EP-044 runtime smoke now mandatory)

EP-044 is NODE_DONE, so the Nexus runtime smoke is MANDATORY for EP-014
node verification. The real EP-044 control plane was brought up with the
canonical repository-owned mechanism and real compose profile:

- `NEXUS_SMOKE_URL=http://127.0.0.1:8443 sh scripts/local-start.sh core`
  -> `local start core: ok` (real `infra/compose/core.yaml`, image
  `nexus-control-plane:local`, container `nexus-control-plane`)
- Live endpoint probes (observed):
  - `GET /healthz` -> `{"status":"healthy"}`
  - `GET /readyz` -> `{"ready":true}`
  - `GET /v1/capabilities` -> `{"capabilities":["health","capabilities"]}` (non-empty)
- `NEXUS_SMOKE_URL=http://127.0.0.1:8443 sh scripts/smoke/runtime.sh`
  -> `runtime smoke: ok`
- `sh scripts/node-verify.sh EP-014` -> `node verify EP-014: ok` with the
  observed line `runtime smoke: ok` (NOT `not-applicable-before EP-044`).
- LF-029 (EP-044 runtime smoke live-fire, run inside verify.sh) starts the
  real runtime, proves the three endpoints, then tears it down via the
  canonical `local-stop.sh`; the runtime is re-started through the owner for
  the committed-state verification below.

Runtime composition: PASS
Runtime smoke (mandatory, real container): PASS

## 2. Real EP-014 implementation exercised (B)

The M5 proof runs the actual production implementation (no mini-provider,
no simplified prompt constructor):

- `DeepSeekFlashProvider` (`crates/nexus-reflex/src/provider.rs`)
- `DeepSeekReflexTransport` (`src/transport.rs`, wrapping EP-013's real
  `OpenAiCompatibleTransport` pinned ureq HTTP client)
- `EffortPolicy` / `EffortInput` (`src/effort.rs`)
- `PromptSegmentCatalog` / `StablePrefix` (`src/segments.rs`, loading the
  REAL canonical config at `config/prompts/reflex/`)
- `CacheLedger` (`src/cache.rs`)
- `NexusControlObjectValidator` (`src/validator.rs`)
- `ReflexDecision` / `ReflexDecisionClass` (`src/decision.rs`,
  `src/vocabulary.rs`)

Gate: `sh scripts/nodes/EP-014.sh M5` -> `EP-014 M5: ok`
(`python3 scripts/node-artifact-check.py EP-014 M5` + full crate suite:
65 tests passed, 0 failed).

## 3. Effort routing (C)

Deterministic effort policy (EffortPolicy::select) governs routing; the
model never chooses its own authority or route. Representative requests
proven by the committed suite (all observed `test result: ok`):

- Deterministic task (`EffortTier::Deterministic`) -> tier DETERMINISTIC,
  selection class EXPLICIT, provider route: model BYPASSED (no transport
  call, 0 prompt tokens), control object `route: DETERMINISTIC`, validated.
  Tests: `ep014_unit_deterministic_tasks_resolve_to_deterministic`,
  `ep014_unit_deterministic_task_bypasses_model_without_transport`,
  `ep014_integration_deterministic_task_bypasses_real_transport`.
- Trivial work -> tier NON_THINKING (never MAX), policy-selected when no
  explicit tier; explicit MAX on trivial work REJECTED by validate().
  Test: `ep014_unit_trivial_work_is_never_max`.
- Default (no explicit tier) -> tier HIGH, class POLICY_SELECTED.
  Test: `ep014_unit_default_is_high`.
- Explicit NON_THINKING / HIGH / MAX / SPECIALIST -> honored
  (class EXPLICIT). Test: `ep014_unit_explicit_tiers_are_honored`.
- Deterministic request with a thinking tier -> REJECTED.
  Test: `ep014_unit_deterministic_task_rejects_thinking_tier`.
- High-effort model path -> provider `deepseek-v4-flash`, 8 canonical
  prompt segments (real config: constitution, schemas, capability-taxonomy,
  risk-policy, examples, tenant-context, session-context, dynamic-request),
  final decision class MODEL, control object validated.
  Test: `ep014_integration_real_transport_normalizes_control_object`.

Effort routing: PASS

## 4. Byte-stable stable prefix (D)

- Same logical stable context constructed twice -> identical canonical
  bytes (`ep014_unit_canonical_serialization_is_byte_stable`,
  `ep014_unit_canonical_config_byte_stable` over the REAL config dir).
- Same stable prefix, only the request-specific tail
  (session-context/dynamic-request) changed -> stable prefix bytes
  IDENTICAL, dynamic portion CHANGED
  (`ep014_unit_stable_prefix_identical_when_tail_changes`, added M5).
- No wall-clock time, random IDs, unstable whitespace, or environment data
  in the prefix: canonical serialization is fixed-order, version-tagged,
  and excludes the volatile tail (`ep014_unit_volatile_tail_is_not_in_prefix`,
  `ep014_unit_canonical_config_prefix_is_cacheable_corpus`).

Stable-prefix byte stability: PASS

## 5. Intentional prefix invalidation (E)

- Segment version bump (CONSTITUTION 1.0 -> 1.1) -> stable-prefix bytes
  CHANGE (`ep014_unit_stable_prefix_fingerprint_changes_on_version_bump`,
  added M5).
- Stable content change (tenant-context) -> stable-prefix bytes CHANGE
  (same test).
- Unchanged inputs -> byte identical (control, same test).
- Unversioned segment rejected (`ep014_unit_canonical_config_rejects_unversioned_segment`).

Intentional prefix invalidation: PASS

## 6. Precise cache claims (F)

- `sh benchmarks/reflex/cache-replay.sh` -> `cache replay benchmark: ok`
  (observed):
  - byte-stability test: ok (1 passed)
  - cache-replay-0.97 test: ok (1 passed) - ledger records real usage
    98/100 prompt-token hits per call over 2 calls = 196/200
  - prefix-corpus test: ok (1 passed)
- Nexus deterministic prefix/cacheability proof: PASS
- Measured replay/cacheability ratio (Nexus ledger over the deterministic
  controlled corpus): 0.98 (196/200 tokens; target >= 0.97 per SPEC-009).
- Provider-reported production cached-token ratio: NOT ASSERTED. The
  `prompt_cache_hit_tokens` values in the integration suite are scripted by
  the controlled provider sandbox (protocol simulator, TESTING.md
  integration layer); no real DeepSeek production telemetry was observed,
  so no provider cache-hit claim is made from the replay benchmark.

## 7. Real DeepSeek provider live-fire (G)

- Real transport boundary: PASS. `DeepSeekReflexTransport` (production)
  performs real HTTP through EP-013's pinned ureq transport over real
  sockets against a controlled provider sandbox speaking the
  OpenAI-compatible chat completions protocol; allow path,
  malformed-response rejection, HTTP 500 classification, connection-refused
  classification all proven
  (`ep014_integration_real_transport_*`, 6 tests).
- External DeepSeek account live-fire: NOT ASSERTED. The EP-014 contract
  does not own commercial-API certification (M3 decision log: "the sandbox
  proves the boundary, not the vendor"); no DeepSeek credential is
  available in the environment (no DEEPSEEK_* env vars; the manifest
  credential is a reference `secret/model/deepseek`, never a value).

## 8. Control object validation (H)

- Valid canonical object -> accepted (`ep014_unit_validator_accepts_canonical_object`,
  `ep014_integration_real_transport_normalizes_control_object`).
- Fail-closed for invalid provider output, all observed ok:
  - malformed JSON/control object (`ep014_unit_deepseek_transport_normalizes_schema_version_and_control`,
    malformed control text -> VALIDATION)
  - missing required field (`ep014_unit_validator_rejects_missing_required_field`)
  - unsupported schema version (`ep014_unit_validator_rejects_wrong_schema_version`,
    `ep014_integration_real_transport_rejects_malformed_provider_response`)
  - invalid route/effort enum (`ep014_unit_validator_rejects_unknown_route`,
    `ep014_unit_effort_tier_rejects_unknown`)
  - invalid risk/privacy vocabulary (`ep014_unit_validator_rejects_unknown_risk`)
  - extra/unknown field (`ep014_unit_validator_rejects_extra_field`)
  - duplicate capabilities, out-of-range confidence, missing boolean
    (`ep014_unit_validator_rejects_duplicate_capabilities`,
    `ep014_unit_validator_rejects_out_of_range_confidence`,
    `ep014_unit_validator_rejects_missing_boolean`)
- Provider text alone is never trusted: only validator-approved control
  objects continue (`DeepSeekFlashProvider::reflex` calls
  `validator.validate` on every model result).

Control-object validation: PASS

## 9. Provider content boundary / no model authority (I)

- Model-emitted `"authorization":"ALLOW"` inside the control payload ->
  REJECTED by the validator as an unknown field (VALIDATION), added M5
  (`ep014_failure_model_allow_string_grants_no_authority`).
- A valid model decision serializes WITHOUT any `authorization` key and
  without the string `ALLOW` (same test).
- Model attempting to grant itself a scope (`grants:["admin"]`,
  `auth.grant`) -> REJECTED (`ep014_failure_authority_bypass_attempt_rejected`).
- `ReflexDecision` carries only request/correlation ids, class, and the
  validated control object; there is no authorization authority field in
  the type (`src/decision.rs`). ActionGateway authorization remains the
  deterministic EP-008 authority path; the model result is advisory
  control-routing input only.

Provider content boundary: PASS

## 10. Real failure path (J)

All committed M4 failure semantics preserved and re-proven by the M5 full
suite (7 `ep014_failure_*` tests, observed ok):

- provider unavailable (listener closed, real connect refused) -> UNAVAILABLE
- malformed provider response (missing usage) -> VALIDATION
- HTTP 500 -> EXTERNAL_PROVIDER
- authority-bypass -> VALIDATION (unknown field)
- duplicate deterministic request -> byte-identical decision (idempotent)
- failed model call leaves no poisoned state (subsequent deterministic
  request succeeds)
- telemetry redaction (Debug never prints credential value or prompt
  content)
- cache ledger rollback safety (failed model path never pollutes ledger;
  invalid records rejected)

No empty/default success object is ever returned on failure; no
fabricated ReflexDecision.

Failure fail-closed behavior: PASS

## 11. Cache ledger (K)

`CacheLedger` (`src/cache.rs`) records only safe metadata: prompt-token
counts and cache-hit prompt-token counts per cacheable request (window
bounded, hit clamped to prompt, zero-prompt records ignored). It never
persists prompt content, credentials, or request bodies. Rolling
cache-hit ratio computed over the fixed window
(`ep014_unit_rolling_ratio_computes_over_window`,
`ep014_unit_window_is_bounded`, `ep014_unit_cache_hit_never_exceeds_prompt`,
`ep014_unit_ledger_serde_round_trip`).

Cache ledger: PASS (safe metadata only)

## 12. Stable segment catalog (L)

All 8 canonical segments load from `config/prompts/reflex/` with locked
versions (`catalog.json` + 8 files); proven by real-config tests:

- deterministic ordering (`ep014_unit_canonical_config_loads`:
  constitution first, dynamic-request last, 8 parts)
- deterministic serialization (`ep014_unit_canonical_config_byte_stable`)
- duplicate segment rejection (`ep014_unit_stable_prefix_rejects_wrong_segments`)
- missing required segment rejection (`ep014_unit_canonical_config_rejects_missing_segment`)
- unsupported/unversioned rejection (`ep014_unit_canonical_config_rejects_unversioned_segment`)
- no network dependency: `from_canonical_dir` reads local files only
- no silent fallback to embedded alternate text: the catalog loader
  validates every referenced segment file and fails closed

Stable segment catalog: PASS

## 13. Configuration / secret boundary (M)

- No credential value anywhere in `config/prompts/reflex/`,
  `config/runtime/core.json`, this evidence, the ledger, or telemetry
  (grep audit for `api_key|secret|password|token|bearer|sk-` -> only
  vocabulary prose and "no credentials" statements; `config/runtime/`
  zero matches).
- Credential material exists only as a reference
  (`secret/model/deepseek`) resolved through the canonical secret
  injection path; the transport is configured with a value at runtime
  that is never logged or serialized
  (`ep014_failure_telemetry_redacts_credential_and_prompt`,
  `ep014_unit_deepseek_transport_normalizes_schema_version_and_control`).
- Test fixture strings (`test-credential`,
  `super-secret-credential-value`) are test-only sentinels used to PROVE
  redaction, never real secrets.

Configuration/secret boundary: PASS

## 14. Performance claims (O)

A bounded M5 run does not certify production throughput, latency under
load, month-long cache hit rate, cost savings, or provider SLA. No such
claim is made. The only numbers asserted are the deterministic ledger
ratio (0.98 on the controlled corpus) and the 0.97 SPEC-009 target.

## 15. Genuine defects found and fixed in M5

1. Committed M1-M4 files were never rustfmt-formatted (the prior session's
   "full M4 gate chain" was interrupted before it ran). The M5 full-suite
   gate exposed `cargo fmt --all -- --check` failing across the whole
   crate. Fixed: `cargo fmt --all` applied; `format check: ok` now green.
2. `ep014_unit_deepseek_transport_normalizes_schema_version_and_control`
   (M3, transport.rs) asserted `!dbg.contains("credential")`, but the
   manifest legitimately prints the `credential_ref` FIELD NAME (a
   reference, not a secret). The test was never executed by any earlier
   gate (M1 ran before transport.rs existed; M3/M4 gates filter to
   integration/failure selectors). Fixed: assert the credential VALUE is
   absent, consistent with the M4 telemetry test. 65/65 now green.
3. Three M5-directed tests added per the closure directive:
   `ep014_unit_stable_prefix_identical_when_tail_changes` (D),
   `ep014_unit_stable_prefix_fingerprint_changes_on_version_bump` (E),
   `ep014_failure_model_allow_string_grants_no_authority` (I).

## 16. Ownership / graph records (T)

- EP-044 is DONE: runtime smoke is MANDATORY and was observed
  `runtime smoke: ok` against the real control-plane during node verify
  and closure checks.
- LF-029 (owned by EP-044) brings the runtime up and tears it down
  canonically inside verify.sh; the runtime is re-started for committed-
  state verification, then stopped cleanly (teardown section below).
- `smoke gate regression: not re-run as a pass post-EP-044-DONE` - test 1
  of `tests/runtime/smoke-gate-regression.sh` asserts the pre-owner
  `not-applicable-before EP-044` sentinel, which is permanently
  unsatisfiable once EP-044 is DONE (by design; the proof was recorded
  during EP-044's own closure). EP-014's closure relies on the live
  runtime smoke (ok) and the fail-closed absence probe
  (`NEXUS_SMOKE_URL=http://127.0.0.1:1 sh scripts/smoke/runtime.sh`
  fails, proving absent runtime never passes).

## 17. Closure gates observed (Q-S)

- `EP-014 M5: ok`
- `node verify EP-014: ok` (includes `runtime smoke: ok`, expected-files,
  verify)
- `scope audit EP-014: ok`
- `expected files EP-014: ok`
- adapter parity: 8x `3505091078 1453` (8/8 PRIME-BLOCK checksums)
- `blueprint validation: ok`
- `security check: ok`
- `license gate: ok`
- `reality gate: ok`
- `format check: ok`
- `lint: ok`
- `dependency audit: ok`
- `runtime smoke: ok` (real container)
- cache replay benchmark: ok

## 18. Teardown (P)

After committed-state verification (below) the runtime is stopped with the
canonical EP-044 mechanism (`sh scripts/local-stop.sh core`) and the
absence is proven: no `nexus-control-plane` container, no runtime orphan,
no test process orphan, no leaked credential temp files.

No credentials or sensitive raw prompt material are contained in this
evidence.
