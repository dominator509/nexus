NODE-META-BEGIN
ID: EP-013
DEPS: EP-012
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-013
VERIFY_SENTINEL: node verify EP-013: ok
GREEN_TAG: green/EP-013
NODE-META-END

# 1. Purpose / Big Picture

Implement the model provider registry, Bifrost-preferred gateway adapter, budgets, fallbacks, and provider health. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-013.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-013.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-012` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-013.md`
- `.agent/specs/SPEC-009-reflex-ai-model-gateway-routing-cache-and-microbrain-seam.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-013.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-013-model-gateway-and-provider-registry.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-013.txt`
- `.agent/node-contracts/EP-013.md`
- `scripts/nodes/EP-013.sh`
- `crates/nexus-model-gateway/`
- `infra/bifrost/`
- `config/models/`
- `tests/models/gateway/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `ModelProvider` | `nexus-model-gateway` | Defined by EP-013; provider-neutral and versioned |
| `ModelGateway` | `nexus-model-gateway` | Defined by EP-013; provider-neutral and versioned |
| `ProviderRegistry` | `nexus-model-gateway` | Defined by EP-013; provider-neutral and versioned |
| `ProviderHealth` | `nexus-model-gateway` | Defined by EP-013; provider-neutral and versioned |
| `ModelBudget` | `nexus-model-gateway` | Defined by EP-013; provider-neutral and versioned |
| `ModelRequest` | `nexus-model-gateway` | Defined by EP-013; provider-neutral and versioned |
| `ModelResponse` | `nexus-model-gateway` | Defined by EP-013; provider-neutral and versioned |
| `ToolCallEnvelope` | `nexus-model-gateway` | Defined by EP-013; provider-neutral and versioned |

Acceptance obligations:

1. Bifrost is preferred but hidden behind ModelGateway
2. Direct provider adapters remain available for replacement and diagnostics
3. Budgets, retries, rate limits, fallbacks, and usage accounting are consistent
4. Provider credentials never leave the gateway

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement the model provider registry, bifrost-preferred gateway adapter, budgets, fallbacks, and provider health.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-013-M1.txt`, `.agent/node-contracts/EP-013.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-013-model-gateway-and-provider-registry.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-013.txt`, `.agent/node-contracts/EP-013.md`, `scripts/nodes/EP-013.sh`, `crates/nexus-model-gateway/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep013_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-013.sh M1`

EXPECT:

- `EP-013 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-013 MILESTONE_PASS "M1 EP-013 M1: ok"`

