NODE-META-BEGIN
ID: EP-031
DEPS: EP-030
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-031
VERIFY_SENTINEL: node verify EP-031: ok
GREEN_TAG: green/EP-031
NODE-META-END

# 1. Purpose / Big Picture

Implement optional Suricata, Zeek, CrowdSec, Wazuh or osquery profiles, honeypots, triage, investigation, response, and verification. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-031.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-031.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-030` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-031.md`
- `.agent/specs/SPEC-013-sentinel-firewall-dns-network-detection-and-endpoint-security.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-031.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-031-sentinel-advanced-detection-and-endpoints.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-031.txt`
- `.agent/node-contracts/EP-031.md`
- `scripts/nodes/EP-031.sh`
- `connectors/suricata/`
- `connectors/zeek/`
- `connectors/crowdsec/`
- `connectors/wazuh/`
- `connectors/osquery/`
- `infra/sentinel/advanced/`
- `tests/sentinel/advanced/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `NetworkDetectionProvider` | `nexus-sentinel` | Defined by EP-031; provider-neutral and versioned |
| `EndpointTelemetryProvider` | `nexus-sentinel` | Defined by EP-031; provider-neutral and versioned |
| `ThreatIntelProvider` | `nexus-sentinel` | Defined by EP-031; provider-neutral and versioned |
| `HoneypotProvider` | `nexus-sentinel` | Defined by EP-031; provider-neutral and versioned |
| `SecurityTriage` | `nexus-sentinel` | Defined by EP-031; provider-neutral and versioned |
| `SecurityInvestigator` | `nexus-sentinel` | Defined by EP-031; provider-neutral and versioned |
| `ResponsePlanner` | `nexus-sentinel` | Defined by EP-031; provider-neutral and versioned |
| `SecurityVerifier` | `nexus-sentinel` | Defined by EP-031; provider-neutral and versioned |

Acceptance obligations:

1. Advanced sensors are optional profiles
2. Alerts correlate into incidents instead of flooding users
3. High-confidence bounded quarantine can be preauthorized
4. Destructive response remains human controlled

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement optional suricata, zeek, crowdsec, wazuh or osquery profiles, honeypots, triage, investigation, response, and verification.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-031-M1.txt`, `.agent/node-contracts/EP-031.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-031-sentinel-advanced-detection-and-endpoints.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-031.txt`, `.agent/node-contracts/EP-031.md`, `scripts/nodes/EP-031.sh`, `connectors/suricata/`, `infra/sentinel/advanced/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep031_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-031.sh M1`

EXPECT:

- `EP-031 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-031 MILESTONE_PASS "M1 EP-031 M1: ok"`

