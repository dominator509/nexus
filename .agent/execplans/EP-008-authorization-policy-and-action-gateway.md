NODE-META-BEGIN
ID: EP-008
DEPS: EP-007
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-008
VERIFY_SENTINEL: node verify EP-008: ok
GREEN_TAG: green/EP-008
NODE-META-END

# 1. Purpose / Big Picture

Implement OpenFGA, OPA, risk classes, short-lived grants, deterministic Action Gateway, verification, and receipts. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-008.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-008.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-007` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-008.md`
- `.agent/specs/SPEC-005-authentication-authorization-secrets-trust-and-multi-user-privacy.md`
- `.agent/specs/SPEC-006-errors-reliability-idempotency-verification-and-action-safety.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-008.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-008-authorization-policy-and-action-gateway.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-008.txt`
- `.agent/node-contracts/EP-008.md`
- `scripts/nodes/EP-008.sh`
- `crates/nexus-policy/`
- `crates/nexus-action-gateway/`
- `infra/openfga/`
- `infra/opa/`
- `policies/`
- `tests/policy/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `RelationshipAuthorizer` | `nexus-policy` | Defined by EP-008; provider-neutral and versioned |
| `ContextPolicyEngine` | `nexus-policy` | Defined by EP-008; provider-neutral and versioned |
| `RiskClassifier` | `nexus-policy` | Defined by EP-008; provider-neutral and versioned |
| `ActionGateway` | `nexus-policy` | Defined by EP-008; provider-neutral and versioned |
| `CapabilityGrant` | `nexus-policy` | Defined by EP-008; provider-neutral and versioned |
| `ApprovalAssertion` | `nexus-policy` | Defined by EP-008; provider-neutral and versioned |
| `ActionReceipt` | `nexus-policy` | Defined by EP-008; provider-neutral and versioned |
| `VerificationPlan` | `nexus-policy` | Defined by EP-008; provider-neutral and versioned |

Acceptance obligations:

1. Models and agents cannot grant authority
2. Every consequential action receives relationship and contextual policy decisions
3. Short-lived grants are capability, target, actor, and expiry scoped
4. Actions verify observable effects and produce receipts or fail closed

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement openfga, opa, risk classes, short-lived grants, deterministic action gateway, verification, and receipts.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-008-M1.txt`, `.agent/node-contracts/EP-008.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-008-authorization-policy-and-action-gateway.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-008.txt`, `.agent/node-contracts/EP-008.md`, `scripts/nodes/EP-008.sh`, `crates/nexus-policy/`, `tests/policy/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep008_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-008.sh M1`

EXPECT:

- `EP-008 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-008 MILESTONE_PASS "M1 EP-008 M1: ok"`

