NODE-META-BEGIN
ID: EP-040
DEPS: EP-039
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-040
VERIFY_SENTINEL: node verify EP-040: ok
GREEN_TAG: green/EP-040
NODE-META-END

# 1. Purpose / Big Picture

Complete contract, integration, E2E, security, accessibility, performance, chaos, provider certification, hardware lab, and flaky-test elimination. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-040.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-040.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-039` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-040.md`
- `.agent/specs/SPEC-008-production-readiness-certification-and-ship-standard.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-040.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-040-testing-hardening-and-chaos.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-040.txt`
- `.agent/node-contracts/EP-040.md`
- `scripts/nodes/EP-040.sh`
- `tests/contract/`
- `tests/integration/`
- `tests/e2e/`
- `tests/security/`
- `tests/chaos/`
- `tests/performance/`
- `tests/accessibility/`
- `tests/provider-certification/`
- `tests/hardware/`
- `.github/workflows/nightly.yml`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `TestMatrix` | `all` | Defined by EP-040; provider-neutral and versioned |
| `ChaosScenario` | `all` | Defined by EP-040; provider-neutral and versioned |
| `ProviderCertificationSuite` | `all` | Defined by EP-040; provider-neutral and versioned |
| `HardwareCertificationSuite` | `all` | Defined by EP-040; provider-neutral and versioned |
| `PerformanceBudget` | `all` | Defined by EP-040; provider-neutral and versioned |
| `AccessibilityAudit` | `all` | Defined by EP-040; provider-neutral and versioned |
| `FlakyTestPolicy` | `all` | Defined by EP-040; provider-neutral and versioned |

Acceptance obligations:

1. Every spec behavior maps to a test path
2. Required failures fail CI instead of becoming informational
3. Provider and hardware certifications use real controlled dependencies
4. Verify passes three consecutive times and flaky behavior is eliminated

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for complete contract, integration, e2e, security, accessibility, performance, chaos, provider certification, hardware lab, and flaky-test elimination.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-040-M1.txt`, `.agent/node-contracts/EP-040.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-040-testing-hardening-and-chaos.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-040.txt`, `.agent/node-contracts/EP-040.md`, `scripts/nodes/EP-040.sh`, `tests/contract/`, `tests/performance/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep040_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-040.sh M1`

EXPECT:

- `EP-040 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-040 MILESTONE_PASS "M1 EP-040 M1: ok"`

FALLBACK: Reduce parallelism or use dedicated test hardware; never weaken the expected result. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-040][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-040.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-040-M2.txt`, `.agent/node-contracts/EP-040.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/integration/`, `tests/accessibility/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep040_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-040.sh M2`

EXPECT:

- `EP-040 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-040 MILESTONE_PASS "M2 EP-040 M2: ok"`

FALLBACK: Reduce parallelism or use dedicated test hardware; never weaken the expected result. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-040][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-040 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-040-M3.txt`, `.agent/node-contracts/EP-040.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/e2e/`, `tests/provider-certification/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep040_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-040.sh M3`

EXPECT:

- `EP-040 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-040 MILESTONE_PASS "M3 EP-040 M3: ok"`

FALLBACK: Reduce parallelism or use dedicated test hardware; never weaken the expected result. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-040][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-040 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-040-M4.txt`, `.agent/node-contracts/EP-040.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/security/`, `tests/hardware/`

CONTENT:

1. Create tests whose names begin `ep040_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-040.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-040 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-040 MILESTONE_PASS "M4 EP-040 M4: ok"`

FALLBACK: Reduce parallelism or use dedicated test hardware; never weaken the expected result. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-040][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-040.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-040-M5.txt`, `.agent/node-contracts/EP-040.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/chaos/`, `.github/workflows/nightly.yml`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-040` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-040.sh M5`
2. `sh scripts/node-verify.sh EP-040`
3. `sh scripts/scope-audit.sh EP-040`

EXPECT:

- `EP-040 M5: ok`
- `node verify EP-040: ok`
- `scope audit EP-040: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-040 MILESTONE_PASS "M5 EP-040 M5: ok"`

FALLBACK: Reduce parallelism or use dedicated test hardware; never weaken the expected result. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-040][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-040` and observe `node verify EP-040: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [ ] M1: Contract, vocabulary, and package boundary
- [ ] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