FALLBACK: Use direct DeepSeek and OpenAI-compatible providers if Bifrost fails conformance or license review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-013][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-013.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-013-M2.txt`, `.agent/node-contracts/EP-013.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/bifrost/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep013_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-013.sh M2`

EXPECT:

- `EP-013 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-013 MILESTONE_PASS "M2 EP-013 M2: ok"`

FALLBACK: Use direct DeepSeek and OpenAI-compatible providers if Bifrost fails conformance or license review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-013][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-013 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-013-M3.txt`, `.agent/node-contracts/EP-013.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `config/models/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep013_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-013.sh M3`

EXPECT:

- `EP-013 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-013 MILESTONE_PASS "M3 EP-013 M3: ok"`

FALLBACK: Use direct DeepSeek and OpenAI-compatible providers if Bifrost fails conformance or license review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-013][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-013 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-013-M4.txt`, `.agent/node-contracts/EP-013.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/models/gateway/`

CONTENT:

1. Create tests whose names begin `ep013_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-013.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-013 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-013 MILESTONE_PASS "M4 EP-013 M4: ok"`

FALLBACK: Use direct DeepSeek and OpenAI-compatible providers if Bifrost fails conformance or license review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-013][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-013.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-013-M5.txt`, `.agent/node-contracts/EP-013.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, and ledger may change in this milestone.

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-013` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-013.sh M5`
2. `sh scripts/node-verify.sh EP-013`
3. `sh scripts/scope-audit.sh EP-013`

EXPECT:

- `EP-013 M5: ok`
- `node verify EP-013: ok`
- `scope audit EP-013: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-013 MILESTONE_PASS "M5 EP-013 M5: ok"`

FALLBACK: Use direct DeepSeek and OpenAI-compatible providers if Bifrost fails conformance or license review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-013][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-013` and observe `node verify EP-013: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [x] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

M1 detail (2026-08-14): built `crates/nexus-model-gateway/` contract crate (manifest, lib, vocabulary, error, registry, budget, gateway modules). All 8 public interfaces from the node contract (ModelProvider, ModelGateway, ProviderRegistry, ProviderHealth, ModelBudget, ModelRequest, ModelResponse, ToolCallEnvelope) defined with typed IDs, authenticated tenant/principal context, canonical errors, correlation, and idempotency. Vocabulary enums parse canonical strings and reject unknowns (SPEC-009). ADR-018 written and docs/vocabulary/README.md updated in the same milestone. Dependency direction enforced by `ep013_unit_dependency_direction` (only serde, serde_json, nexus-domain, nexus-identity, nexus-auth allowed). M1 gate wired in scripts/nodes/EP-013.sh (set -e style, no rc masking). Fence amended to add root Cargo.toml, Cargo.lock, docs/vocabulary/README.md, references/ADR-018 (cross-node workspace member change, precedent chain EP-011/EP-012). Observed sentinels: `EP-013 M1: ok`, `scope audit EP-013: ok`, `security check: ok`, `license gate: ok`, `reality gate: ok`, `format check: ok`, `lint: ok`. Expected-files full-fence red is structural: `infra/bifrost/` is M2-manifest-owned, completes at node end (EP-012 `infra/gateway/` precedent). 31 tests green (30 ep013_unit + 1 dependency direction).

# 12. Surprises & Discoveries

- 2026-08-14: `f64` fields in ModelRoute/ModelRouteDecision (`cache_hit_ratio`) make the structs non-`Eq`; derived `Eq` must be dropped (compile error).
- 2026-08-14: `BudgetDecision` needed `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` to match the wire form of every other vocabulary enum; a unit test caught `"Denied"` vs `"DENIED"`.
- 2026-08-14: `provider_mut` on ProviderRegistry requires an explicit `'static` trait-object lifetime bound (`&mut (dyn ModelProvider + 'static)`) to compile against the stored `Box<dyn ModelProvider + Send + Sync + 'static>`.
- 2026-08-14: Root Cargo.toml workspace member addition is a cross-node file; scope audit flags it. Fence amendment is the established remedy (EP-011 Cargo.toml, EP-012 ExecPlan precedents), recorded here before commit.
- 2026-08-14: `BifrostError`/`ModelGatewayErrorCode` serialize with declared variant names (`"ExternalProvider"`), not the canonical SCREAMING_SNAKE_CASE string; `as_str()` remains the canonical form. Test asserts both surfaces.
- 2026-08-14: The budget-denied proof needed an exact allowed-count assertion (66 calls allowed before denial, no Allowed event after), not a blanket "no Allowed ever" check that the pre-denial successes would violate.
- 2026-08-14: `f64` backoff factor makes `RetryPolicy` non-`Eq` (same class of issue as M1 `cache_hit_ratio`).
- 2026-08-14: ureq 2.12.1 with `default-features = false` does NOT re-export `ureq::http`; status-error classification is proven by the real 429 sandbox integration test, not a constructed unit error.
- 2026-08-14: ureq transport errors classify via `ureq::ErrorKind` (ConnectionFailed/Dns/Io) with a TimedOut source check, exactly as infra/opa 2.12.1 does; the `Transport::TimeoutKind` path does not compile in this configuration.
- 2026-08-14: The transport `Debug` impl prints `credential_present: bool`, never the value; proven by a unit test that formats the transport and asserts the secret string is absent.

# 13. Decision Log

- 2026-08-14 | Decision: Add `crates/nexus-model-gateway/` as the EP-013 contract crate; keep it provider-neutral with real adapters deferred to M2/M3 (`infra/bifrost/`, `config/models/`). Evidence: `EP-013 M1: ok` (31 tests), crate compiles with `--locked`. Alternatives: adapters in the same crate now (rejected: mixes provider specifics into ports, violates SPEC-009 layering). Consequence: M1 is pure contract, later milestones prove real behavior. Reversal: revert M1 commit. Security: no secrets in contracts; credentials stay in gateway config behind typed refs. License: no new dependencies beyond serde/serde_json. Compatibility: crate is additive; no existing surface changed.
- 2026-08-14 | Decision: Amend `.agent/expected-files/EP-013.txt` to add root `Cargo.toml`, `Cargo.lock`, `docs/vocabulary/README.md`, `references/ADR-018-model-gateway-and-provider-registry-vocabulary.md` because the workspace member registration and same-milestone vocabulary obligations touch cross-node paths. Evidence: `scope audit EP-013: ok` after amendment. Alternatives: reject the workspace member (breaks repo build); duplicate vocabulary docs (breaks same-milestone rule). Consequence: fence now matches reality; node-end full-fence audit still gates `infra/bifrost/` at M2. Reversal: revert fence file. Security: none. License: none. Compatibility: none.
- 2026-08-14 | Decision: Build `infra/bifrost/` as the `nexus-bifrost` adapter crate implementing the real `ModelGateway` contract: deterministic router (Bifrost preferred when healthy AND certified; deterministic fallback to direct providers; budget exhaustion and certification failures fail closed), retry policy with deterministic backoff, fixed-window rate limiting, fallback chain, and usage accounting after success. I/O sits behind the `ModelProvider`/`ModelBudget` ports; real HTTP transports land in M3 `config/models/`. Evidence: `EP-013 M2: ok` (30 model-gateway unit + 1 dep-dir + 26 bifrost unit = 57 tests), all side gates green. Alternatives: put transports in M2 now (rejected: M3 owns transport/config per milestone manifest); implement router as provider-specific (rejected: violates SPEC-009 provider neutrality). Consequence: gateway behavior is real and deterministic in M2; transport wiring is M3. Reversal: revert M2 commit. Security: credentials referenced by id only, never serialized (proven by `ep013_unit_config_credential_ref_never_value`); telemetry redacted. License: no new dependencies beyond serde/serde_json/nexus-domain/nexus-model-gateway. Compatibility: additive; gateway implements the existing M1 contract unchanged.
- 2026-08-14 | Decision: Build `config/models/` as the `nexus-model-transport` crate: a REAL ureq HTTP transport implementing `ModelProvider` against the OpenAI-compatible chat completions surface (Bifrost preferred gateway, DeepSeek V4 Flash fallback), plus provider manifests (`config/models/providers/providers.json`) recording exact component identity per M3 requirement 6. Integration tests use a controlled provider sandbox (authorized by M3 CONTENT item 3 and TESTING.md integration layer): a scripted OpenAI-compatible HTTP server proving real request bytes, canonical response normalization, typed error classification (429 -> ExternalProvider, connection refused -> Unavailable), and the full BifrostGateway + real transport + budget composition. Evidence: `EP-013 M3: ok` (10 unit + 1 dep-dir + 4 integration), side gates green. Alternatives: call the live DeepSeek API (rejected: requires credentials and provider certification, not available in this environment; certification is a later gate); fake transport in-memory (rejected: reality gate). Consequence: transport boundary proven over real HTTP; live provider certification remains a future gate. Reversal: revert M3 commit. Security: credentials resolved by the caller and never logged (Debug redacts; proven by unit test); manifests carry credential REFS only. License: ureq 2.12.1 pinned identical to infra/opa + infra/openfga. Compatibility: additive; transport implements the M1 ModelProvider contract unchanged.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