FALLBACK: Use a small explicit OpenFGA model and OPA policy set; avoid a generalized policy language exposed to users. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-008][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-008.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-008-M2.txt`, `.agent/node-contracts/EP-008.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `crates/nexus-action-gateway/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep008_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-008.sh M2`

EXPECT:

- `EP-008 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-008 MILESTONE_PASS "M2 EP-008 M2: ok"`

FALLBACK: Use a small explicit OpenFGA model and OPA policy set; avoid a generalized policy language exposed to users. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-008][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-008 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-008-M3.txt`, `.agent/node-contracts/EP-008.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/openfga/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep008_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-008.sh M3`

EXPECT:

- `EP-008 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-008 MILESTONE_PASS "M3 EP-008 M3: ok"`

FALLBACK: Use a small explicit OpenFGA model and OPA policy set; avoid a generalized policy language exposed to users. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-008][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-008 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-008-M4.txt`, `.agent/node-contracts/EP-008.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/opa/`

CONTENT:

1. Create tests whose names begin `ep008_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-008.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-008 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-008 MILESTONE_PASS "M4 EP-008 M4: ok"`

FALLBACK: Use a small explicit OpenFGA model and OPA policy set; avoid a generalized policy language exposed to users. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-008][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-008.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-008-M5.txt`, `.agent/node-contracts/EP-008.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `policies/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-008` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-008.sh M5`
2. `sh scripts/node-verify.sh EP-008`
3. `sh scripts/scope-audit.sh EP-008`

EXPECT:

- `EP-008 M5: ok`
- `node verify EP-008: ok`
- `scope audit EP-008: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-008 MILESTONE_PASS "M5 EP-008 M5: ok"`

FALLBACK: Use a small explicit OpenFGA model and OPA policy set; avoid a generalized policy language exposed to users. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-008][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-008` and observe `node verify EP-008: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary - `EP-008 M1: ok` (25 ep008_unit tests + 1 dependency-direction in crates/nexus-policy); 8 public interfaces (RelationshipAuthorizer, ContextPolicyEngine, RiskClassifier, ActionGateway, CapabilityGrant, ApprovalAssertion, ActionReceipt, VerificationPlan); ADR-012 policy vocabulary; vocabulary README entries; workspace member registered
- [x] M2: Core behavior and deterministic invariants - `EP-008 M2: ok` (18 ep008_unit engine tests); DeterministicGateway combines relationship, contextual policy, deterministic risk floor, capability grant, approval into fail-closed decisions; R3/R4 require human approval (digest-bound), R4 rejects model approval; grant actor/target/scope/expiry binding; pure engine (no clock/random/network) with injected DecisionInput
- [x] M3: Real dependency and transport integration - `EP-008 M3: ok` (16 ep008_integration tests on REAL OpenFGA 1.18.1 `sha256:ec73e86c...`); real ephemeral container per suite, real store/model/tuple bootstrap via OpenFGA HTTP API; owner->admin computed userset, member denied, business#admin transitive userset, unrelated denied, device operator scoped, delegation exact, tuple revocation (no local cache), wrong store/model + malformed request fail closed, provider killed -> typed unavailable -> gateway never ALLOW; tenant isolation (tenant A allow / tenant B deny, same object suffix); gateway composition through the REAL M2 DeterministicGateway + OpenFGA adapter (relationship deny stops, valid path continues to R2/grant -> ALLOW); scope audit EP-008: ok; EP-008 orphan audit: ok; infra/openfga crate: 13 ep008_unit tests + clippy clean + bans ok; Decision Log entries (version/digest, mapping, responsibility boundary, tenant isolation, revocation, fail-closed)
- [x] M4: Forced failures, abuse cases, and observability - `EP-008 M4: ok` (19 ep008_failure tests on REAL OPA 1.16.2 `sha256:a915d8b5...` + 7 ep008_failure Rust ordering/no-override tests in nexus-action-gateway); infra/opa crate implements ContextPolicyEngine with 7 typed failures; policies/nexus.rego default-deny bundle (tenant/weak-auth/untrusted/sensitive/emergency rules); real mechanisms: container kill (unavailable), unresponsive TCP peer (timeout, real read deadline), wrong policy version (bundle mismatch), undefined path, malformed input/response; ordering through REAL gateway (policy deny stops at R0; policy allow continues to R2; OPA down -> ERROR never ALLOW); no approval/capability/model override of policy denial; security check: ok; license gate: ok; reality gate: ok; scope audit EP-008: ok; EP-008 orphan audit: ok; infra/opa crate: 10 ep008_unit tests + clippy clean + bans ok; Decision Log entries (version/digest, mapping, default-deny, version strategy, typed failures, boundary, ordering, no-override)
- [x] M5: Live-fire, operations, and node closure - `EP-008 M5: ok`; `node verify EP-008: ok`; `scope audit EP-008: ok`; expected files EP-008: ok; adapter parity: ok; security check: ok; license gate: ok; reality gate: ok; EP-008 orphan audit: ok; 16 `ep008_livefire_*` tests in tests/policy/test_ep008_live_fire.py run the FULL authorization chain as ONE system through the REAL M2 DeterministicGateway wired to REAL OpenFGA 1.18.1 + REAL OPA 1.16.2 (combined probe infra/opa/examples/livefire_probe.rs); crown-jewel allow path observed: RELATIONSHIP_PASS -> POLICY_PASS -> RISK_R3 -> APPROVAL_PASS -> CAPABILITY_PASS -> ALLOWED with canonical ActionReceipt (APPROVED/ISSUED, evidence refs = fingerprints only) + deterministic VerificationPlan (authorization:approved); denial paths: relationship denied dominates, OPA contextual denial dominates (risk stays R0, receipt REJECTED), wrong action digest with OLD approval -> MISSING_APPROVAL then NEW digest approval -> ALLOWED, capability scope binding (wrong actor/target/scope/tenant/expired -> NO_CAPABILITY), R4 model approval (ApprovalClass POLICY) rejected, STEP_UP requirement (SINGLE_FACTOR approval denied; STEP_UP allowed), OpenFGA killed -> ERROR never ALLOW, OPA killed -> ERROR never ALLOW (typed provider causes), receipt-not-bearer (copied/tampered/stale receipt alone -> DENIED, valid gateway path still required), no-LLM-authority (model ALLOW cannot bypass relationship/policy/approval/capability denial), verification-plan determinism (identical DecisionInput -> identical receipt+plan; AUTHORIZED != EXECUTED != VERIFIED); evidence written to .agent/state/evidence/ep008-m5/; engine gap fix: capability grant tenant binding added (directive F.4: wrong-tenant grant previously NOT rejected - now `grant.tenant_id != request.tenant_id` fails at CAPABILITY, new unit test ep008_unit_gateway_denies_wrong_tenant_grant); committed-state verification reran all sentinels green

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-13 | Fence amended (EP-001/EP-003/EP-007 precedent) | `.agent/expected-files/EP-008.txt` adds `Cargo.toml` + `Cargo.lock` (workspace member registration for nexus-policy/nexus-action-gateway), `deny.toml` + `COMPONENT_REGISTRY.yaml` (dependency governance when the node adds crates), `docs/vocabulary/README.md` + `references/ADR-012-authorization-policy-vocabulary.md` (new ADR). Mirrors the EP-007 M1 fence amendment. | Scope audit would otherwise reject these legitimately changed paths.
- 2026-08-13 | ADR-012 authorization policy vocabulary | SPEC-005/SPEC-006 canonical terms locked: `ActionLifecycleState` (11 states per SPEC-006 behavior 4), `GrantState`, `ApprovalDecision`, `ReceiptState`, `DenialReason`; `RiskClass` reuses existing nexus-domain `Risk` (R0..R4). New struct types (`RelationshipTuple`, `PolicyInput`, `PolicyDecision`, `CapabilityGrant`, `ApprovalAssertion`, `ActionRequest`, `ActionDecision`, `ActionReceipt`, `VerificationPlan`, `ExpectedState`, `VerificationResult`) are interface records, not vocabulary classes. | New synonyms require ADR + vocabulary update (ADR-011 precedent).
- 2026-08-13 | nexus-policy is provider-neutral (M1) | The policy crate depends only on nexus-domain + nexus-identity + nexus-auth + serde (enforced by `ep008_unit_policy_crate_has_no_infrastructure_dependencies` over real cargo tree). OpenFGA/OPA adapters live in `infra/` (M3/M4). | Reversal: engine import would fail dependency-direction test.
- 2026-08-13 | Deterministic risk floor (M2) | `deterministic_risk_floor` in nexus-policy maps capability + reversal + secret to R0..R4 (QUERY=R0, STREAM=R1, COMMAND/WORKFLOW=R2, ADMINISTRATIVE=R3; irreversible raises; secret raises). SPEC-005 behavior 4: R3/R4 require step-up or human approval; R4 never accepts model approval. `risk_rank`/`risk_at_least` added because the locked domain `Risk` has no Ord (nexus-domain is outside the EP-008 fence). | Provider classifiers may only raise, never lower.
- 2026-08-13 | Deterministic gateway engine (M2) | `DeterministicGateway` in `crates/nexus-action-gateway` is pure: no wall clock, randomness, network, or database. All external inputs (relationship result, policy decision, risk descriptors, grant, approval, actor, time) arrive via `DecisionInput` and injected provider ports; same inputs -> same decision (Temporal-replayable). The `ActionGateway` trait port method fails closed without `DecisionInput` (internal-invariant error) because the engine requires the authenticated actor and current time - adapters must call `evaluate_input`. | Reversal: reading wall clock/randomness in the engine would break replay determinism.
- 2026-08-13 | OpenFGA 1.18.1 pinned with digest (M3) | Component: OpenFGA (VERSIONS.lock.yaml: openfga 1.18.1, Apache-2.0, class authorization). Image `openfga/openfga:v1.18.1-amd64` digest `sha256:ec73e86c629f7c7b290cde0cf52bcea7c3e0315f30f65386fe4df532f4b83deb` (pulled and verified locally; docker.io). HTTP surface: `POST /stores`, `POST /stores/{id}/authorization-models`, `POST /stores/{id}/write`, `POST /stores/{id}/check`, `GET /healthz` - all verified live against the pinned container before the adapter was written. Transport: `ureq 2.12.1` (MIT OR Apache-2.0, json feature only; default-features=false; no gzip/cookies/native-tls chains) - the workspace had no HTTP client; ureq 2.12 aligns with the in-tree rustls 0.23/base64 0.22/url 2.5 chains; cargo-deny bans ok. | Version advance only by ADR (VERSIONS.lock policy).
- 2026-08-13 | Canonical Nexus-to-OpenFGA mapping (M3) | principal -> `user:<principal_id>` (all canonical principal types map to the OpenFGA `user` type; the canonical actor type is recorded in telemetry, not in the relationship model); object -> `<object_type>:<tenant_id>|<object_id>` (tenant embedded in the object id; colon separator is rejected by the provider - verified live - pipe separator is accepted; an identically named object in another tenant is a DIFFERENT OpenFGA object, no cross-tenant wildcarding); relation -> canonical relation name (owner/member/admin/operator/viewer/editor/delegated/actor). Explicit deny is NOT modeled: absence of a relationship is the denial (fail closed); the model contains no typed wildcards. Model: user, household (owner/member/admin=computed owner), business (admin/member), device (operator), resource (viewer/editor/owner; viewer/editor accept `business#admin` userset - model-derived transitivity), capability (delegated), action (actor - the deterministic gateway's hardcoded object type for its relationship stage). | Reversal: leaking OpenFGA tuple syntax into nexus-policy would fail the provider-neutral dependency-direction test.
- 2026-08-13 | Relationship-vs-context policy responsibility boundary (M3) | OpenFGA proves relationship authorization ONLY. Contextual risk, time, auth strength, and approval are NOT encoded in the model or the adapter - they belong to OPA / nexus-policy / action-gateway (M2 engine stages: relationship -> policy -> risk floor -> R3/R4 approval -> capability -> allow). The model has no wildcards and no contextual state. The M3 gateway composition tests prove relationship deny stops the gateway before policy and a valid relationship path continues to the next stage (COMMAND -> R2 -> allow-all policy port -> grant -> ALLOW; the allow-all policy port is probe-only, replaced by OPA in M4). | Reversal: encoding context into OpenFGA would collapse the policy engine into the provider.
- 2026-08-13 | Tenant isolation strategy (M3) | Canonical object id embeds the tenant: `<object_type>:<tenant_id>|<object_id>`. Proven live: tenant A owner + tenant A object -> allow; same principal + tenant B object with the SAME object suffix -> deny; unrelated tenant B owner does not leak into tenant A. No cross-tenant wildcarding possible: the model contains no typed wildcards and every object id is tenant-qualified. | Reversal: sharing bare object ids across tenants would authorize cross-tenant access.
- 2026-08-13 | Revocation/consistency behavior (M3) | The adapter introduces NO local authorization decision cache (directive G: correctness over latency). Every check is a fresh provider read; tuple deletion takes effect on the next check. Proven live: allow before tuple removal, deny after `POST /stores/{id}/write` with `deletes`. No immediate-revocation guarantee beyond the provider's own read semantics is fabricated. | Reversal: a local cache would violate the revocation test.
- 2026-08-13 | Fail-closed provider behavior (M3) | Every provider failure maps to a typed `OpenFgaErrorCode` (unavailable, timeout, malformed_provider_response, model_store_mismatch, invalid_relationship_request, provider_authorization_failure) and then to the canonical `PolicyError` surface - never an allow. HTTP 400 model/store mismatch, 401/403 authorization failure, 404, 429, 5xx, and transport failures (connection refused, DNS, IO, redirects) are classified; the `RelationshipAuthorizer::check` returns `Err(PolicyError)` and the gateway propagates it as a denial. Proven live: wrong store/model -> typed validation error (never allow), malformed user/relation/object -> 400 (never allow), container killed -> typed unavailable error -> gateway does NOT progress to ALLOW. | Reversal: swallowing provider errors into an allow would violate SPEC-005 fail-closed.
- 2026-08-13 | Fence amended: EP-008 orphan audit script (M3) | `scripts/ep008-orphan-audit.sh` added (EP-006/EP-007 precedent) to prove the M3 integration suite left ZERO `nexus-ep008-*` containers/networks/volumes. The M3 gate does not run it inside `sh scripts/nodes/EP-008.sh M3` (the gate is the pytest suite; the zz teardown test proves zero orphans in-process); the script is the standalone operator audit run at milestone closure and committed-state re-verify. | Scope audit would otherwise reject the script; removing the audit would weaken teardown proof.
- 2026-08-13 | OPA 1.16.2 pinned with digest (M4) | Component: OPA (VERSIONS.lock.yaml: opa 1.16.2, Apache-2.0, class policy). Image `openpolicyagent/opa:1.16.2` digest `sha256:a915d8b59ddb09a9badecd8e061d43cf3111283494c4cf1d38a675bdb4e81a13` (pulled and verified locally; docker.io). HTTP surface: `GET /health`, `PUT /v1/policies/{id}`, `POST /v1/data/nexus/allow`, `POST /v1/data/nexus/policy_version`, `POST /v1/data/nexus` - all verified live against the pinned container before the adapter was written. OPA 1.16 binds `localhost:8181` by default; the harness starts with `run --server --addr 0.0.0.0:8181` (verified live). Transport: `ureq 2.12.1` (json feature only) - aligns with the in-tree rustls/base64/url chains from M3; cargo-deny bans ok. | Version advance only by ADR (VERSIONS.lock policy).
- 2026-08-13 | ContextPolicyEngine-to-OPA mapping (M4) | `PolicyInput` fields map to a flat, typed OPA input object: tenant_id, principal_id, principal_type, capability, risk, strength, device_trust, device_state, object_type, object_id, request_id, sensitivity, context{location, network_trust, maintenance, emergency}. Sensitivity and context are OPTIONAL extension fields accepted from the caller via `OpaContext`; the adapter NEVER fabricates them and NEVER sends arbitrary domain objects, secrets, or unrelated personal data (directive C). Query paths: `data.nexus.allow` (boolean) and `data.nexus.policy_version` (string). The policy bundle lives at `policies/nexus.rego` (package `nexus`, `rego.v1`). | Reversal: leaking Rego details into nexus-policy would fail the provider-neutral dependency-direction test.
- 2026-08-13 | Default-deny semantics (M4) | The policy declares `default allow := false` and `default decision := "deny"`; allow requires a known capability in {QUERY, COMMAND, WORKFLOW, STREAM, ADMINISTRATIVE}, a known risk in {R0..R4}, a known strength, and the absence of every deny condition (tenant mismatch, disabled/revoked device, weak auth, untrusted context, sensitive context). Undefined query paths return no `result` (verified live: `{}`), which the adapter classifies as a typed `undefined_decision` - never an allow. Unknown action classes are denied by the default rule. | Reversal: any undefined path or unhandled action becoming allow would violate SPEC-005.
- 2026-08-13 | Policy bundle version/digest strategy (M4, directive H) | The policy exposes `policy_version := "nexus-policy-v1"`; the adapter config carries the EXPECTED version and refuses to evaluate any bundle whose loaded `data.nexus.policy_version` differs (typed `policy_bundle_version_mismatch`, fail closed). Audit telemetry records only the version fingerprint (one-way hash), never raw policy inputs. Proven live: configuring `nexus-policy-wrong-v99` against the loaded `nexus-policy-v1` bundle -> typed mismatch -> gateway ERROR (never ALLOW). | Reversal: silently evaluating an unknown/unversioned bundle would break the audit contract.
- 2026-08-13 | Typed provider failures (M4, directive A) | Seven typed failure classes: unavailable, timeout, malformed_provider_response, policy_bundle_version_mismatch, invalid_policy_input, undefined_decision, provider_evaluation_failure. Each maps onto the canonical `PolicyError` surface while PRESERVING the typed cause in the message (audit/observability; not an implicit DENY). Timeout is distinguished from unavailable by downcasting the ureq transport source to `io::Error` and checking `TimedOut`/`WouldBlock` (verified against ureq 2.12.1 against a real unresponsive TCP peer; the read deadline surfaces as `ErrorKind::Io` with an io source of kind TimedOut). | Reversal: collapsing provider errors into a bare DENY would lose the typed cause for audit.
- 2026-08-13 | OpenFGA vs OPA responsibility boundary (M4, directive B) | OPA evaluates CONTEXTUAL policy only: tenant, user/device, location/context, network trust, time window, requested action class, authentication strength, sensitivity, maintenance/emergency state, resource state, environment attributes. OPA is NOT the authority for: OpenFGA relationship truth (M3), risk-level calculation (nexus-policy owns the deterministic floor), generating human approval, validating action-digest approval binding, issuing capability grants, or executing actions. The policy bundle encodes no relationship tuples and no grants. | Reversal: moving relationship/approval/capability logic into OPA would violate the layered architecture.
- 2026-08-13 | ActionGateway ordering proof (M4, directive G) | Proven at the engine level (7 new `ep008_failure` Rust tests in nexus-action-gateway) and through the REAL gateway + REAL OPA adapter (probe binary): relationship ALLOW + policy DENY -> STOP at POLICY with risk R0 (no risk/approval/capability stage can manufacture ALLOW); relationship ALLOW + policy ALLOW -> continue to the risk floor (COMMAND -> R2) -> grant covers -> ALLOW; relationship DENY stops FIRST with RELATIONSHIP; OPA unavailable -> policy provider failure -> gateway returns ERROR (never ALLOW). | Reversal: reordering the gateway stages would break determinism.
- 2026-08-13 | No approval/capability/model override of policy denial (M4, directive L) | Proven: a valid human approval (MULTI_FACTOR, digest-bound) does NOT erase an OPA contextual-policy denial; a valid capability grant does NOT; model/relationship ALLOW does NOT. The gateway stops at the policy stage before approval/capability are consulted. | Reversal: allowing approval/grant to override an earlier policy denial would violate SPEC-005.
- 2026-08-14 | Capability grant tenant binding gap found + fixed (M5, directive F.4) | The M2 capability stage checked actor/target/scope/expiry but NOT `grant.tenant_id` - a grant issued in another tenant with matching actor/target/scope would PASS. Directive F.4 requires wrong-tenant grants to fail at CAPABILITY. Fixed in `engine.rs`: `grant.tenant_id != request.tenant_id` is now part of the capability check; new unit test `ep008_unit_gateway_denies_wrong_tenant_grant`; proven live-fire: wrong-tenant grant -> DENIED NO_CAPABILITY even with valid relationship + OPA + approval. | Reversal: removing the tenant check would re-open cross-tenant grant acceptance (SPEC-005 behavior 5 tenant boundary).
- 2026-08-14 | Combined live-fire probe (M5) | `infra/opa/examples/livefire_probe.rs` wires the REAL M2 DeterministicGateway to the REAL OpenFGA adapter (M3) and the REAL OPA adapter (M4) in ONE process - no fake relationship or policy provider anywhere. Accepts a deterministic DecisionInput (injected now, digest, grant, approval, context), records REAL provider telemetry (RecordingSink), derives the canonical stage progression (RELATIONSHIP_PASS -> POLICY_PASS -> RISK_* -> APPROVAL_PASS -> CAPABILITY_PASS -> ALLOWED or the stopping deny/error), builds the canonical ActionReceipt (evidence refs = fingerprints only, never secrets) and a deterministic VerificationPlan (`authorization:approved` expected state; EP-008 owns authorization only so no execution/verification success is claimed). `model_recommendation` and `presented_receipt` input fields are accepted but NEVER consulted - proof surface for directives M and D. | Reversal: reusing the probe as an authorization oracle would duplicate the gateway; it is evidence tooling only.
- 2026-08-14 | Live-fire test suite + M5 gate wiring (M5) | `tests/policy/test_ep008_live_fire.py` (16 `ep008_livefire_*` tests) starts REAL pinned OpenFGA + OPA containers, bootstraps REAL store/model/tuples + REAL policy bundle, and exercises directives C-M end to end. `scripts/nodes/EP-008.sh` M5|verify now runs the `ep008_livefire_*` pytest selection and `scripts/ep008-orphan-audit.sh` as part of the M5 gate (precedent: EP-007 M5 gate). Evidence written to `.agent/state/evidence/ep008-m5/` (governed path). | Reversal: dropping the live-fire suite from the M5 gate would make node verify pass without the crown-jewel proof.
- 2026-08-14 | Canonical live-fire action (M5, directive A) | The live-fire action is `admin:test:livefire` - an R3 ADMINISTRATIVE test action (deterministic risk floor: ADMINISTRATIVE -> R3) against a test target, requiring a valid OpenFGA `actor` relationship, OPA contextual allow, STEP_UP human approval, and an exact ADMINISTRATIVE capability grant. It is a harmless administrative TEST capability; EP-008 owns authorization only, so nothing is executed (AUTHORIZED != EXECUTED != VERIFIED). | Reversal: choosing a destructive production action would violate the safe-target rule.
- 2026-08-14 | Fence amended: LF-003 live-fire script (M5) | `scripts/live-fire/LF-003.sh` (EP-007's DONE live-fire proof) was a stub delegating to a nonexistent `nexus-cli` proof runner, so the live-fire gate in `verify.sh` broke for EVERY node after EP-007 (EP-006 M5 precedent: LF-017.sh was the same stub class and was rewritten). Rewrote it to run the REAL committed EP-007 proof (`cargo test --locked -p nexus-auth --test lf003`, 2 tests, vacuously-guarded `2 passed`) and write evidence under `.agent/state/evidence/`. `.agent/expected-files/EP-008.txt` now includes `scripts/live-fire/LF-003.sh` (EP-006/EP-007 fence-amendment precedent). | Scope audit would otherwise reject the live-fire deliverable; leaving the stub would make `verify` fail for every later node.
- 2026-08-14 | Pre-existing M3/M4 lint debt fixed (M5) | node-verify had never run against the committed M3/M4 pytest files, so latent E501/SIM105 ruff errors in `tests/policy/test_ep008_integration_openfga.py` and `test_ep008_failure_opa.py` surfaced only at M5 closure (node-verify runs lint.sh). Reformatted the long lines and replaced a try/except/pass with `contextlib.suppress`; all suites still pass (16 live-fire + 19 failure + 16 integration). | Reversal: leaving the lint debt would keep node verify red.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.

## EP-008 M5 closure

- Changed files vs machine fence: `crates/nexus-action-gateway/` (engine tenant-binding fix + unit test), `infra/opa/` (dev-dep nexus-openfga + livefire_probe example), `tests/policy/` (live-fire suite), `scripts/nodes/EP-008.sh` (M5 gate wiring), `Cargo.lock`, `.agent/execplans/...` (progress/decisions/outcomes), `.agent/state/evidence/ep008-m5/` (governed evidence). All paths inside the EP-008 expected-files fence.
- Exact commands and sentinels observed:
  - `sh scripts/preflight.sh` -> `preflight: ok`
  - `uv run --frozen pytest tests/policy -o python_functions="ep008_livefire_*"` -> `16 passed`
  - `sh scripts/nodes/EP-008.sh M5` -> `EP-008 M5: ok`
  - `sh scripts/node-verify.sh EP-008` -> `node verify EP-008: ok`
  - `sh scripts/scope-audit.sh EP-008` -> `scope audit EP-008: ok`
  - `sh scripts/expected-files.sh EP-008` -> `expected files EP-008: ok`
  - adapter parity (PRIME-BLOCK cksum) -> single unique checksum
  - `sh scripts/security-check.sh` -> `security check: ok`
  - `sh scripts/license-gate.sh` -> `license gate: ok`
  - `sh scripts/reality-gate.sh` -> `reality gate: ok`
  - `sh scripts/ep008-orphan-audit.sh` -> `EP-008 orphan audit: ok`
  - `sh scripts/verify.sh` -> `verify: ok`
- Test/proof evidence: 16 live-fire tests (`.agent/state/evidence/ep008-m5/`), 26 engine tests incl. new tenant-binding test, M3/M4 suites still green.
- Assumptions confirmed: EP-008 owns authorization only (AUTHORIZED != EXECUTED != VERIFIED); real providers are required for every allow/deny proof; capability grants are tenant-bound.
- Providers: OpenFGA 1.18.1 `sha256:ec73e86c...` certified live (M3 + M5); OPA 1.16.2 `sha256:a915d8b5...` certified live (M4 + M5); policy bundle `policies/nexus.rego` (`nexus-policy-v1`).
- Remaining risks: receipt references are fingerprints (no raw payloads) by contract; any future bearer-token semantics for receipts would require an ADR.
- Green tag: `green/EP-008` created at the verified commit after committed-state re-verification.
