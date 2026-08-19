NODE-META-BEGIN
ID: EP-028
DEPS: EP-027
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-028
VERIFY_SENTINEL: node verify EP-028: ok
GREEN_TAG: green/EP-028
NODE-META-END

# 1. Purpose / Big Picture

Implement the authenticated Nexus-to-Hydra capability, context, action, event, identity, and business binding seam. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-028.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-028.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-027` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-028.md`
- `.agent/specs/SPEC-015-business-control-hydra-crm-social-command-center-and-attribution.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-028.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-028-hydra-business-control-plane.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-028.txt`
- `.agent/node-contracts/EP-028.md`
- `scripts/nodes/EP-028.sh`
- `crates/nexus-hydra/`
- `connectors/hydra/`
- `schemas/hydra/`
- `tests/hydra/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `HydraProvider` | `nexus-hydra` | Defined by EP-028; provider-neutral and versioned |
| `HydraBusinessBinding` | `nexus-hydra` | Defined by EP-028; provider-neutral and versioned |
| `HydraCapabilityMap` | `nexus-hydra` | Defined by EP-028; provider-neutral and versioned |
| `HydraContextProjection` | `nexus-hydra` | Defined by EP-028; provider-neutral and versioned |
| `HydraActionRequest` | `nexus-hydra` | Defined by EP-028; provider-neutral and versioned |
| `HydraEventConsumer` | `nexus-hydra` | Defined by EP-028; provider-neutral and versioned |

Acceptance obligations:

1. Hydra remains the CRM canonical source
2. Nexus uses authenticated MCP, REST, and durable events only
3. Business-to-Hydra tenant binding is explicit
4. Dual authorization gates and end-to-end correlation are preserved

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement the authenticated nexus-to-hydra capability, context, action, event, identity, and business binding seam.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-028-M1.txt`, `.agent/node-contracts/EP-028.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-028-hydra-business-control-plane.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-028.txt`, `.agent/node-contracts/EP-028.md`, `scripts/nodes/EP-028.sh`, `crates/nexus-hydra/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep028_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-028.sh M1`

EXPECT:

- `EP-028 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-028 MILESTONE_PASS "M1 EP-028 M1: ok"`

