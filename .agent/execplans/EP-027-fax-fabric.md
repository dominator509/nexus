NODE-META-BEGIN
ID: EP-027
DEPS: EP-026
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-027
VERIFY_SENTINEL: node verify EP-027: ok
GREEN_TAG: green/EP-027
NODE-META-END

# 1. Purpose / Big Picture

Implement ICTFax, HylaFAX compatibility, fax documents, inbound routing, outbound status, T.38 or carrier fallback, and audit. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-027.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-027.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-026` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-027.md`
- `.agent/specs/SPEC-014-email-phone-fax-notifications-and-communications-routing.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-027.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-027-fax-fabric.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-027.txt`
- `.agent/node-contracts/EP-027.md`
- `scripts/nodes/EP-027.sh`
- `crates/nexus-fax/`
- `connectors/ictfax/`
- `connectors/hylafax/`
- `infra/fax/`
- `tests/fax/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `FaxProvider` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `IctFaxProvider` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `HylaFaxProvider` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `CloudFaxProvider` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `FaxJob` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `FaxDocument` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `FaxStatus` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |
| `InboundFaxRoute` | `nexus-fax` | Defined by EP-027; provider-neutral and versioned |

Acceptance obligations:

1. ICTFax is the primary self-hosted control sidecar
2. HylaFAX is a compatibility backend
3. Cloud carrier fallback uses the same FaxProvider contract
4. Outbound and inbound documents, status, retries, routing, and audit are real

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement ictfax, hylafax compatibility, fax documents, inbound routing, outbound status, t.38 or carrier fallback, and audit.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M1.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-027-fax-fabric.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-027.txt`, `.agent/node-contracts/EP-027.md`, `scripts/nodes/EP-027.sh`, `crates/nexus-fax/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep027_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M1`

EXPECT:

- `EP-027 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M1 EP-027 M1: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-027.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M2.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/ictfax/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep027_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M2`

EXPECT:

- `EP-027 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M2 EP-027 M2: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-027 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M3.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/hylafax/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep027_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M3`

EXPECT:

- `EP-027 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M3 EP-027 M3: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-027 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M4.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/fax/`

CONTENT:

1. Create tests whose names begin `ep027_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-027 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M4 EP-027 M4: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-027.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-027-M5.txt`, `.agent/node-contracts/EP-027.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/fax/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-027` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-027.sh M5`
2. `sh scripts/node-verify.sh EP-027`
3. `sh scripts/scope-audit.sh EP-027`

EXPECT:

- `EP-027 M5: ok`
- `node verify EP-027: ok`
- `scope audit EP-027: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-027 MILESTONE_PASS "M5 EP-027 M5: ok"`

FALLBACK: Use HylaFAX with a certified modem or SIP path if ICTFax packaging cannot pass the selected deployment profile. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-027][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-027` and observe `node verify EP-027: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-013` `fax-lifecycle`: Send a real test fax through the certified profile, receive status callbacks, route inbound fax, and archive the artifact.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary (2026-08-19; gate + node sentinels observed; commit pending)
- [ ] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-19 M1: The pre-created EP-027 M1 test for unknown vocabulary fed a JSON object (`{"kind":"ICT_FAX"}`) to a bare-string enum. serde treats `kind` as a variant name and rejects it with `unknown variant \`kind\``. The wire vocabulary is SCREAMING_SNAKE_CASE (`ICT_FAX`/`HYLA_FAX`/`CLOUD_FAX`), confirmed by the actual serde error message. Fixed the test to use bare strings and added explicit serde rename attributes so the wire spelling is vocabulary-locked, not serde-default accidental.
- 2026-08-19 M1: `FaxNumber` and the typed ids derived `Deserialize`, which bypassed the `new()` contract checks: an invalid number or empty id could be constructed from the wire. Added custom `Deserialize` impls that run the same normalization/validation (fail closed, never bypass). Tests prove invalid wire values are rejected and valid ones round-trip.
- 2026-08-19 M1: `validate_send_request` accepted a request whose `approval_class` was below the job requirement (the field existed but was ignored). Added the policy check; test proves `Policy` error before any provider call.
- 2026-08-19 M1: There was no seam proving "no provider mutation after denial". Added `submit_governed` (validate -> policy -> provider.submit) and `verify_delivery` (exact-target, SUBMITTED never verifies). A tracking provider test proves denied sends make zero `submit` calls and approved sends make exactly one.
- 2026-08-19 M1: The write/read tool redacts phone-like literals at the display layer (`+15551234567` shown as `+155****4567`); grep/od confirm the file bytes are correct. Tests use split literals where a canonical dial string is needed so file bytes are never masked.
- 2026-08-19 M1: `cargo test` splits the suite across two binaries (15 unit + 1 dependency-direction = 16); the gate floor guard must sum passed counts across binaries, not match a single result line.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-19 M1 | Canonical provider-kind wire representation: explicit serde renames `ICT_FAX`/`HYLA_FAX`/`CLOUD_FAX` (SCREAMING_SNAKE_CASE), vocabulary-locked; internal `as_str()` keeps domain constants (`ICTFAX`, ...) distinct from wire spelling. Evidence: `ep027_unit_provider_kind_wire_vocabulary` + `ep027_unit_unknown_vocabulary_rejected` green. Alternatives: serde default naming (rejected: accidental undocumented protocol), object-tagged wire form (rejected: not the enum's serde shape). Consequence: changing a wire spelling is a schema change requiring ADR + ledger entry. Reversal: revert enum renames to `rename_all` if the blueprint mandates it. Security/license/compat: no new deps; no compat impact (crate is new).
- 2026-08-19 M1 | Fax-number normalization: E.164-ish canonical form (strip space/dash/dot/paren, single leading `+`, 7..=16 digits, deterministic output), rejecting letters, empty, too-short/too-long, embedded/repeated `+`, and any non-canonical residue. Evidence: `ep027_unit_fax_number_normalization` green. Alternatives: store raw dial strings (rejected: domain never compares raw dial strings per SPEC-014). Consequence: providers carry carrier-specific rendering; the domain compares normalized numbers only. Reversal: adjust normalization per SPEC-014 schema update.
- 2026-08-19 M1 | State-ladder semantics: DRAFT < QUEUED < SUBMITTING < SUBMITTED < DELIVERED plus terminal FAILED/CANCELLED/ARCHIVED; SUBMITTED is carrier acceptance, DELIVERED requires independent recipient/carrier evidence. Evidence: `ep027_unit_submitted_is_not_delivered` + `verify_delivery` exact-target tests green. Alternatives: treat carrier 200/acceptance as delivery (rejected: would fabricate delivery, Reality rule). Consequence: later provider milestones must carry delivery evidence, never infer it from submission.
- 2026-08-19 M1 | Pre-mutation gates: `submit_governed` runs `validate_send_request` (job match, idempotency key, approval class) then `enforce_fax_policy` (approval minimum, scan CLEAN, sender != recipient) BEFORE any `provider.submit`; denied sends never reach the carrier. Evidence: `ep027_unit_governed_submit_denies_before_provider_mutation` (tracking provider: zero submits on every denial, exactly one on approval). Alternatives: validate inside providers (rejected: per-provider drift, no central proof). Consequence: adapters must call `submit_governed`, not bare `submit`. Reversal: none without ADR.
- 2026-08-19 M1 | Serde must not bypass contract checks: `FaxNumber` and typed ids implement custom `Deserialize` running the same validation as `new()`. Evidence: `ep027_unit_number_and_ids_fail_closed_via_serde` green. Alternatives: derive `Deserialize` (rejected: invalid numbers/empty ids constructible from wire). Consequence: wire payloads are validated at the boundary; malformed values fail closed. Reversal: derive again only with a schema change.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

- 2026-08-19 M1: Contract crate green and gate replacement complete. Changed files vs fence: all M1-owned paths only (crates/nexus-fax/, scripts/ep027-m1-tests.sh, scripts/nodes/EP-027.sh M1 branch, .agent/milestone-files/EP-027-M1.txt, .agent/expected-files/EP-027.txt Cargo.toml/Cargo.lock registration, ExecPlan). Commands + sentinels: `cargo test -p nexus-fax --all-targets` -> 15 unit + 1 dependency-direction, 0 failed; `sh scripts/ep027-m1-tests.sh` -> `EP-027 M1: ok`; `sh scripts/nodes/EP-027.sh M1` -> `EP-027 M1: ok`; scope audit EP-027: ok; security check: ok; license gate: ok; reality gate: ok; blueprint validation: ok; dependency audit: ok. Certification: M1 is INTERNAL CONTRACT CERTIFIED only; no fax provider claimed. Assumptions: SPEC-014 vocabulary locked per node contract. Remaining risks: provider transport, delivery evidence, and live-fire owned by M2-M5.
