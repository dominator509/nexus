NODE-META-BEGIN
ID: EP-030
DEPS: EP-029
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-030
VERIFY_SENTINEL: node verify EP-030: ok
GREEN_TAG: green/EP-030
NODE-META-END

# 1. Purpose / Big Picture

Implement OPNsense and OpenWrt adapters, AdGuard Home, inventory, segmentation, baselines, anomaly scoring, and quarantine proposals. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-030.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-030.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-029` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-030.md`
- `.agent/specs/SPEC-013-sentinel-firewall-dns-network-detection-and-endpoint-security.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-030.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-030-sentinel-core-network-and-dns.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-030.txt`
- `.agent/node-contracts/EP-030.md`
- `scripts/nodes/EP-030.sh`
- `crates/nexus-sentinel/`
- `connectors/opnsense/`
- `connectors/openwrt/`
- `connectors/adguard-home/`
- `infra/sentinel/core/`
- `tests/sentinel/core/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `FirewallProvider` | `nexus-sentinel` | Defined by EP-030; provider-neutral and versioned |
| `DnsSecurityProvider` | `nexus-sentinel` | Defined by EP-030; provider-neutral and versioned |
| `NetworkInventory` | `nexus-sentinel` | Defined by EP-030; provider-neutral and versioned |
| `DeviceFingerprint` | `nexus-sentinel` | Defined by EP-030; provider-neutral and versioned |
| `BehaviorBaseline` | `nexus-sentinel` | Defined by EP-030; provider-neutral and versioned |
| `NetworkFinding` | `nexus-sentinel` | Defined by EP-030; provider-neutral and versioned |
| `QuarantineProposal` | `nexus-sentinel` | Defined by EP-030; provider-neutral and versioned |

Acceptance obligations:

1. OPNsense and OpenWrt share a canonical network provider
2. AdGuard Home supplies DNS security and telemetry
3. IoT, trusted, guest, camera, and quarantine segments are modeled
4. Core Sentinel is light enough for a normal home and can propose verified containment

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement opnsense and openwrt adapters, adguard home, inventory, segmentation, baselines, anomaly scoring, and quarantine proposals.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-030-M1.txt`, `.agent/node-contracts/EP-030.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-030-sentinel-core-network-and-dns.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-030.txt`, `.agent/node-contracts/EP-030.md`, `scripts/nodes/EP-030.sh`, `crates/nexus-sentinel/`, `tests/sentinel/core/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep030_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-030.sh M1`

EXPECT:

- `EP-030 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-030 MILESTONE_PASS "M1 EP-030 M1: ok"`

