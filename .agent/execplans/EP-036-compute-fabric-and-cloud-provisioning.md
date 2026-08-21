NODE-META-BEGIN
ID: EP-036
DEPS: EP-035
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-036
VERIFY_SENTINEL: node verify EP-036: ok
GREEN_TAG: green/EP-036
NODE-META-END

# 1. Purpose / Big Picture

Implement node registry, workload placement, OpenTofu modules, cloud-init, Contabo, Hetzner, DigitalOcean, AWS, generic SSH, and private mesh. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-036.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-036.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-035` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-036.md`
- `.agent/specs/SPEC-016-deployment-profiles-setup-compute-fabric-provisioning-and-updates.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-036.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-036-compute-fabric-and-cloud-provisioning.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-036.txt`
- `.agent/node-contracts/EP-036.md`
- `scripts/nodes/EP-036.sh`
- `crates/nexus-compute/`
- `infra/opentofu/`
- `infra/cloud-init/`
- `providers/contabo/`
- `providers/hetzner/`
- `providers/digitalocean/`
- `providers/aws/`
- `providers/existing-ssh/`
- `tests/infra/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `ComputeNode` | `EP-036` | Defined by EP-036; provider-neutral and versioned |
| `PlacementConstraint` | `EP-036` | Defined by EP-036; provider-neutral and versioned |
| `PlacementDecision` | `EP-036` | Defined by EP-036; provider-neutral and versioned |
| `CloudProvider` | `EP-036` | Defined by EP-036; provider-neutral and versioned |
| `ProvisioningPlan` | `EP-036` | Defined by EP-036; provider-neutral and versioned |
| `BootstrapBundle` | `EP-036` | Defined by EP-036; provider-neutral and versioned |
| `FleetEnrollment` | `EP-036` | Defined by EP-036; provider-neutral and versioned |

Acceptance obligations:

1. Workloads declare resources, locality, trust, latency, and availability
2. OpenTofu modules provision supported providers reproducibly
3. Cloud-init establishes only bootstrap identity and pulls signed releases
4. Fully local and existing SSH remain first-class paths

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement node registry, workload placement, opentofu modules, cloud-init, contabo, hetzner, digitalocean, aws, generic ssh, and private mesh.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-036-M1.txt`, `.agent/node-contracts/EP-036.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-036-compute-fabric-and-cloud-provisioning.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-036.txt`, `.agent/node-contracts/EP-036.md`, `scripts/nodes/EP-036.sh`, `crates/nexus-compute/`, `providers/digitalocean/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep036_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-036.sh M1`

EXPECT:

- `EP-036 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-036 MILESTONE_PASS "M1 EP-036 M1: ok"`

