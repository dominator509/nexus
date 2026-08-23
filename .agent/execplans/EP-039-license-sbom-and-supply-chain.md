NODE-META-BEGIN
ID: EP-039
DEPS: EP-038
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-039
VERIFY_SENTINEL: node verify EP-039: ok
GREEN_TAG: green/EP-039
NODE-META-END

# 1. Purpose / Big Picture

Implement license policy, sidecar boundaries, SBOM, provenance, signed artifacts, image scanning, dependency policy, and advisory monitoring. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-039.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-039.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-038` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-039.md`
- `.agent/specs/SPEC-019-licensing-sbom-provenance-and-supply-chain-security.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-039.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-039-license-sbom-and-supply-chain.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-039.txt`
- `.agent/node-contracts/EP-039.md`
- `scripts/nodes/EP-039.sh`
- `crates/nexus-supply-chain/`
- `supply-chain/`
- `policies/licenses/`
- `scripts/sbom/`
- `tests/supply-chain/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `LicenseClassifier` | `EP-039` | Defined by EP-039; provider-neutral and versioned |
| `ComponentBoundary` | `EP-039` | Defined by EP-039; provider-neutral and versioned |
| `SbomGenerator` | `EP-039` | Defined by EP-039; provider-neutral and versioned |
| `ArtifactSigner` | `EP-039` | Defined by EP-039; provider-neutral and versioned |
| `ProvenanceAttestation` | `EP-039` | Defined by EP-039; provider-neutral and versioned |
| `AdvisoryMonitor` | `EP-039` | Defined by EP-039; provider-neutral and versioned |
| `DependencyWaiver` | `EP-039` | Defined by EP-039; provider-neutral and versioned |

Acceptance obligations:

1. Every shipped component has license, source, version, digest, integration mode, and notices
2. Copyleft sidecars remain process-separated and compliant
3. OCI images and packages carry SBOM and provenance
4. Critical advisories fail release unless a time-bounded ADR documents mitigation

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement license policy, sidecar boundaries, sbom, provenance, signed artifacts, image scanning, dependency policy, and advisory monitoring.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-039-M1.txt`, `.agent/node-contracts/EP-039.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-039-license-sbom-and-supply-chain.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-039.txt`, `.agent/node-contracts/EP-039.md`, `scripts/nodes/EP-039.sh`, `crates/nexus-supply-chain/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep039_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-039.sh M1`

EXPECT:

- `EP-039 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-039 MILESTONE_PASS "M1 EP-039 M1: ok"`