FALLBACK: Use read-only Hydra context and proposal generation until Hydra execution capabilities advertise certified availability. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-028][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-028.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-028-M2.txt`, `.agent/node-contracts/EP-028.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/hydra/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep028_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-028.sh M2`

EXPECT:

- `EP-028 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-028 MILESTONE_PASS "M2 EP-028 M2: ok"`

FALLBACK: Use read-only Hydra context and proposal generation until Hydra execution capabilities advertise certified availability. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-028][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-028 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-028-M3.txt`, `.agent/node-contracts/EP-028.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `schemas/hydra/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep028_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-028.sh M3`

EXPECT:

- `EP-028 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-028 MILESTONE_PASS "M3 EP-028 M3: ok"`

FALLBACK: Use read-only Hydra context and proposal generation until Hydra execution capabilities advertise certified availability. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-028][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-028 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-028-M4.txt`, `.agent/node-contracts/EP-028.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/hydra/`

CONTENT:

1. Create tests whose names begin `ep028_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-028.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-028 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-028 MILESTONE_PASS "M4 EP-028 M4: ok"`

FALLBACK: Use read-only Hydra context and proposal generation until Hydra execution capabilities advertise certified availability. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-028][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-028.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-028-M5.txt`, `.agent/node-contracts/EP-028.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, and ledger may change in this milestone.

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-028` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-028.sh M5`
2. `sh scripts/node-verify.sh EP-028`
3. `sh scripts/scope-audit.sh EP-028`

EXPECT:

- `EP-028 M5: ok`
- `node verify EP-028: ok`
- `scope audit EP-028: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-028 MILESTONE_PASS "M5 EP-028 M5: ok"`

FALLBACK: Use read-only Hydra context and proposal generation until Hydra execution capabilities advertise certified availability. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-028][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-028` and observe `node verify EP-028: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-015` `hydra-cross-crm-command`: Ask for hot leads across businesses, receive canonical Hydra context, propose a governed update, execute it, and consume the resulting Hydra event.
- `LF-025` `ceo-business-brief`: Combine Hydra, social, communications, and finance connector data into a permission-filtered executive brief with source provenance.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary (2026-08-19; commit cc70806)
- [x] M2: Core behavior and deterministic invariants (2026-08-19; gate + node sentinels observed; commit pending)
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-19 M1: the pre-created node M1 branch ran only `python3 scripts/node-artifact-check.py EP-028 M1` - the EP-001 gate-masking class (artifact check certifies nothing). Replaced with `sh scripts/ep028-m1-tests.sh` (real suite + vacuity + anti-masking sentinel guards).
- 2026-08-19 M1: SPEC-015 behavior 2 (authenticated MCP/REST/durable events only) is encoded structurally: `HydraAccessChannel` has exactly MCP/REST/DURABLE_EVENT variants and no DIRECT_DATABASE variant; a fabricated direct-database channel is rejected at parse/serde. This makes the "no direct DB access" rule impossible to violate by construction.
- 2026-08-19 M1: SPEC-015 behavior 6 (identity linking only through deterministic or human-reviewed resolution) is enforced by `SocialAccount::with_link` and `CustomerReference::mergeable`: UNLINKED (or any non-owned class) fails closed; `LLM_GUESS` is not even a valid vocabulary value.
- 2026-08-19 M1: SPEC-015 behavior 8 (paid-ad budget changes and public crisis responses require human approval) is a deterministic gate in `enforce_hydra_action_policy`, proven with a tracking sink: denied requests make ZERO provider calls; human-approved requests reach the sink exactly once.
- 2026-08-19 M1: `HydraError` with `Option<String>` context fields trips clippy `result_large_err` (Err variant >= 128 bytes). Switched correlation/actor/tenant/resource to `Option<Box<str>>` (nexus-capabilities precedent); clippy -D warnings clean.
- 2026-08-19 M1: clippy also flagged an unused `request` parameter in the test TrackingSink impl (named `_request`).
- 2026-08-19 M2: the pre-created node M2 branch ran `python3 scripts/node-artifact-check.py EP-028 M2; cargo test --locked -p nexus-hydra ep028_unit` - artifact check plus the M1 CONTRACT crate suite, which certifies nothing about the M2 adapter (EP-001 masking class). Replaced with `sh scripts/ep028-m2-tests.sh` (real connector suite + M1 regression + anti-masking sentinel).
- 2026-08-19 M2: the in-flight idempotency concurrency proof needs the transport trait object to be shareable (`Box<dyn HydraTransport + Send + Sync>`); the first GatedTransport used an mpsc Receiver that is not Sync. Switched to a shared `Arc<Mutex<bool>>` gate (Mutex<bool> is Sync); the test proves a duplicate in-flight call is Conflict with zero second transport call, and a retry after completion is not a Conflict.
- 2026-08-19 M2: the zero-call proof requires the transport call counter to be observable by the test; `OkTransport` holds `Arc<AtomicUsize>` so denial proves 0 calls and approval proves exactly 1.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-19 M1 | Vocabulary-locked Hydra enums with explicit SCREAMING_SNAKE_CASE wire spellings (`HydraAccessChannel`, `BusinessScope`, `IdentityResolutionClass`, `HydraActionKind`, `HydraActionState`, `HydraCapabilityKind`, `SocialMessageState`, `CampaignState`, `CeoBriefSourceClass`, `LeadHandoffState`), each with FromStr + serde that reject unknown values (fail closed). Evidence: `ep028_unit_vocabulary_wire_spelling_locked`, `ep028_unit_vocabulary_rejects_unknown`, `ep028_unit_access_channels_have_no_direct_database_variant` green. Alternatives: serde default naming (rejected: accidental undocumented protocol), per-provider vocab (rejected: provider-neutral contract). Consequence: wire spelling changes require ADR + schema update. Security/license/compat: no new deps; crate is new.
- 2026-08-19 M1 | Semantic boundary REFERENCE != TRUTH: `CustomerReference` and `HydraContextProjection` carry Hydra person/business references only, never duplicated CRM records (SPEC-015 behavior 1; non-goal: duplicating Hydra CDM). Evidence: `ep028_unit_customer_reference_is_reference_not_truth`, `ep028_unit_projection_carries_references_only` green. Consequence: later milestones cannot accidentally build a second CRM.
- 2026-08-19 M1 | Semantic boundary SINGLE != PORTFOLIO: `BusinessContext` requires exactly one business for SINGLE_BUSINESS scope and forbids one for PORTFOLIO (SPEC-015 behavior 3). Evidence: `ep028_unit_business_context_single_requires_business` green. Consequence: cross-business isolation is explicit, never accidental.
- 2026-08-19 M1 | Semantic boundary APPROVED != PUBLISHED: `SocialMessage` state ladder enforces Draft -> PendingApproval -> Approved -> Published; publish without approval fails closed (SPEC-015 behavior 5; non-goal: blind social auto-replies). Evidence: `ep028_unit_social_message_approval_ladder` green.
- 2026-08-19 M1 | Policy before mutation: `hydra_action_governed` runs `enforce_hydra_action_policy` (validate + approval-class gate for PAID_AD_BUDGET_CHANGE / PUBLIC_CRISIS_RESPONSE) BEFORE any provider sink call; tracking-sink tests prove zero calls on denial and exactly one on approval (node contract acceptance obligation 4: dual authorization gates). Evidence: `ep028_unit_governed_action_denied_before_provider_zero_calls`, `ep028_unit_governed_action_human_approved_reaches_provider_once` green. Alternatives: validate after provider call (rejected: mutation before gate), per-provider gating (rejected: drift).
- 2026-08-19 M1 | Fail-closed capability map: a fresh `HydraCapabilityMap` advertises nothing; unadvertised capabilities resolve UNAVAILABLE (node contract fallback: read-only context + proposal generation until execution capabilities advertise certified availability). Evidence: `ep028_unit_capability_map_fails_closed_when_empty` green.
- 2026-08-19 M1 | Unbound provider fails closed: `UnboundHydraProvider` returns Unavailable for read_context/submit_action and advertises an empty capability map (reality rule: an interface is not operational merely because it compiles). Evidence: `ep028_unit_unbound_provider_fails_closed` green.
- 2026-08-19 M1 | Typed ids + serde-proof validation: all nine Hydra ids (`HydraBindingId` ... `HydraActionId`) validate 1..=128 chars in both `new` and `Deserialize` (wire input cannot bypass the invariant). Evidence: `ep028_unit_typed_ids_validate_and_reject`, `ep028_unit_typed_ids_serde_cannot_bypass_validation` green.
- 2026-08-19 M2 | Transport port + real HTTP transport: `HydraTransport` port fails closed (unbound -> Unavailable), `HttpHydraTransport` is the real blocking reqwest client over the versioned canonical surface (GET /v1/context, GET /v1/capabilities, POST /v1/actions, GET /v1/actions/{id}) with SPEC-006 status classification (401/403 -> Authorization, 404 -> NotFound, 409 -> Conflict, 429 -> RateLimit, 500/502/503/504 -> Unavailable, silent peer -> Timeout, refused -> Unavailable, malformed JSON -> External). The bearer credential is used ONLY for the Authorization header and never enters errors/telemetry. Evidence: `ep028_unit_classify_status_maps_spec006`, `ep028_unit_unbound_transport_fails_closed` green. Alternatives: in-memory transport only (rejected: M2 is the production adapter core; M3 owns the real dependency), invented vendor endpoints (rejected: anti-hallucination - the canonical surface is the M1 versioned contract).
- 2026-08-19 M2 | Adapter dual authorization gates: `HydraAdapter::submit_action` runs `enforce_hydra_action_policy` (validate + human-approval gate for PAID_AD_BUDGET_CHANGE / PUBLIC_CRISIS_RESPONSE) BEFORE any transport call, then the transport enforces the binding's authorized channel + credential (node contract acceptance obligation 4). Binding mismatch -> Authorization; inactive binding -> Policy. Evidence: `ep028_unit_governed_action_denied_makes_zero_transport_calls` (Arc<AtomicUsize> proves 0 calls), `ep028_unit_binding_mismatch_denied`, `ep028_unit_read_context_rejects_inactive_binding` green. Alternatives: trust callers (rejected: per-caller drift), validate after transport (rejected: mutation before gate).
- 2026-08-19 M2 | In-flight idempotency with release-after-end: duplicate in-flight action (same binding + action id + idempotency key) -> Conflict with ZERO second transport call; completion/failure releases the entry so a retry is not a Conflict. The transport trait object is `Send + Sync` so the concurrency proof shares the adapter across threads. Evidence: `ep028_unit_in_flight_duplicate_is_conflict_and_releases` green (real gated transport blocks the first call; second caller observes Conflict; after release a retry succeeds).
- 2026-08-19 M2 | Bounded poison-safe observability: `HydraObservability` (256-entry redacted audit ring + outcome counters + `hydra-<nanos>-<seq>` correlation) redacts configured secrets at insert so the ring is safe to dump after a poisoned error. Evidence: `ep028_unit_observability_redacts_secret_at_insert`, `ep028_unit_observability_bounded_ring_and_counters`, `ep028_unit_observability_correlation_ids_unique` green. Alternatives: unbounded logging (rejected: memory + privacy), no redaction (rejected: SECURITY.md).

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

- 2026-08-19 M1: Contract, vocabulary, and package boundary green. Changed files vs fence: crates/nexus-hydra/ (Cargo.toml, src/lib.rs, src/error.rs, src/vocabulary.rs, src/model.rs, src/capability.rs, src/action.rs, src/context.rs, src/events.rs, src/provider.rs, tests/dependency_direction.rs), scripts/ep028-m1-tests.sh (real gate), scripts/nodes/EP-028.sh (M1 branch rewired from the EP-001-masking artifact check), .agent/milestone-files/EP-028-M1.txt, .agent/expected-files/EP-028.txt (Cargo.toml/Cargo.lock/gate/fence registered), Cargo.toml (workspace member), Cargo.lock, ExecPlan. Commands + sentinels: `cargo test --locked -p nexus-hydra --all-targets` -> 27 passed 0 failed (25 unit + 2 dependency-direction; 2 suites); `sh scripts/ep028-m1-tests.sh` -> `EP-028 M1: ok`; `sh scripts/nodes/EP-028.sh M1` -> `EP-028 M1: ok` (RC=0); fmt clean; clippy -D warnings clean; scope audit EP-028: ok; security check: ok; license gate: ok; reality gate: ok; dependency audit: ok; blueprint validation: ok. Certification: M1 is INTERNAL CONTRACT CERTIFIED only (SPEC-015 behaviors 1,2,3,5,6,8 encoded structurally in the provider-neutral contract; no provider claimed). Assumptions: SPEC-015 vocabulary locked per node contract; M2 owns connectors/hydra, M3 schemas/hydra, M4 tests/hydra, M5 live-fire LF-015/LF-025. Remaining risks: provider transport, real Hydra/Postiz integration, live-fire owned by M2-M5; Postiz AGPL sidecar boundary deferred to M3.
- 2026-08-19 M2: Core behavior and deterministic invariants green. Changed files vs fence: connectors/hydra/ (Cargo.toml, src/lib.rs, src/transport.rs, src/adapter.rs, src/observability.rs), scripts/ep028-m2-tests.sh (real gate), scripts/nodes/EP-028.sh (M2 branch rewired from the EP-001-masking artifact+contract-crate line), .agent/milestone-files/EP-028-M2.txt, .agent/expected-files/EP-028.txt (M2 gate registered), Cargo.toml (workspace member), Cargo.lock, ExecPlan. Commands + sentinels: `cargo test --locked -p nexus-hydra-connector --all-targets` -> 11 passed 0 failed (adapter 6 + observability 3 + transport 2); `sh scripts/ep028-m2-tests.sh` -> `EP-028 M2: ok`; `sh scripts/nodes/EP-028.sh M2` -> `EP-028 M2: ok` (RC=0); M1 contract regression green (27 tests); fmt clean; clippy -D warnings clean; scope audit EP-028: ok; security check: ok; license gate: ok; reality gate: ok; dependency audit: ok; blueprint validation: ok. Certification: M2 is INTERNAL CONTRACT CERTIFIED adapter core over the versioned canonical transport surface (authenticated REST; SPEC-006 classification; dual authorization gates; in-flight idempotency; poison-safe observability); NO live Hydra instance claimed (M3 owns the real dependency + schemas/hydra + integration). Assumptions: the canonical Hydra REST surface is the M1 versioned contract (anti-hallucination: no invented vendor endpoints); M3 selects and pins the real component. Remaining risks: real Hydra/Postiz integration, live-fire LF-015/LF-025 owned by M3-M5; Postiz AGPL sidecar boundary deferred to M3 (EP-029 owns the Postiz connector itself).
