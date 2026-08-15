NODE-META-BEGIN
ID: EP-014
DEPS: EP-013
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-014
VERIFY_SENTINEL: node verify EP-014: ok
GREEN_TAG: green/EP-014
NODE-META-END

# 1. Purpose / Big Picture

Implement DeepSeek V4 Flash ReflexProvider, effort tiers, deterministic prompt segments, cache accounting, and schema validation. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-014.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-014.txt`.
- Implement real behavior, tests, telemetry, security, operations, and any owning live-fire proof.
- Preserve self-hosted-first selection and API fallback contracts.
- Keep optional providers disabled until certified.

# 3. Non-goals

- No work owned by a later node.
- No broad refactor, dependency replacement, vendor-specific domain model, or alternate architecture.
- No production deployment.
- No mocks, stubs, demonstration modes, or sample success in production paths.
- No claim that an adapter or hardware class is operational before real certification.
- No weakening of a spec, policy, security boundary, test, or GraphLock gate.

# 4. Context and Orientation

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-013` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-014.md`
- `.agent/specs/SPEC-009-reflex-ai-model-gateway-routing-cache-and-microbrain-seam.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-014.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-014-deepseek-reflex-and-cache.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-014.txt`
- `.agent/node-contracts/EP-014.md`
- `scripts/nodes/EP-014.sh`
- `crates/nexus-reflex/`
- `config/prompts/reflex/`
- `tests/models/reflex/`
- `benchmarks/reflex/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `ReflexProvider` | `nexus-reflex` | Defined by EP-014; provider-neutral and versioned |
| `DeepSeekFlashProvider` | `nexus-reflex` | Defined by EP-014; provider-neutral and versioned |
| `PromptSegment` | `nexus-reflex` | Defined by EP-014; provider-neutral and versioned |
| `CacheLedger` | `nexus-reflex` | Defined by EP-014; provider-neutral and versioned |
| `EffortPolicy` | `nexus-reflex` | Defined by EP-014; provider-neutral and versioned |
| `NexusControlObjectValidator` | `nexus-reflex` | Defined by EP-014; provider-neutral and versioned |
| `ReflexDecision` | `nexus-reflex` | Defined by EP-014; provider-neutral and versioned |

Acceptance obligations:

1. Deterministic tasks bypass the model
2. Non-thinking, high, and max effort are policy selected
3. Stable prefix segments are canonical and versioned
4. Rolling token cache-hit ratio is measured and targets at least 0.97 on the cacheable corpus
5. Only validated NexusControlObject output continues

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement deepseek v4 flash reflexprovider, effort tiers, deterministic prompt segments, cache accounting, and schema validation.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-014-M1.txt`, `.agent/node-contracts/EP-014.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-014-deepseek-reflex-and-cache.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-014.txt`, `.agent/node-contracts/EP-014.md`, `scripts/nodes/EP-014.sh`, `crates/nexus-reflex/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep014_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-014.sh M1`

EXPECT:

- `EP-014 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-014 MILESTONE_PASS "M1 EP-014 M1: ok"`

FALLBACK: Use direct DeepSeek calls with the same segment and validation contracts when the preferred gateway is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-014][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-014.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-014-M2.txt`, `.agent/node-contracts/EP-014.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `config/prompts/reflex/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep014_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-014.sh M2`

EXPECT:

- `EP-014 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-014 MILESTONE_PASS "M2 EP-014 M2: ok"`

FALLBACK: Use direct DeepSeek calls with the same segment and validation contracts when the preferred gateway is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-014][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-014 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-014-M3.txt`, `.agent/node-contracts/EP-014.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/models/reflex/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep014_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-014.sh M3`

EXPECT:

- `EP-014 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-014 MILESTONE_PASS "M3 EP-014 M3: ok"`

