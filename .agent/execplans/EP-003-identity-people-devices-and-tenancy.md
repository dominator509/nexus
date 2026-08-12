NODE-META-BEGIN
ID: EP-003
DEPS: EP-002
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-003
VERIFY_SENTINEL: node verify EP-003: ok
GREEN_TAG: green/EP-003
NODE-META-END

# 1. Purpose / Big Picture

Implement people, households, businesses, devices, sessions, presence evidence, and tenant boundaries. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-003.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-003.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-002` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-003.md`
- `.agent/specs/SPEC-001-core-domain-identity-references-and-world-model.md`
- `.agent/specs/SPEC-005-authentication-authorization-secrets-trust-and-multi-user-privacy.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-003.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-003-identity-people-devices-and-tenancy.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-003.txt`
- `.agent/node-contracts/EP-003.md`
- `scripts/nodes/EP-003.sh`
- `crates/nexus-identity/`
- `crates/nexus-presence/`
- `schemas/identity/`
- `tests/identity/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `Principal` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |
| `PrincipalType` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |
| `PersonProfile` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |
| `Household` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |
| `BusinessBinding` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |
| `DeviceIdentity` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |
| `PresenceEvidence` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |
| `IdentityConfidence` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |
| `InteractionContext` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |
| `PrivacyContext` | `nexus-identity` | Defined by EP-003; provider-neutral and versioned |

Acceptance obligations:

1. People, households, businesses, devices, and sessions remain independently scoped
2. Voice, room, BLE, mobile, and camera evidence combine without becoming cryptographic authentication
3. Unknown and guest users receive bounded local permissions
4. Cross-tenant and cross-business reads fail without existence disclosure

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement people, households, businesses, devices, sessions, presence evidence, and tenant boundaries.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-003-M1.txt`, `.agent/node-contracts/EP-003.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-003-identity-people-devices-and-tenancy.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-003.txt`, `.agent/node-contracts/EP-003.md`, `scripts/nodes/EP-003.sh`, `crates/nexus-identity/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep003_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-003.sh M1`

EXPECT:

- `EP-003 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-003 MILESTONE_PASS "M1 EP-003 M1: ok"`

