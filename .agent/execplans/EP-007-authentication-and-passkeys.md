NODE-META-BEGIN
ID: EP-007
DEPS: EP-006
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-007
VERIFY_SENTINEL: node verify EP-007: ok
GREEN_TAG: green/EP-007
NODE-META-END

# 1. Purpose / Big Picture

Deploy Keycloak and implement OIDC, passkeys, service identities, sessions, device enrollment, and step-up. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-007.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-007.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-006` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-007.md`
- `.agent/specs/SPEC-005-authentication-authorization-secrets-trust-and-multi-user-privacy.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-007.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-007-authentication-and-passkeys.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-007.txt`
- `.agent/node-contracts/EP-007.md`
- `scripts/nodes/EP-007.sh`
- `crates/nexus-auth/`
- `infra/keycloak/`
- `tests/auth/`
- `schemas/auth/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `OidcClient` | `nexus-auth` | Defined by EP-007; provider-neutral and versioned |
| `TokenValidator` | `nexus-auth` | Defined by EP-007; provider-neutral and versioned |
| `PasskeyEnrollment` | `nexus-auth` | Defined by EP-007; provider-neutral and versioned |
| `DeviceEnrollment` | `nexus-auth` | Defined by EP-007; provider-neutral and versioned |
| `SessionService` | `nexus-auth` | Defined by EP-007; provider-neutral and versioned |
| `StepUpChallenge` | `nexus-auth` | Defined by EP-007; provider-neutral and versioned |
| `RecoveryKit` | `nexus-auth` | Defined by EP-007; provider-neutral and versioned |

Acceptance obligations:

1. Owner onboarding creates passkey and offline recovery material
2. Tokens validate issuer, audience, signature, time, scopes, and device context
3. Sessions revoke and audit correctly
4. High-risk operations require configured step-up strength

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for deploy keycloak and implement oidc, passkeys, service identities, sessions, device enrollment, and step-up.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-007-M1.txt`, `.agent/node-contracts/EP-007.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-007-authentication-and-passkeys.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-007.txt`, `.agent/node-contracts/EP-007.md`, `scripts/nodes/EP-007.sh`, `crates/nexus-auth/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep007_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-007.sh M1`

EXPECT:

- `EP-007 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-007 MILESTONE_PASS "M1 EP-007 M1: ok"`