FALLBACK: Operate in observe-only mode on unsupported routers while retaining DNS and endpoint signals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-030][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-030.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-030-M2.txt`, `.agent/node-contracts/EP-030.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/opnsense/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep030_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-030.sh M2`

EXPECT:

- `EP-030 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-030 MILESTONE_PASS "M2 EP-030 M2: ok"`

FALLBACK: Operate in observe-only mode on unsupported routers while retaining DNS and endpoint signals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-030][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-030 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-030-M3.txt`, `.agent/node-contracts/EP-030.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/openwrt/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep030_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-030.sh M3`

EXPECT:

- `EP-030 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-030 MILESTONE_PASS "M3 EP-030 M3: ok"`

FALLBACK: Operate in observe-only mode on unsupported routers while retaining DNS and endpoint signals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-030][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-030 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-030-M4.txt`, `.agent/node-contracts/EP-030.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/adguard-home/`

CONTENT:

1. Create tests whose names begin `ep030_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-030.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-030 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-030 MILESTONE_PASS "M4 EP-030 M4: ok"`

FALLBACK: Operate in observe-only mode on unsupported routers while retaining DNS and endpoint signals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-030][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-030.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-030-M5.txt`, `.agent/node-contracts/EP-030.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/sentinel/core/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-030` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-030.sh M5`
2. `sh scripts/node-verify.sh EP-030`
3. `sh scripts/scope-audit.sh EP-030`

EXPECT:

- `EP-030 M5: ok`
- `node verify EP-030: ok`
- `scope audit EP-030: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-030 MILESTONE_PASS "M5 EP-030 M5: ok"`

FALLBACK: Operate in observe-only mode on unsupported routers while retaining DNS and endpoint signals. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-030][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-030` and observe `node verify EP-030: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-010` `network-diagnosis`: Diagnose a controlled DNS or Wi-Fi fault from OPNsense or OpenWrt and AdGuard telemetry, explain evidence, and propose a reversible fix.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-20 (M1): tests/sentinel/core is one directory deeper than tests/social; path deps must be `../../../crates/...` not `../../crates/...` (cargo manifest load failure observed and fixed).
- 2026-08-20 (M1): gate display count pattern must be `ok\.` not `ok\\.` in grep -E through sh (double-escaped backslash matches a literal backslash, producing an empty count; vacuity guards unaffected).
- 2026-08-20 (M2): QuarantineProposal has no `with_state` builder in the M1 contract (fields are public by design); the adapter uses struct-update syntax `QuarantineProposal { state: ..., ..proposal }` instead of adding a builder to the contract crate (M2 fence forbids contract-crate changes).
- 2026-08-20 (M2): The documented searchRule response normalizes `enabled` as string "1"/"0" in the docs example, but OpnsenseRule's serde contract is bool; the production transport normalizes via a raw SearchRow with `Option<serde_json::Value>`. The unit test must feed the struct's own serde shape (bool), not the raw wire string.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-20 (M1): Contract vocabulary mirrors the established typed-id + vocabulary-enum pattern (nexus-social/nexus-hydra precedent) but with SPEC-013-owned names only (Sentinel, DeviceFingerprint, Baseline, Quarantine, OPNsense, OpenWrt, AdGuard). Nexus-wide ids (TenantId, DeviceId, IncidentId, ApprovalId) and ApprovalClass are imported from nexus-domain, never redefined. Evidence: `ep030_unit_typed_ids_validate_and_reject`, `ep030_unit_segments_model_all_five_classes`. Alternatives: redefining TenantId/ApprovalClass locally (rejected: violates dependency-direction/vocabulary-lock), heavy SOC vocabulary (rejected: SPEC-013 non-goal). Consequence: EP-030 owns only sentinel-specific vocabulary; all five segments modeled. Reversal: requires ADR + schema update.
- 2026-08-20 (M1): The seven node-contract public interfaces are split into provider ports (FirewallProvider, DnsSecurityProvider, NetworkInventory) + value objects (DeviceFingerprint, BehaviorBaseline, NetworkFinding, QuarantineProposal), matching the provider-neutral port pattern from EP-029/EP-024. OPNsense and OpenWrt share FirewallProvider (acceptance obligation 1); AdGuard Home maps to DnsSecurityProvider (obligation 2); segments are modeled in vocabulary (obligation 3); QuarantineProposal with the PROPOSED->APPROVED->APPLIED->VERIFIED ladder and is_auto_applicable() (preauthorized + reversible) encodes obligation 4 verified containment. Evidence: `ep030_unit_opnsense_and_openwrt_share_canonical_provider`, `ep030_unit_core_sentinel_proposes_verified_containment`. Alternatives: separate OPNsense/OpenWrt traits (rejected: obligation 1 requires shared canonical provider), proposal-as-executed-rule (rejected: SPEC-013 behavior 5/6). Consequence: contract encodes fail-closed containment; unbound providers never fabricate. Reversal: requires ADR + schema update.
- 2026-08-20 (M2): OPNsense adapter transport is built ONLY from the DOCUMENTED firewall automation API (docs.opnsense.org/development/api/core/firewall.html + OPNsense core source FilterController.php/Filter.xml): GET searchRule?current=1&rowCount=N&searchPhrase=S, POST addRule {"rule": {...}}, POST toggleRule/{uuid}/{enabled}, POST apply; HTTP Basic auth with API key/secret; rule fields action pass|block|reject, direction in|out|any, ipprotocol inet|inet6|inet46, protocol, source_net, destination_net, description. Containment payload = action block, direction any, ipprotocol inet46, protocol any, source_net = device label, destination_net any. Evidence: `ep030_unit_transport_containment_payload_is_block_both_directions`, `ep030_unit_transport_status_classification`. Alternatives: invented endpoints (rejected: anti-hallucination), pfSense/iptables surface (rejected: wrong provider). Consequence: containment is a reversible OPNsense automation rule applied via documented endpoints. Reversal: requires upstream API change.
- 2026-08-20 (M2): Dual-gate containment: apply_containment requires (1) proposal state APPROVED and (2) is_auto_applicable() (preauthorized AND reversible) BEFORE any transport call; revoke requires APPLIED state with rule_ref; verification binds the exact proposal/device to the observed rule by rule_ref + enabled + action==block readback. Zero provider calls on denial (AtomicUsize proofs); in-flight idempotency with release-after-end (real concurrent duplicate -> Conflict, retry after completion not Conflict). Evidence: `ep030_unit_apply_requires_approved_state_zero_calls_on_denial`, `ep030_unit_verify_binds_exact_rule_and_device`, `ep030_unit_inflight_duplicate_is_conflict_and_release_after_end`. Alternatives: apply-without-approval (rejected: behavior 5/6), verification by description alone (rejected: not exact-target). Consequence: automated containment limited to preauthorized reversible approved rules; never destructive. Reversal: requires changing the adapter contract.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