FALLBACK: Use direct DeepSeek calls with the same segment and validation contracts when the preferred gateway is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-014][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-014 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-014-M4.txt`, `.agent/node-contracts/EP-014.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `benchmarks/reflex/`

CONTENT:

1. Create tests whose names begin `ep014_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-014.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-014 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-014 MILESTONE_PASS "M4 EP-014 M4: ok"`

FALLBACK: Use direct DeepSeek calls with the same segment and validation contracts when the preferred gateway is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-014][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-014.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-014-M5.txt`, `.agent/node-contracts/EP-014.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, and ledger may change in this milestone.

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-014` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-014.sh M5`
2. `sh scripts/node-verify.sh EP-014`
3. `sh scripts/scope-audit.sh EP-014`

EXPECT:

- `EP-014 M5: ok`
- `node verify EP-014: ok`
- `scope audit EP-014: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-014 MILESTONE_PASS "M5 EP-014 M5: ok"`

FALLBACK: Use direct DeepSeek calls with the same segment and validation contracts when the preferred gateway is unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-014][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-014` and observe `node verify EP-014: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [x] M3: Real dependency and transport integration
- [x] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-15 | Decision: Build `crates/nexus-reflex` as the EP-014 reflex plane crate. It re-exports the canonical model-plane vocabulary (`EffortTier`, `PromptSegment`, `CacheHitRatio`, `NexusControlObject`, `ProviderHealth`, `PromptSegmentPart`, `UsageReport`) from `nexus-model-gateway` instead of redefining it (ADR-018 owns those names), and adds the reflex-specific vocabulary (`ReflexDecisionClass`, `EffortSelectionClass`) plus provider-neutral ports/types (`ReflexProvider`, `ReflexTransport`, `ReflexRequest`, `ReflexDecision`, `DeepSeekFlashProvider`, `EffortPolicy`, `EffortInput`, `CacheLedger`, `PromptSegmentCatalog`, `NexusControlObjectValidator`) recorded in ADR-021. Evidence: `EP-014 M1: ok` (40 unit tests + 1 dependency-direction), `format check: ok`, `lint: ok` (clippy clean). Alternatives: redefine PromptSegment/EffortTier in the reflex crate (rejected: vocabulary-locked names must not be duplicated; ADR-018 owns them); couple the provider to a vendor SDK directly (rejected: transport is injected behind `ReflexTransport` so no vendor SDK enters the production tree). Consequence: deterministic tasks bypass the model (EffortTier::Deterministic resolves without a transport); non-deterministic tasks fail closed with UNAVAILABLE until a real transport is injected in M3; control-object validation is deterministic and rejects extra/invalid fields. Reversal: revert M1 commit. Security: credentials never enter requests or telemetry; errors are typed and redacted. License: no new dependency classes (serde/serde_json already workspace-pinned). Compatibility: additive workspace member; no existing surface changed.
- 2026-08-15 | Decision: Add the canonical prompt segment catalog at `config/prompts/reflex/` (catalog.json + 8 versioned segment files: constitution, schemas, capability-taxonomy, risk-policy, examples, tenant-context stable prefix; session-context, dynamic-request volatile tail) and the loader `PromptSegmentCatalog::from_canonical_dir` that reads it, validates canonical order/coverage (exactly constitution..tenant-context in the stable prefix), and rejects missing or unversioned segments. This satisfies SPEC-009 required behavior 4 (ordered immutable head, volatile tail) and the stable-prefix cacheable-corpus invariant. Evidence: `EP-014 M2: ok` (44 unit tests incl. 4 real-config invariants + 1 dependency-direction), clippy clean, format/lint ok. Alternatives: embed segment strings in code (rejected: the machine fence owns `config/prompts/reflex/`; canonical versioned artifacts must live as config, not literals); load at request time from a database (rejected: M2 keeps domain rules pure and I/O behind the catalog loader; no DB dependency is selected for prompts). Consequence: byte-stable canonical serialization is proven across loads; volatile ids/timestamps stay in the tail; the cacheable prefix is exactly the stable six segments. Reversal: revert M2 commit. Security: segment content is configuration, not secrets; no credentials in files. License: none new. Compatibility: additive config + loader.
- 2026-08-15 | Decision: Wire the real DeepSeek V4 Flash transport boundary. `DeepSeekReflexTransport` (new `src/transport.rs`) adapts EP-013's real `OpenAiCompatibleTransport` (pinned ureq) to the reflex `ReflexTransport` port, using the canonical DeepSeek manifest (`deepseek-v4-flash`, base `https://api.deepseek.com/v1`, credential ref `secret/model/deepseek`) from `config/models/providers/providers.json` / COMPONENT_REGISTRY. The M3 integration suite (`tests/models/reflex/` + `crates/nexus-reflex/tests/ep014_integration_transport.rs`) proves the real boundary over a controlled provider sandbox: allow path normalizes the canonical control object, malformed provider response fails closed VALIDATION, HTTP 500 -> EXTERNAL_PROVIDER, connection refused -> UNAVAILABLE, deterministic tasks bypass the real transport entirely, and cache ledger meets the 0.97 target on real usage. Real defect found and fixed: the EP-013 transport emits envelope schema_version "1.0" and wraps model text in `control.content`; the reflex adapter must parse the text as the structured control payload and stamp the canonical 1.0.0 version (fail closed on malformed text). Evidence: `EP-014 M3: ok` (6 integration tests + unit tests), clippy clean, format/lint ok. Alternatives: call the DeepSeek API directly with a new SDK (rejected: EP-013 already owns the real ureq transport behind the ModelProvider port; the reflex adapter reuses it, keeping one pinned HTTP path); in-memory fake transport (rejected: M3 requires a real dependency and real transport). Consequence: the V1 primary ReflexProvider now has a real transport path; provider certification of the live commercial API remains a later gate (the sandbox proves the boundary, not the vendor). Reversal: revert M3 commit. Security: credential value is passed to the transport without logging and never serialized; Debug redacts. License: no new dependency classes (nexus-model-transport and its pinned ureq are already workspace members). Compatibility: additive adapter + tests; no existing surface changed.
- 2026-08-15 | Decision: Add the M4 failure/abuse suite and the cache-replay benchmark. `crates/nexus-reflex/tests/ep014_failure_reflex.rs` proves 7 REAL failure mechanisms against the production adapters: provider unreachable -> UNAVAILABLE fail-closed, malformed provider payload -> VALIDATION, authority-bypass attempt (model granting itself a scope) -> VALIDATION unknown-field rejection (SPEC-009 behavior 10), duplicate deterministic request -> byte-identical decision (idempotent by construction), failed model call leaves no poisoned state (subsequent deterministic request succeeds), telemetry redaction (Debug never prints credential/prompt), and cache-ledger rollback safety. `benchmarks/reflex/cache-replay.sh` is the deterministic cacheable-corpus benchmark: byte stability across loads, 0.97 cache replay on real recorded usage, and volatile-tail exclusion. Evidence: `EP-014 M4: ok` (7 failure tests), `security check: ok`, `license gate: ok`, cache replay benchmark ok, clippy clean, format/lint ok. Alternatives: mock the provider for failures (rejected: M4 CONTENT item 2 requires exercising the real failure mechanism; the sandbox scripts the failure and the adapter under proof is never mocked); skip the benchmark (rejected: SPEC-009 required tests include cache replay at 0.97 and prompt byte stability, which M4 owns). Consequence: every reflex failure mode is proven fail-closed; the cacheable-corpus discipline is benchmarked deterministically. Reversal: revert M4 commit. Security: redaction proven at the reflex boundary; no credentials in evidence. License: none new. Compatibility: additive tests + benchmark; no existing surface changed.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