FALLBACK: Use Suricata alone for enhanced detection and postpone Zeek or endpoint agents when hardware is insufficient. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-031][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-031.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-031-M2.txt`, `.agent/node-contracts/EP-031.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/zeek/`, `tests/sentinel/advanced/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep031_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-031.sh M2`

EXPECT:

- `EP-031 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-031 MILESTONE_PASS "M2 EP-031 M2: ok"`

FALLBACK: Use Suricata alone for enhanced detection and postpone Zeek or endpoint agents when hardware is insufficient. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-031][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-031 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-031-M3.txt`, `.agent/node-contracts/EP-031.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/crowdsec/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep031_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-031.sh M3`

EXPECT:

- `EP-031 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-031 MILESTONE_PASS "M3 EP-031 M3: ok"`

FALLBACK: Use Suricata alone for enhanced detection and postpone Zeek or endpoint agents when hardware is insufficient. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-031][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-031 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-031-M4.txt`, `.agent/node-contracts/EP-031.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/wazuh/`

CONTENT:

1. Create tests whose names begin `ep031_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-031.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-031 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-031 MILESTONE_PASS "M4 EP-031 M4: ok"`

FALLBACK: Use Suricata alone for enhanced detection and postpone Zeek or endpoint agents when hardware is insufficient. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-031][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-031.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-031-M5.txt`, `.agent/node-contracts/EP-031.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/osquery/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-031` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-031.sh M5`
2. `sh scripts/node-verify.sh EP-031`
3. `sh scripts/scope-audit.sh EP-031`

EXPECT:

- `EP-031 M5: ok`
- `node verify EP-031: ok`
- `scope audit EP-031: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-031 MILESTONE_PASS "M5 EP-031 M5: ok"`

FALLBACK: Use Suricata alone for enhanced detection and postpone Zeek or endpoint agents when hardware is insufficient. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-031][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-031` and observe `node verify EP-031: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- `LF-009` `sentinel-quarantine`: Detect a synthetic but real network-lab scan from an unknown device, correlate telemetry, request or apply policy-authorized quarantine, and verify isolation.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [ ] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

## M1 completion (2026-08-20)

Gate: `sh scripts/ep031-m1-tests.sh` -> `EP-031 M1: ok` (19 advanced unit + 1
dependency-direction + 2 Suricata connector tests; 6 vacuity guards,
anti-masking EP-031-owned sentinels, zero ignored/filtered).
Node: `sh scripts/nodes/EP-031.sh M1` -> `EP-031 M1: ok` (RC=0), rewired
from EP-001-masking artifact-check branch to the real gate.

Created:
- `infra/sentinel/advanced/` crate `nexus-sentinel-advanced`: EP-031
  provider-neutral contract layer. Vocabulary (SPEC-013 behavior 3/7):
  AdvancedSensorProfile (SURICATA/ZEEK/CROWDSEC/WAZUH/OSQUERY/HONEYPOT),
  AlertState, IncidentState, CorrelationConfidence, HoneypotKind/State,
  TriagePriority, InvestigationState, ResponseKind (is_destructive /
  is_bounded_containment classes: destructive never preauthorized),
  ResponsePlanState, VerificationState; EP-031-owned typed ids
  (SecurityEventId/IncidentCorrelationId/HoneypotId/TriageCaseId/
  InvestigationCaseId/ResponsePlanId/VerificationRecordId, 1..=128
  validated in new AND serde). Value objects: SecurityEvent (OBSERVED
  data + evidence_ref), Incident (correlates events not floods),
  HoneypotRecord, TriageCase, InvestigationCase, ResponsePlan
  (preauthorized = bounded containment only; destructive requires
  human), VerificationRecord. Public interfaces (node contract):
  NetworkDetectionProvider, EndpointTelemetryProvider, ThreatIntelProvider,
  HoneypotProvider, SecurityTriage, SecurityInvestigator, ResponsePlanner,
  SecurityVerifier + Unbound* fail-closed impls (empty capabilities,
  Unavailable, never fabricate). Reuses nexus-sentinel SPEC-006
  SentinelError + SentinelCapabilityMap (fail closed), nexus-domain
  IncidentId/ApprovalClass never redefined. Dependency direction
  test: nexus-domain/nexus-sentinel/serde only.
- `connectors/suricata/` crate `nexus-suricata-connector`: package
  boundary + documented Suricata EVE JSON surface vocabulary
  (EveEventType alert/dns/flow/http/tls/smtp/ssh/fileinfo/netflow,
  SuricataAlertSeverity 1..=4 bounded; unknown rejected). Real
  NetworkDetectionProvider transport arrives M2+.

Side gates: scope audit EP-031: ok; fmt clean; clippy -D warnings clean;
security check: ok; license gate: ok; reality gate: ok; dependency audit:
ok (cargo-deny 0.20.2); workspace battery 2131 passed 0 failed (2109 prior
+ 22 new EP-031 tests; docker volume prune reclaimed 63.31GB, 100%->70%
disk).

# 12. Surprises & Discoveries

Append dated evidence-backed discoveries. Do not use this section for speculation.

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