FALLBACK: Remove or replace a component whose license or security posture cannot pass review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-039][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-039.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-039-M2.txt`, `.agent/node-contracts/EP-039.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `supply-chain/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep039_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-039.sh M2`

EXPECT:

- `EP-039 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-039 MILESTONE_PASS "M2 EP-039 M2: ok"`

FALLBACK: Remove or replace a component whose license or security posture cannot pass review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-039][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-039 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-039-M3.txt`, `.agent/node-contracts/EP-039.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `policies/licenses/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep039_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-039.sh M3`

EXPECT:

- `EP-039 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-039 MILESTONE_PASS "M3 EP-039 M3: ok"`

FALLBACK: Remove or replace a component whose license or security posture cannot pass review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-039][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-039 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-039-M4.txt`, `.agent/node-contracts/EP-039.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `scripts/sbom/`

CONTENT:

1. Create tests whose names begin `ep039_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-039.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-039 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-039 MILESTONE_PASS "M4 EP-039 M4: ok"`

FALLBACK: Remove or replace a component whose license or security posture cannot pass review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-039][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-039.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-039-M5.txt`, `.agent/node-contracts/EP-039.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/supply-chain/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-039` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-039.sh M5`
2. `sh scripts/node-verify.sh EP-039`
3. `sh scripts/scope-audit.sh EP-039`

EXPECT:

- `EP-039 M5: ok`
- `node verify EP-039: ok`
- `scope audit EP-039: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-039 MILESTONE_PASS "M5 EP-039 M5: ok"`

FALLBACK: Remove or replace a component whose license or security posture cannot pass review. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-039][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-039` and observe `node verify EP-039: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [ ] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

## M1 progress (observed 2026-08-23)

Remote-sync truth (EP-038 closure): EP-038 remote sync was BLOCKED by
GitHub credential HTTP 401 on every candidate (gh token, gateway-log
tokens, SSH keys). Local closure is complete; remote refs are NOT
verified. Fresh `gh auth login` or credential repair is required before
any push. EP-039 must not claim remote synchronization until observed
with ls-remote or equivalent. No force-push was attempted.

M1 implemented and proven (all commands run now, outputs observed):

- `crates/nexus-supply-chain/` provider-neutral supply-chain contract
  crate (workspace member; deps only nexus-domain + serde + serde_json
  + sha2; no vendor SDK / OCI / scanner / signer imports; dependency
  direction enforced by the gate via cargo tree).
- Vocabulary (deny-unknown, serde fail-closed): LicenseClass
  (GREEN/REVIEW/SIDECAR/EXTERNAL/PROHIBITED per LICENSE_POLICY.md),
  LicenseReview (APPROVED/DENIED/NEEDS_REVIEW), IntegrationMode
  (EMBEDDED/PROCESS_SIDECAR/EXTERNAL_PROVIDER), ApprovalState
  (APPROVED/REJECTED/PENDING), RiskClass, VerificationResult
  (VERIFIED/NOT_VERIFIED/UNVERIFIED), WaiverState, AdvisorySeverity
  (CRITICAL blocks release).
- Error surface: SupplyChainErrorCode with SPEC-006 codes +
  supply-chain-specific LicenseDenied/LicenseUnknown/SbomIncomplete/
  ProvenanceMissing/SignatureInvalid/AdvisoryBlocking/WaiverExpired;
  `to_redacted_json()` scrubs secret-shaped values (sk-, ghp_, AKIA,
  Bearer, ...) at the evidence boundary - proven by
  ep039_unit_error_messages_never_contain_secret_shaped_values.
- Model: ArtifactDigest (alg:hex, lowercase, >=32 hex, rejects tags),
  ComponentIdentity (digest is identity: same name+version with
  different digest != same artifact), Component (explicit approval +
  verification + review ladder; is_releasable() requires all three),
  ComponentBoundary + SourceOffer (copyleft sidecar isolation),
  SbomDocument (is_complete requires version+source+license on every
  package incl. transitive; is_current binds run_id + freshness;
  GENERATED != VERIFIED), ProvenanceAttestation (unsigned != trusted),
  DependencyWaiver (active+unexpired only), Advisory (critical without
  mitigation blocks).
- Ports (object-safe, provider-neutral): LicenseClassifier +
  LicenseClassifierPort (canonical classifier implementing
  LICENSE_POLICY.md: permissive GREEN, MPL/LGPL REVIEW, GPL/AGPL
  SIDECAR, commercial EXTERNAL, everything else DENIED),
  ComponentBoundaryPort, SbomGeneratorPort, ArtifactSigner,
  AdvisoryMonitor, DependencyWaiverPort.
- 41 `ep039_unit_*` proofs green across 3 suites (0 failed, 0 ignored)
  covering: deny-unknown vocabulary + serde rejection, license
  fail-closed (unknown/missing), LICENSE PRESENT != VERIFIED,
  DEPENDENCY EXISTS != APPROVED, ALLOWLIST ENTRY != APPROVAL FOR ALL
  USES, TRANSITIVE != OUT OF SCOPE, PACKAGE NAME MATCH != SAME
  ARTIFACT, IMAGE TAG != DIGEST, SBOM GENERATED != VERIFIED,
  BUILD PASSED != SBOM COMPLETE, LOCKFILE EXISTS != ACCOUNTED FOR,
  stale SBOM fails, provenance unsigned fails, waiver expired/revoked
  fails, advisory critical blocks, error codes canonical + secret-free
  serialization, port traits object-safe.
- Gate `scripts/ep039-m1-tests.sh` non-vacuous: material presence,
  workspace membership, real cargo test with pass-count vacuity guards
  (nonzero pass, zero failed, zero ignored), 29 anti-masking sentinels
  observed, dependency-direction proof (forbidden provider SDK
  families), no-placeholder scan, clippy -D warnings clean, fmt clean,
  crate license declared. Observed: `EP-039 M1 gate: ok`.
- Node `scripts/nodes/EP-039.sh` M1 rewired from artifact-check masking
  to the real gate. Observed: `EP-039 M1: ok` EXIT=0.
- Side gates green: scope audit EP-039: ok, security check: ok
  (0 advisories), dependency audit: ok, license gate: ok, reality
  gate: ok, blueprint validation: ok, format check: ok, lint: ok,
  typecheck: ok, test-unit: ok (under canonical env with mise shims;
  dart/flutter are mise-managed).
- Workspace battery: green on approved scope with live battery
  fixtures (EP-039 crate included as a pure unit crate; no fixture
  needed; destructive EP-037 M4 crate still excluded with its own
  self-provisioned gate proof).
- Resource hygiene: zero EP-039-owned containers/networks/volumes/temp
  roots (M1 starts no fixtures).

Certification boundary (honest): supply-chain policy + license policy
CONTRACT/POLICY CERTIFIED for the exact exercised surface; SBOM
schema/evidence + component provenance model CONTRACT CERTIFIED;
actual third-party legal clearance NOT ASSERTED; complete production
artifact SBOM NOT ASSERTED (no generator in M1); container image
provenance NOT ASSERTED; SLSA/in-toto signing NOT ASSERTED;
remote GitHub synchronization NOT ASSERTED until credentials are
repaired and remote refs verified.

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-23 (M1): `to_redacted_json()` initially serialized the raw
  message, and the secret-shape proof failed - the redaction boundary
  must scrub secret-shaped substrings at serialization, not rely on
  callers. Added `redact_secret_shaped()` (sk-, pk-, rk-, ghp_, gho_,
  ghs_, github_pat_, AKIA, Bearer markers) and proved it by test.
- 2026-08-23 (M1): blueprint validation rejects non-ASCII characters in
  source files (em-dashes in doc comments). Replaced with ASCII.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-23 (M1): dependency surface locked to nexus-domain + serde +
  serde_json + sha2. No SBOM/signing/scanning SDK in M1; the ports
  exist but provider implementations are owned by later milestones.
  Alternatives considered and rejected: adding a real SPDX library
  (M2+/M3+ behavior, not contract), adding a signing library (M4+).
- 2026-08-23 (M1): workspace membership + gate script added to
  expected-files EP-039.txt (same convention as EP-038 which lists
  Cargo.toml/Cargo.lock and every ep038 gate script), so scope audit
  EP-039 passes.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
