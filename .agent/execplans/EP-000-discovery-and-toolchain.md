NODE-META-BEGIN
ID: EP-000
DEPS: -
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-000
VERIFY_SENTINEL: node verify EP-000: ok
GREEN_TAG: green/EP-000
NODE-META-END

# 1. Purpose / Big Picture

Discovery, source verification, toolchain lock, license baseline, and truthful command surface. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-000.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-000.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `-` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-000.md`
- `.agent/specs/SPEC-000-product-scope-and-constitutional-priorities.md`
- `.agent/specs/SPEC-019-licensing-sbom-provenance-and-supply-chain-security.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-000.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-000-discovery-and-toolchain.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-000.txt`
- `.agent/node-contracts/EP-000.md`
- `scripts/nodes/EP-000.sh`
- `references/`
- `infra/devcontainer/`
- `scripts/source-verify.sh`
- `scripts/version-verify.sh`
- `rust-toolchain.toml`
- `.tool-versions`
- `mise.toml`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `VerifiedSourceRecord with URL, authoritative owner, version, release date, license, checksum or digest, retrieval date, and decision status` | `repository control` | Defined by EP-000; provider-neutral and versioned |
| `ToolchainLock with exact versions and a containerized development fallback` | `repository control` | Defined by EP-000; provider-neutral and versioned |
| `ComponentDecision with self-hosted default, replacement contract, license mode, security posture, and source evidence` | `repository control` | Defined by EP-000; provider-neutral and versioned |

Acceptance obligations:

1. Every locked version and source has authoritative evidence
2. Every selected component has a commercial integration mode and replacement boundary
3. The development toolchain can be reproduced without an unpinned latest reference
4. Every command in COMMANDS.md has a truthful implementation or is owned by a later node

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for discovery, source verification, toolchain lock, license baseline, and truthful command surface.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-000-M1.txt`, `.agent/node-contracts/EP-000.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-000-discovery-and-toolchain.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-000.txt`, `.agent/node-contracts/EP-000.md`, `scripts/nodes/EP-000.sh`, `references/`, `.tool-versions`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep000_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-000.sh M1`

EXPECT:

- `EP-000 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-000 MILESTONE_PASS "M1 EP-000 M1: ok"`

FALLBACK: Use the pinned devcontainer as the canonical toolchain when the host cannot install a locked compiler or SDK. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-000][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-000.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-000-M2.txt`, `.agent/node-contracts/EP-000.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/devcontainer/`, `mise.toml`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep000_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-000.sh M2`

EXPECT:

- `EP-000 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-000 MILESTONE_PASS "M2 EP-000 M2: ok"`

FALLBACK: Use the pinned devcontainer as the canonical toolchain when the host cannot install a locked compiler or SDK. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-000][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-000 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-000-M3.txt`, `.agent/node-contracts/EP-000.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `scripts/source-verify.sh`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep000_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-000.sh M3`

EXPECT:

- `EP-000 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-000 MILESTONE_PASS "M3 EP-000 M3: ok"`

FALLBACK: Use the pinned devcontainer as the canonical toolchain when the host cannot install a locked compiler or SDK. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-000][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-000 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-000-M4.txt`, `.agent/node-contracts/EP-000.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `scripts/version-verify.sh`

CONTENT:

1. Create tests whose names begin `ep000_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-000.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-000 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-000 MILESTONE_PASS "M4 EP-000 M4: ok"`

FALLBACK: Use the pinned devcontainer as the canonical toolchain when the host cannot install a locked compiler or SDK. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-000][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-000.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-000-M5.txt`, `.agent/node-contracts/EP-000.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `rust-toolchain.toml`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-000` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-000.sh M5`
2. `sh scripts/node-verify.sh EP-000`
3. `sh scripts/scope-audit.sh EP-000`

EXPECT:

- `EP-000 M5: ok`
- `node verify EP-000: ok`
- `scope audit EP-000: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-000 MILESTONE_PASS "M5 EP-000 M5: ok"`

FALLBACK: Use the pinned devcontainer as the canonical toolchain when the host cannot install a locked compiler or SDK. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-000][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-000` and observe `node verify EP-000: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

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

# 12. Surprises & Discoveries

- 2026-08-12: The pack's own L5 gates do not pass on the unmodified pack. Three
  defects found and fixed with ADRs: (1) security-check.sh regex `sk-[A-Za-z0-9_-]{24,}`
  false-positives on the canonical EP-025 artifact name `EP-025-asterisk-telephony-and-ai-calling.md`
  (ADR-001); (2) COMPONENT_REGISTRY.yaml lacked `replacement_contract:` and
  `commercial_review:` fields required by license_validate.py (ADR-002); (3)
  LICENSE_POLICY.md lacked the literal token `copyleft` required by license_validate.py
  even though the SIDECAR class is copyleft policy in substance (ADR-003).
- 2026-08-12: Upstream mappings in VERSIONS.lock.yaml needed correction to reach
  authoritative sources: temporal-typescript-sdk v1.17.2 has no GitHub release
  (npm shasum verified instead), glitchtip releases are Docker Hub images (6.1.8
  confirmed), ictfax is ictinnovations/ictfax, a2a-js-sdk is a2aproject/a2a-js,
  docker-engine tags are `docker-v29.1.x`, kokoro lock version 0.19 refers to the
  model lineage while pyproject reports 0.9.4 (recorded for ADR review).
- 2026-08-12: The host toolchain is older than the lock (rust 1.96.0 vs 1.97.1,
  node 24.16.0 vs 24.18.0, etc.). The devcontainer is the canonical fallback;
  mise (the repo's declared toolchain manager) is being used to provision locked
  versions on the host for verification parity.

# 13. Decision Log

- 2026-08-12 | ADR-001 | Fix security gate regex precision with left word boundary
  instead of allowlisting Asterisk-named files. Evidence: gate reproduced failing,
  fixed, negative control still catches real secrets. Alternative: allowlist (rejected).
  Reversal: revert one-line regex. Security: gate now catches only true secrets.
- 2026-08-12 | ADR-002 | Complete COMPONENT_REGISTRY.yaml with real
  replacement_contract and commercial_review values for all 26 components,
  grounded in VERSIONS.lock.yaml + SOURCE_REGISTRY.md + SOURCE_VERIFICATION.json.
  Alternative: weaken gate (rejected). Reversal: revert registry diff + fence line.
- 2026-08-12 | ADR-003 | Add `copyleft` token to LICENSE_POLICY.md SIDECAR class
  description. Documentation-only vocabulary alignment; substance unchanged.
  Reversal: revert one-line edit.
- 2026-08-12 | Source evidence collector preserves original retrieval_date on
  re-runs so the evidence file is deterministic and idempotent (first retrieval
  is authoritative). Failure test enforces byte-identical re-runs.
- 2026-08-12 | .tool-versions uses mise tool names; `tofu` maps to lock component
  `opentofu`. Alias recorded in the unit test and verified.

# 14. Outcomes & Retrospective

- M1: 7 unit tests, 50/50 source records verified. Commit 5c91e3f.
- M2: devcontainer + mise.toml. Commit 8127fb1.
- M3: source-verify.sh + 3 live integration tests against upstreams. Commit 9821b91.
- M4 in progress: failure tests, security + license gate fixes (ADR-001..003), version-verify.sh.