FALLBACK: Use account plus device identity without probabilistic presence fusion until every evidence provider is available. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-003][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-003.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-003-M2.txt`, `.agent/node-contracts/EP-003.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `crates/nexus-presence/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep003_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-003.sh M2`

EXPECT:

- `EP-003 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-003 MILESTONE_PASS "M2 EP-003 M2: ok"`

FALLBACK: Use account plus device identity without probabilistic presence fusion until every evidence provider is available. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-003][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-003 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-003-M3.txt`, `.agent/node-contracts/EP-003.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `schemas/identity/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep003_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-003.sh M3`

EXPECT:

- `EP-003 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-003 MILESTONE_PASS "M3 EP-003 M3: ok"`

FALLBACK: Use account plus device identity without probabilistic presence fusion until every evidence provider is available. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-003][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-003 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-003-M4.txt`, `.agent/node-contracts/EP-003.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/identity/`

CONTENT:

1. Create tests whose names begin `ep003_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-003.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-003 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-003 MILESTONE_PASS "M4 EP-003 M4: ok"`

FALLBACK: Use account plus device identity without probabilistic presence fusion until every evidence provider is available. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-003][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-003.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-003-M5.txt`, `.agent/node-contracts/EP-003.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, and ledger may change in this milestone.

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-003` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-003.sh M5`
2. `sh scripts/node-verify.sh EP-003`
3. `sh scripts/scope-audit.sh EP-003`

EXPECT:

- `EP-003 M5: ok`
- `node verify EP-003: ok`
- `scope audit EP-003: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-003 MILESTONE_PASS "M5 EP-003 M5: ok"`

FALLBACK: Use account plus device identity without probabilistic presence fusion until every evidence provider is available. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-003][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-003` and observe `node verify EP-003: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

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

M1 completed 2026-08-12: `crates/nexus-identity` created with all ten public
interfaces (Principal, PersonProfile, Household, BusinessBinding,
DeviceIdentity, PresenceEvidence, IdentityConfidence, InteractionContext,
PrivacyContext) plus Session/SessionState; vocabulary enums added by
ADR-007 (EvidenceKind, ConfidenceLevel, DeviceKind, TrustLevel,
LifecycleState, SessionState); 30 `ep003_unit_` tests + dependency-direction
test pass. Sentinel: `EP-003 M1: ok`. Fence extended with Cargo.toml,
Cargo.lock, docs/vocabulary/README.md, references/ADR-007; node-contract
spec path typo fixed.

M2 completed 2026-08-12: `crates/nexus-presence` created with the
`PresenceFusionEngine` (recency-weighted fusion, single-source cap 0.6,
stale-evidence fail-closed), `GuestPolicy` (bounded local permissions for
unknown/guest principals), and `TenantGuard` (uniform NotFound across
tenant boundaries, no existence disclosure). 13 `ep003_unit_` tests +
dependency-direction test pass. Sentinel: `EP-003 M2: ok`.

M3 completed 2026-08-12: `schemas/identity/` created with nine canonical
JSON Schemas (principal, person-profile, household, business-binding,
device-identity, presence-evidence, identity-confidence,
interaction-context, privacy-context, session); 4 `ep003_integration_` tests
prove identity records, sessions, and presence evidence round-trip through
real postgres:18.4 on dynamic host ports with container cleanup. Sentinel:
`EP-003 M3: ok`.

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-12 (M1): **Identity vocabulary added by ADR-007.** EvidenceKind,
  ConfidenceLevel, DeviceKind, TrustLevel, LifecycleState, SessionState are
  vocabulary-locked enums owned by `nexus-identity`. Evidence: ADR-007,
  vocabulary README, unit tests, M3 schemas. Alternative rejected: free-form
  strings (lose parse-time rejection). Consequence: `nexus-identity`
  depends on `nexus-domain` and serde only. Reversal: remove enums, ADR, and
  vocabulary entries together.
- 2026-08-12 (M1): **Workspace membership and fence extension.** Adding
  `crates/nexus-identity` requires root Cargo.toml membership; Cargo.lock
  regenerated offline (89 packages, +9 lines for the new member). The
  node's machine fence was extended with Cargo.toml, Cargo.lock,
  docs/vocabulary/README.md, and references/ADR-007 per the EP-002
  precedent. Alternative rejected: stand-alone crate outside the workspace
  (breaks `--locked` workspace gates). Reversal: remove the member.
- 2026-08-12 (M1): **Session revocation dominates expiry.** A revoked
  session stays revoked even if `expire()` is called later; the unit test
  caught the naive overwrite. Alternative rejected: allow expiry to
  downgrade revocation (weaker fail-closed semantics).
- 2026-08-12 (M2): **Single-source presence cap.** `PresenceFusionEngine`
  caps fused confidence at 0.6 when only one evidence kind is fresh, so a
  lone camera or BLE observation can never reach HIGH. Evidence: fusion
  unit tests. Alternative rejected: trust any single evidence source
  (violates the combine-multiple-kinds obligation and INV-003).
- 2026-08-12 (M2): **Presence behavior lives in `nexus-presence`.** The
  identity types stay in `nexus-identity`; the engine, guest bounds, and
  tenant guard are behavior in the presence crate. Alternative rejected:
  fold behavior into `nexus-identity` (blurs the M1/M2 fence).
- 2026-08-12 (M3): **Identity schemas are standalone canonical contracts.**
  `schemas/identity/` holds nine JSON Schema 2020-12 documents mirroring
  the identity enums and wire names; the generator globs top-level schemas
  only, so identity bindings are not generated until a later node owns
  cross-language identity contracts. Evidence: integration tests
  round-trip the Rust types through real postgres JSONB with exact enum
  wire values. Alternative rejected: force `schemas/identity/` into the
  generator now (out of this node's fence).

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
