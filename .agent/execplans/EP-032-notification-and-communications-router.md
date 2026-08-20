NODE-META-BEGIN
ID: EP-032
DEPS: EP-031
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-032
VERIFY_SENTINEL: node verify EP-032: ok
GREEN_TAG: green/EP-032
NODE-META-END

# 1. Purpose / Big Picture

Implement person-aware push, desktop, speaker, SMS, email, phone, watch, car, privacy, urgency, quiet hours, and escalation routing. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-032.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-032.txt`.
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

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-031` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-032.md`
- `.agent/specs/SPEC-014-email-phone-fax-notifications-and-communications-routing.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-032.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-032-notification-and-communications-router.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-032.txt`
- `.agent/node-contracts/EP-032.md`
- `scripts/nodes/EP-032.sh`
- `crates/nexus-notifications/`
- `connectors/push/`
- `connectors/sms/`
- `connectors/desktop-notify/`
- `tests/notifications/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `NotificationRouter` | `nexus-notifications` | Defined by EP-032; provider-neutral and versioned |
| `NotificationEnvelope` | `nexus-notifications` | Defined by EP-032; provider-neutral and versioned |
| `ChannelProvider` | `nexus-notifications` | Defined by EP-032; provider-neutral and versioned |
| `DeliveryPolicy` | `nexus-notifications` | Defined by EP-032; provider-neutral and versioned |
| `PrivacyRouting` | `nexus-notifications` | Defined by EP-032; provider-neutral and versioned |
| `EscalationPolicy` | `nexus-notifications` | Defined by EP-032; provider-neutral and versioned |
| `DeliveryReceipt` | `nexus-notifications` | Defined by EP-032; provider-neutral and versioned |

Acceptance obligations:

1. Person, urgency, privacy, presence, availability, quiet hours, and acknowledgement determine delivery
2. Sensitive shared-room responses route privately
3. Failures escalate across configured channels without duplication
4. Every delivery has a receipt and correlation

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement person-aware push, desktop, speaker, sms, email, phone, watch, car, privacy, urgency, quiet hours, and escalation routing.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-032-M1.txt`, `.agent/node-contracts/EP-032.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-032-notification-and-communications-router.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-032.txt`, `.agent/node-contracts/EP-032.md`, `scripts/nodes/EP-032.sh`, `crates/nexus-notifications/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep032_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-032.sh M1`

EXPECT:

- `EP-032 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-032 MILESTONE_PASS "M1 EP-032 M1: ok"`

FALLBACK: Use mobile push plus dashboard inbox as the minimum reliable channel pair. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-032][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-032.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-032-M2.txt`, `.agent/node-contracts/EP-032.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/push/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep032_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-032.sh M2`

EXPECT:

- `EP-032 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-032 MILESTONE_PASS "M2 EP-032 M2: ok"`