FALLBACK: Use Keycloak local accounts and passkeys before enabling external identity federation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-007][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-007.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-007-M2.txt`, `.agent/node-contracts/EP-007.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/keycloak/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep007_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-007.sh M2`

EXPECT:

- `EP-007 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-007 MILESTONE_PASS "M2 EP-007 M2: ok"`

FALLBACK: Use Keycloak local accounts and passkeys before enabling external identity federation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-007][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-007 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-007-M3.txt`, `.agent/node-contracts/EP-007.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/auth/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep007_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-007.sh M3`

EXPECT:

- `EP-007 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-007 MILESTONE_PASS "M3 EP-007 M3: ok"`

FALLBACK: Use Keycloak local accounts and passkeys before enabling external identity federation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-007][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-007 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-007-M4.txt`, `.agent/node-contracts/EP-007.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `schemas/auth/`

CONTENT:

1. Create tests whose names begin `ep007_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-007.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-007 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-007 MILESTONE_PASS "M4 EP-007 M4: ok"`

FALLBACK: Use Keycloak local accounts and passkeys before enabling external identity federation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-007][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-007.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-007-M5.txt`, `.agent/node-contracts/EP-007.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: Only the active ExecPlan progress, Decision Log, and ledger may change in this milestone.

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-007` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-007.sh M5`
2. `sh scripts/node-verify.sh EP-007`
3. `sh scripts/scope-audit.sh EP-007`

EXPECT:

- `EP-007 M5: ok`
- `node verify EP-007: ok`
- `scope audit EP-007: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-007 MILESTONE_PASS "M5 EP-007 M5: ok"`

FALLBACK: Use Keycloak local accounts and passkeys before enabling external identity federation. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-007][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-007` and observe `node verify EP-007: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-003` `owner-passkey-onboarding`: Create an owner, enroll a passkey and recovery material, sign in, revoke the session, and verify audit records.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary - `EP-007 M1: ok` (56 ep007_unit tests + 1 dependency-direction; 8 files in crates/nexus-auth); ADR-011; vocabulary README entries; 7 public interfaces (OidcClient, TokenValidator, PasskeyEnrollment, DeviceEnrollment, SessionService, StepUpChallenge, RecoveryKit); workspace member registered; challenge time windows issuance-to-expiry
- [x] M2: Core behavior and deterministic invariants - `EP-007 M2: ok` (22 adapter ep007_unit tests + 1 dependency-direction); infra/keycloak adapter crate `nexus-keycloak` on Keycloak 26.7.0 (pinned): realm import (3 clients), OIDC discovery normalization, Keycloak claims <-> nexus-auth TokenClaims mapping + validator construction, deterministic authorize/refresh/client-credentials URL bodies; provider-neutral boundary enforced by dependency-direction test
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

- 2026-08-13 | ADR-011 authentication vocabulary | SPEC-005 locks Authentication Strength, Service Identity, Capability Token, Secret Reference, Trust Zone; ADR-010 documented AuthenticationStrength for the workflow package but no Rust representation existed. Added ADR-011 + Rust enums in `crates/nexus-auth` (AuthenticationStrength ordered ladder NONE < SINGLE_FACTOR < MULTI_FACTOR < STEP_UP, TokenClass, PasskeyState, DeviceEnrollmentState, StepUpState, RecoveryMaterialKind, GrantFlow, RecoveryKitState) + vocabulary README entries. Service Identity / Capability Token / Secret Reference / Trust Zone remain locked names owned by later nodes. | Vocabulary parse-time rejection; new names require ADR.
- 2026-08-13 | Provider-neutral auth contracts crate | `crates/nexus-auth` never imports an OIDC/Keycloak/WebAuthn SDK; it depends only on nexus-domain + nexus-identity + serde (enforced by ep007_unit_auth_crate_has_no_infrastructure_dependencies over real cargo tree). Keycloak adapter lives in infra/keycloak (M2). | Reversal: engine import in contracts would fail dependency-direction test.
- 2026-08-13 | Fence amended (EP-001/EP-003 precedent) | `.agent/expected-files/EP-007.txt` adds `Cargo.toml` + `Cargo.lock` (workspace member registration for the new crate), `docs/vocabulary/README.md` (auth vocabulary entries), `references/ADR-011-authentication-and-passkey-vocabulary.md` (new ADR). Mirrors the EP-003 fence (which lists Cargo.toml, Cargo.lock, docs/vocabulary/README.md, references/ADR-007). | Scope audit would otherwise reject these legitimately changed paths.
- 2026-08-13 | Challenge time windows are issuance-to-expiry | PasskeyChallenge and StepUpChallenge carry `created_at_unix_s`; `is_valid_at` requires `created_at <= now < expires` (a challenge is not usable before issuance). Construction rejects inverted windows. Test suite caught the gap (is_valid_at(999) before creation was wrongly true). | Honest validity windows; no challenge usable before issuance.
- 2026-08-13 | Keycloak adapter is deterministic-core first | `infra/keycloak` (`nexus-keycloak`) implements the deterministic adapter surface in M2 (realm import, discovery normalization, claim mapping, URL/request body construction) with no HTTP transport; the real container integration lands in M3 (EP-005 M2/M3 precedent: adapter unit tests first, real dependency in integration). Signature verification is a boundary flag (`signature_verified`) set by the transport layer, never assumed. | Transport without a real dependency would be untestable; M3 proves it against a real Keycloak 26.7.0 container.
- 2026-08-13 | Realm import owns three canonical clients | `nexus-app` (public, authorization-code + refresh, PKCE), `nexus-scheduler` (confidential, client-credentials + refresh, workflow scope), `nexus-connector-runtime` (confidential, client-credentials, connector scope). Service identities use client credentials per SPEC-005 behavior 1. | Canonical clients; later nodes register additional clients through the realm import only.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