FALLBACK: Use generic existing-SSH provisioning when a cloud API adapter fails certification. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-036][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-036.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-036-M2.txt`, `.agent/node-contracts/EP-036.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/opentofu/`, `providers/aws/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep036_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-036.sh M2`

EXPECT:

- `EP-036 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-036 MILESTONE_PASS "M2 EP-036 M2: ok"`

FALLBACK: Use generic existing-SSH provisioning when a cloud API adapter fails certification. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-036][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-036 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-036-M3.txt`, `.agent/node-contracts/EP-036.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/cloud-init/`, `providers/existing-ssh/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep036_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-036.sh M3`

EXPECT:

- `EP-036 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-036 MILESTONE_PASS "M3 EP-036 M3: ok"`

FALLBACK: Use generic existing-SSH provisioning when a cloud API adapter fails certification. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-036][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-036 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-036-M4.txt`, `.agent/node-contracts/EP-036.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `providers/contabo/`, `tests/infra/`

CONTENT:

1. Create tests whose names begin `ep036_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-036.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-036 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-036 MILESTONE_PASS "M4 EP-036 M4: ok"`

FALLBACK: Use generic existing-SSH provisioning when a cloud API adapter fails certification. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-036][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-036.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-036-M5.txt`, `.agent/node-contracts/EP-036.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `providers/hetzner/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-036` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-036.sh M5`
2. `sh scripts/node-verify.sh EP-036`
3. `sh scripts/scope-audit.sh EP-036`

EXPECT:

- `EP-036 M5: ok`
- `node verify EP-036: ok`
- `scope audit EP-036: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-036 MILESTONE_PASS "M5 EP-036 M5: ok"`

FALLBACK: Use generic existing-SSH provisioning when a cloud API adapter fails certification. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-036][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-036` and observe `node verify EP-036: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary (2026-08-21): crates/nexus-compute @nexus-compute provider-neutral Compute Fabric contract crate (SPEC-016; workspace member +1, deps only nexus-domain + serde/serde_json, dependency-direction enforced) + providers/digitalocean @nexus-provider-digitalocean DigitalOcean provider binding root (region slug shape validation, opaque credential-ref binding, no SDK import). Public interfaces per node contract: ComputeNode (node registry entry; declared capacity never becomes observed/certified), PlacementConstraint (deterministic min CPU/RAM/disk/architecture/GPU/class/region/privacy/tenant; never decided by provider name), PlacementDecision (assigned/rejected with fail-closed class), CloudProvider (ProviderBinding tenant/account/region + credential ref; API health distinct from resource health), ProvisioningPlan (resource-state ladder REQUESTED->PLANNED->SUBMITTED->PROVISIONING->CREATED->REACHABLE->READY->VERIFIED->CERTIFIED with validated transitions; AMBIGUOUS provisioning outcome requires provider reconciliation, never blind retry; delete ladder DELETE REQUESTED != DELETE ACCEPTED != RESOURCE ABSENT VERIFIED; billing estimate != incurred != settled; quota semantics modeled not fabricated), BootstrapBundle (release/offline/signature references only, no credentials), FleetEnrollment (DISCOVERED -> ENROLLMENT_REQUESTED -> IDENTITY_VERIFIED -> ENROLLED -> TRUSTED validated ladder); WorkloadAssignment (ASSIGNED != STARTED != HEALTHY != VERIFIED); CloudCredentialRef opaque reference rejecting secret-shaped literals (AKIA/-----BEGIN/secret/password/token) with redacted Display; CapacityProfile provenance DECLARED != OBSERVED != CERTIFIED; SPEC-006 ComputeErrorCode (validation/authentication/authorization/policy/unavailable/timeout/conflict/not-found/rate-limit/external-provider/verification/compensation/vocabulary/internal); ports CloudProviderPort (submit/readback/delete exact-target bound) + ComputeFabricPort (register/list providers); provider-neutral M1 - zero SDK/transport/framework imports. 48 tests green (ep036_unit_contract 32 + model-internal 11 + dependency_direction 2 + digitalocean binding 3; zero failed/ignored); clippy -D warnings clean; cargo fmt clean; dependency-direction proof (cargo tree depth 1 rejects tokio/axum/reqwest/aws/digitalocean/hetzner/contabo/opentofu/terraform/cloud-init/kubernetes/docker/ssh/sqlx/nats/tonic/tracing/temporal); gate scripts/ep036-m1-tests.sh 10 anti-masking proof sentinels (unknown provider rejection, resource ladder, fail-closed placement, privacy boundary, ambiguous reconciliation, credential redaction, receipt no-overclaim, health separation, workload ladder, DO binding); node M1 rewired from artifact-only masking to real gate with rc propagation; side gates: scope audit EP-036: ok, expected files (later-owned infra/opentofu etc. recorded M2-M5), preflight: ok, reality gate: ok, security check: ok, license gate: ok, dependency audit: ok, blueprint validation: ok, format check: ok, lint: ok, typecheck: ok, test-unit: ok (workspace incl. nexus-compute 48); certification: EP-036 M1 contract layer + ComputeFabric semantics + provisioning state machine + placement policy DETERMINISTIC/INTERNAL CONTRACT CERTIFIED; real cloud API / real VPS creation / real Docker-Kubernetes provisioning / real GPU hardware / real workload execution / cloud account access / billing / quota / physical hardware certification NOT ASSERTED (M2-M5 + deployment/native/ship milestones own them)
- [x] M2: Core behavior and deterministic invariants (2026-08-21): providers/aws @nexus-provider-aws AWS provider binding crate (workspace member +1, deps nexus-compute + nexus-domain + serde only, no SDK import; AWS region slug shape validation us-east-1 style lowercase partition + name + digit, opaque credential-ref binding, ProviderKind::Aws binding; 3 ep036_unit_aws_* tests green) + infra/opentofu/ OpenTofu module root (SPEC-016: OpenTofu modules provision supported providers reproducibly; real OpenTofu v1.12.1 binary via mise tofu alias; modules/aws provider-neutral module with validated variables region/instance_type/node_name/ami_id/ssh_key_name + aws_instance resource + instance_id/public_ip exact-target outputs; tofu init -backend=false + tofu validate Success + tofu fmt --check clean, scratch .terraform removed after proof); OpenTofu plan against a live cloud account NOT ASSERTED at M2 (no real cloud account, plan requires IMDS/credentials - recorded honest boundary; provider certification owned by later milestones); gate scripts/ep036-m2-tests.sh (roots present, tofu binary present, real validate+fmt, 3 owned proof sentinels, clippy -D warnings clean, fmt clean, M1 regression); node M2 rewired from phantom tests/infra/test_ep036.py masking to real gate with rc propagation; side gates: scope audit EP-036: ok, node artifact check EP-036 M2: ok, format check: ok, lint: ok, typecheck: ok, test-unit: ok (workspace incl. aws binding 3 + compute 48); certification (honest): AWS binding identity INTERNAL CONTRACT CERTIFIED, OpenTofu module root DETERMINISTIC/TOOL CERTIFIED (validate+fmt against real OpenTofu); real AWS API / real instance creation / real cloud account access / billing / quota readback NOT ASSERTED (later milestones own them)
- [x] M3: Real dependency and transport integration (2026-08-21): providers/existing-ssh @nexus-provider-existing-ssh generic existing-SSH provider binding crate (workspace member +1; deps nexus-compute + nexus-domain only, no SDK/transport framework imports; ExistingSshBinding host/port/user/tenant + opaque CloudCredentialRef, validated constructor host 1..=253 / port >0 / user 1..=64, ProviderKind::GenericSsh binding mapping; SshProbeState Reachable/Unreachable distinct from READY/HEALTHY; redacted Debug/Display via CloudCredentialRef) + infra/cloud-init/nexus-node.cfg cloud-init bootstrap config (real cloud-init v26.1 schema validation `Valid schema nexus-node.cfg`; ssh_authorized_keys uses __NEXUS_BOOTSTRAP_PUBKEY__ placeholder, signed_release_url reference only, verified:false; never installs arbitrary software, never carries cloud credentials, never marks VERIFIED). Real transport integration: throwaway alpine+openssh image built per run, ephemeral sshd container on random host port (127.0.0.1::22 publish), real ssh-keyscan -T 10 probe against the binding's declared target, positive sentinel ep036_real_transport_ssh_keyscan_probe proves REAL probe (docker-unavailable silent skip can never print it), container removed in all exit paths + docker ps -a leak assertion. 4 tests green (3 unit + 1 integration; zero failed/ignored); clippy -D warnings clean; fmt clean; gate scripts/ep036-m3-tests.sh (roots present, cloud-init schema lint real binary + Valid schema sentinel, real cargo test --nocapture, non-zero pass vacuity guard, zero failed/ignored guards, anti-masking sentinels incl. real transport sentinel + docker-skip guard, clippy -D warnings, fmt check, M1+M2 regressions); node M3 rewired from phantom tests/infra/test_ep036.py masking to real gate with rc propagation + anti-phantom guard (fail if phantom path reappears); side gates: scope audit EP-036: ok, node artifact check EP-036 M3: ok, preflight: ok, reality gate: ok, security check: ok, license gate: ok, dependency audit: ok, blueprint validation: ok, format check: ok, lint: ok, typecheck: ok, test-unit: ok (workspace); certification (honest): existing-SSH binding INTERNAL CONTRACT CERTIFIED, ephemeral sshd path REAL TRANSPORT CERTIFIED (exact controlled path), ssh-keyscan reachability CERTIFIED for exact controlled path (PORT OPEN != SSH SERVER OBSERVED != HOST IDENTITY VERIFIED != AUTHENTICATED SESSION != NODE BOOTSTRAPPED != NODE ENROLLED != NODE READY; host key observed != trusted/pinned), cloud-init schema CERTIFIED VALID (SCHEMA VALID != BOOT EXECUTED != BOOTSTRAP SUCCEEDED != FLEET ENROLLED); SSH authentication / remote bootstrap / cloud-init execution on a VM / fleet enrollment / physical-cloud node readiness NOT ASSERTED (later milestones own them)
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

- 2026-08-21: Pre-created node script `scripts/nodes/EP-036.sh` was EP-001 masking class: M1 artifact-only, M2-M5 all called nonexistent `tests/infra/test_ep036.py`, M5 had a no-op `:` branch and an unconditional ok tail. Rewired M1 to the real gate with rc propagation; later milestones will rewire their own branches.
- 2026-08-21: nexus-domain typed IDs (CorrelationId/TenantId) enforce strict UUIDv7 format; test fixtures must use canonical `00000000-0000-7000-8000-...` shape, not arbitrary labels (caught by real test failures, fixed content-derived fixtures).
- 2026-08-21: DigitalOcean region slugs are shape-validated (lowercase letters + trailing digit, e.g. nyc1/sfo3) without hard-coding a provider region catalog; the exact available set is provider readback data for later milestones.
- 2026-08-21: `tofu plan` on the AWS module requires real cloud credentials (no EC2 IMDS role found / credential refresh failure observed); therefore M2 certifies the module via `tofu validate` + `tofu fmt --check` only, and records real plan/apply as later-milestone provider certification.
- 2026-08-21: Node script M2 branch called phantom `tests/infra/test_ep036.py` (never created); rewired to the real M2 gate with rc propagation.
- 2026-08-21: Node script M3 branch called phantom `tests/infra/test_ep036.py` (never created); rewired to the real M3 gate with rc propagation plus an anti-phantom guard that fails if the phantom path ever reappears.
- 2026-08-21: M3 gate initial run caught a real vacuity hole: the integration test's docker-unavailable path silently returned ok, and cargo test without --nocapture hid the transport eprintln sentinel. Fixed by printing a positive real-transport sentinel only after ssh-keyscan reaches the ephemeral sshd, requiring it in the gate, and failing the gate if the docker-skip message appears.
- 2026-08-21: Side-gate security check caught a real defect in committed M1 source: literal secret-shaped canaries (AKIAIOSFODNN7EXAMPLE, -----BEGIN PRIVATE KEY-----) violated the repo runtime-construction convention and only passed at M1 because security-check scans tracked files (the crate was untracked pre-commit). Rewrote the canaries as runtime-concatenated strings; security check green, M1 suite re-run 45 tests green.
- 2026-08-21: Expected-files EP-036 still lists later-owned dirs (providers/contabo M4, providers/hetzner M5, tests/infra M4) which do not exist until their milestones; this is the recorded M5-gate convention, not a defect - those dirs must NOT be pre-created.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

- 2026-08-21: M1 encodes the provider-neutral contract (what a provider must prove); provider adapters and SDKs are owned by M2-M5. Evidence: dependency-direction test rejects SDK/transport/framework crates; M1 fence owns only crates/nexus-compute + providers/digitalocean. Consequence: real cloud certification is deferred, never claimed from contract-only work. Reversal: only by ADR.
- 2026-08-21: Provisioning state ladder and ambiguous-outcome semantics are contract invariants: provider acceptance != resource created != ready; UNKNOWN mutation requires reconciliation before retry. Evidence: 48-test suite incl. ambiguous reconciliation and delete-ladder proofs. Consequence: later provider adapters cannot shortcut to VERIFIED without readback.
- 2026-08-21: Cloud credentials are opaque references (CloudCredentialRef) that reject secret-shaped literals; raw API keys/tokens/private keys never enter contract data. Evidence: redaction + rejection tests. Consequence: secret provider integration (later milestone) is the only place secrets live.
- 2026-08-21: Placement is constraint-based and fails closed: privacy/tenant/region/class boundaries are never silently downgraded; fallback may only select targets satisfying the same policy. Evidence: fail-closed placement tests.
- 2026-08-21: OpenTofu module proof at M2 is validate + fmt against the real OpenTofu binary; a real plan/apply requires a live cloud account and is NOT ASSERTED until provider certification. Evidence: `tofu plan` failure without IMDS credentials observed and recorded. Consequence: no false claim of reproducible cloud provisioning; the module contract is tool-validated only.
- 2026-08-21: Provider roots follow the M1 digitalocean pattern: each milestone adds one binding crate (aws M2, existing-ssh M3, contabo M4, hetzner M5), keeping contract layers provider-neutral and SDK-free. Evidence: M1/M2 dependency-direction proofs.
- 2026-08-21: M3 ssh-keyscan proves SSH reachability/host-key OBSERVATION only - never credential validity, login, sudo, cloud-init execution, or fleet enrollment; host key observed is not trusted/pinned (pinning ownership stays with the authoritative trust milestone). Credential refs remain non-secret references (ExistingSshBinding carries CloudCredentialRef, no inline keys/passwords). cloud-init schema validation != cloud-init execution on a VM. AWS plan/apply remains deferred. No provisioning retry is added from ambiguous transport state - discovery/probe is observation; provisioning authority stays elsewhere. Evidence: M3 4-test suite + cloud-init schema lint + M1/M2 regressions.
- 2026-08-21: The M3 ephemeral-sshd fixture always uses a randomly published host port (127.0.0.1::22) and a uniquely named owned container (nexus-ep036-ssh-<pid>); teardown runs on success and failure with a docker ps -a leak assertion, and the throwaway image is removed after the gate. No hard-coded port 22/2222 cross-run collisions. Evidence: M3 integration test + hygiene check (zero owned containers, zero leaked sshd processes, zero bound random ports).


# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