FALLBACK: Use mobile push plus dashboard inbox as the minimum reliable channel pair. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-032][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-032 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-032-M3.txt`, `.agent/node-contracts/EP-032.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/sms/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep032_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-032.sh M3`

EXPECT:

- `EP-032 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-032 MILESTONE_PASS "M3 EP-032 M3: ok"`

FALLBACK: Use mobile push plus dashboard inbox as the minimum reliable channel pair. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-032][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-032 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-032-M4.txt`, `.agent/node-contracts/EP-032.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `connectors/desktop-notify/`

CONTENT:

1. Create tests whose names begin `ep032_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-032.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-032 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-032 MILESTONE_PASS "M4 EP-032 M4: ok"`

FALLBACK: Use mobile push plus dashboard inbox as the minimum reliable channel pair. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-032][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-032.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-032-M5.txt`, `.agent/node-contracts/EP-032.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `tests/notifications/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-032` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-032.sh M5`
2. `sh scripts/node-verify.sh EP-032`
3. `sh scripts/scope-audit.sh EP-032`

EXPECT:

- `EP-032 M5: ok`
- `node verify EP-032: ok`
- `scope audit EP-032: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-032 MILESTONE_PASS "M5 EP-032 M5: ok"`

FALLBACK: Use mobile push plus dashboard inbox as the minimum reliable channel pair. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-032][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-032` and observe `node verify EP-032: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

## Progress

- [x] M1: Contract, vocabulary, and package boundary
- [x] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

## M2 completion (2026-08-20)

Gate: `sh scripts/ep032-m2-tests.sh` -> `EP-032 M2: ok` (10 tests
total: 6 provider invariants + 4 real-socket transport; 8 vacuity
guards incl. anti-masking push sentinel, real-socket roundtrip
sentinel, M1 regression, zero ignored/filtered).
Node: `sh scripts/nodes/EP-032.sh M2` -> `EP-032 M2: ok` (RC=0).

Created:
- `connectors/push/` crate `nexus-push-connector`: mobile push channel
  provider behind the nexus-notifications ChannelProvider port
  (SPEC-014 behavior 7; channel class MOBILE_PUSH).
  - `transport.rs`: PushTransport boundary + JsonPushTransport over an
    arbitrary duplex byte source (socket/pipe/file). Wire: canonical
    NotificationEnvelope as one JSON line, one ack line back. Ack
    shape documented in-crate (provider_ref, delivered,
    delivered_at_ms, error; deny_unknown_fields - unknown ack fields
    rejected, never guessed). Malformed ack / closed peer fail closed
    (External) with correlation. No external push provider API
    invented (anti-hallucination).
  - `adapter.rs`: PushChannelProvider<T> implements ChannelProvider.
    Deterministic invariants: available() true ONLY when a transport
    is bound (Reality rule; unbound advertises nothing + Unavailable);
    every delivery returns a DeliveryReceipt carrying notification id +
    correlation (acceptance obligation 4; SENT != DELIVERED, receipt
    is the ONLY delivery authority); ack delivered=false OBSERVED as
    Failed receipt, never fabricated into success; duplicate
    notification id rejected with Conflict (bounded 256-entry
    recent-delivery ring, idempotency); sensitive payload never in
    errors/telemetry.
- `scripts/ep032-m2-tests.sh`: M2 gate (8 reality guards +
  anti-masking sentinels + M1 regression + no-ignored/no-filtered).

Side gates: fmt clean; clippy -p nexus-push-connector --all-targets -D
warnings clean; M1 regression green; scope audit pending node commit.

Certification: push connector transport TRANSPORT_CERTIFIED over real
std::net sockets vs controlled fixtures (production transport never
mocked; mocks control the peer only). No real push provider
(APNs/FCM/etc.) claimed; M3 owns connectors/sms, M4
connectors/desktop-notify, M5 live-fire + closure.

## Surprises & Discoveries

- 2026-08-20 M2: `TcpStream::pair()` is nightly-only; the repo's
  established real-socket pattern is TcpListener::bind("127.0.0.1:0")
  + accept (CrowdSec precedent), used for all real-duplex tests.
- 2026-08-20 M2: the peer thread must be joined AFTER the client I/O
  completes, or the test deadlocks (peer waits for the envelope line
  that the client only writes during deliver()).

## Decision Log

- 2026-08-20 M2: push transport is provider-neutral over an arbitrary
  duplex byte source with an in-crate documented ack shape; NO
  external push provider API is claimed (no APNs/FCM/ntfy credentials
  or documented surface exists in-repo). Alternatives (inventing a
  provider HTTP surface, claiming a real provider without
  certification) rejected as hallucination / false certification.
  Consequence: honest TRANSPORT_CERTIFIED-over-real-sockets boundary;
  real provider certification deferred to deployment/ship review.
  Reversal: swap transport behind PushTransport trait.
- 2026-08-20 M2: duplicate delivery rejected with Conflict via a
  bounded 256-entry ring (idempotency for retryable commands,
  SPEC-006); alternative (unbounded map) rejected for memory bounds.
- 2026-08-20 M2: ack delivered=false is a Failed receipt, NOT an
  error - the provider observed a failure and the receipt records it;
  fabricating an error (or success) would violate the receipt-as-
  authority invariant.

## Outcomes & Retrospective

Changed files versus the machine fence: connectors/push/ (4 files),
scripts/ep032-m2-tests.sh, .agent/expected-files/EP-032.txt,
.agent/execplans/EP-032-..., .agent/state/LEDGER.md, Cargo.toml,
Cargo.lock - all within the authorized fence.

## M1 completion (2026-08-20)

Gate: `sh scripts/ep032-m1-tests.sh` -> `EP-032 M1: ok` (21 tests
total: 20 unit + 1 dependency-direction; 10 vacuity guards incl.
anti-masking ep032_unit_* sentinel, dependency-direction, zero
ignored/filtered).
Node: `sh scripts/nodes/EP-032.sh M1` -> `EP-032 M1: ok` (RC=0).

Created:
- `crates/nexus-notifications/` crate `nexus-notifications`:
  provider-neutral notification contracts (SPEC-014 behavior 7).
  - `error.rs`: NotificationError + NotificationErrorCode (SPEC-006
    codes; Box<str> refs keep Err small; redaction-safe fields).
  - `vocabulary.rs`: owned NotificationUrgency (LOW/NORMAL/HIGH/
    CRITICAL - matches schemas/notification-envelope.schema.json
    exactly), DeliveryState (PENDING/SENDING/DELIVERED/FAILED/
    EXPIRED/ESCALATED), EscalationStage (PRIMARY/SECONDARY/TERTIARY/
    FINAL); typed ids NotificationId + DeliveryReceiptId validate in
    `new` AND serde deserialization (malformed wire values never
    bypass; fail closed). Channel classes and privacy classes come
    from nexus-domain (never redefined). No vendor brand leaks.
  - `model.rs`: NotificationEnvelope mirrors the canonical schema
    field-for-field (deny_unknown_fields; title 1..=160, summary
    1..=1000, at least one channel, non-empty expires_at; serde test
    asserts all 9 required schema fields present); DeliveryPolicy
    (min_urgency + explicit channel allowlist, fail closed - a
    channel not on the allowlist is denied); PrivacyRouting
    (SENSITIVE-or-higher privacy NEVER routes to shared-room
    channels SPEAKER/CAR - explicit rank helper since nexus-domain
    Privacy does not derive ordering); EscalationPolicy (ordered
    fallback chain, duplicate channel rejected at construction);
    DeliveryReceipt (ONLY delivery authority; Delivered state only;
    carries notification_id + correlation_id).
  - `provider.rs`: ChannelProvider port (channel(), available(),
    deliver()); NotificationRouter port (route() applies policy +
    privacy + escalation); UnboundChannelProvider +
    UnboundNotificationRouter fail closed (advertise nothing,
    Unavailable).
  - `tests/dependency_direction.rs`: nexus-domain + serde/serde_json
    only (ep032_unit_dependency_direction).
- `scripts/ep032-m1-tests.sh`: M1 gate (10 reality guards +
  anti-masking sentinel + no-ignored/no-filtered).
- `scripts/nodes/EP-032.sh`: node script (M1-M5 modes + verify).

Workspace member registered (Cargo.toml + Cargo.lock); node contract
spec path typo fixed (`.agent/specs/.agent/specs/...` -> `.agent/
specs/...`); expected-files/EP-032.txt extended with the M1 gate
script and workspace files.

Side gates: fmt clean; clippy -p nexus-notifications --all-targets -D
warnings clean (Box<str> error refs + too_many_arguments on the
schema-shaped constructor only); dependency-direction passed; scope
audit pending node commit.

Certification: nexus-notifications contract INTERNAL_CERTIFIED
(contract + vocabulary only). No channel provider is claimed
operational; M2 owns connectors/push, M3 connectors/sms, M4
connectors/desktop-notify, M5 live-fire + closure.

## Surprises & Discoveries

- 2026-08-20 M1: `schemas/notification-envelope.schema.json` already
  exists and is the canonical wire contract; the generated
  nexus-contracts DTO exists but has no validated/typed layer, so
  nexus-notifications defines the typed contract layer directly from
  the schema (anti-hallucination: field names and bounds copied from
  the schema, not invented).
- 2026-08-20 M1: nexus-domain `Privacy` does NOT derive PartialOrd,
  so "SENSITIVE or higher" cannot use `>=`; implemented an explicit
  rank helper matching SPEC-001 class order.
- 2026-08-20 M1: clippy result_large_err fired on the 5-Option-String
  error struct; the repo's established fix (nexus-sentinel) is
  Box<str> refs, which also matches redaction-safe telemetry goals.

## Decision Log

- 2026-08-20 M1: nexus-notifications owns the typed notification
  contract layer (envelope/policy/privacy/escalation/receipt) and
  reuses nexus-domain NotificationChannel + Privacy + PersonId +
  CorrelationId; alternatives (hand-rolling channels in-crate,
  editing nexus-domain) rejected as parallel vocabulary / cross-node
  scope. Consequence: single canonical vocabulary. Reversal: ADR +
  schema update. Security: least privilege, fail-closed allowlists.
  License: MIT, no new deps.
- 2026-08-20 M1: DeliveryState/EscalationStage are EP-032-owned
  vocabulary matching the schema's envelope contract; recorded here
  as the owning node's vocabulary addition (no ADR needed - they are
  node-owned classes, not synonyms for locked terms).
- 2026-08-20 M1: EscalationPolicy rejects duplicate channels at
  construction (acceptance obligation 3: without duplication), and
  DeliveryPolicy denies unlisted channels (fail closed) rather than
  defaulting to "allow all".

## Outcomes & Retrospective

Changed files versus the machine fence: crates/nexus-notifications/
(8 files), scripts/ep032-m1-tests.sh, scripts/nodes/EP-032.sh,
.agent/expected-files/EP-032.txt, .agent/node-contracts/EP-032.md,
.agent/execplans/EP-032-..., .agent/state/LEDGER.md, Cargo.toml,
Cargo.lock - all within the authorized fence.

