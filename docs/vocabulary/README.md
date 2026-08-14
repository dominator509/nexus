# Nexus Canonical Vocabulary (EP-002)

This directory documents the vocabulary-locked canonical classes owned by
`crates/nexus-domain`. Names here come from the accepted specs
(SPEC-001, SPEC-003, SPEC-022) and the generated schemas under `schemas/`.
A new synonym requires an ADR and a schema update; the vocabulary enums
reject unknown classes at parse time.

## Typed identifiers

All identifiers are opaque UUIDv7 values represented as lowercase canonical
strings (`8-4-4-4-12` hex layout, version nibble `7`, variant nibble
`8/9/a/b`). Each kind is a distinct Rust newtype and is not interchangeable
at compile time.

| ID              | Meaning                      | Spec         |
| --------------- | ---------------------------- | ------------ |
| `NexusId`       | Nexus-wide opaque identifier | SPEC-001     |
| `TenantId`      | Tenant boundary              | SPEC-001     |
| `PersonId`      | Person                       | SPEC-001     |
| `HouseholdId`   | Household                    | SPEC-001     |
| `BusinessId`    | Business                     | SPEC-001     |
| `DeviceId`      | Device                       | SPEC-001     |
| `ObjectiveId`   | Objective                    | SPEC-001     |
| `TaskId`        | Task                         | SPEC-001     |
| `CapabilityId`  | Capability                   | SPEC-003/022 |
| `ArtifactId`    | Immutable artifact           | SPEC-003     |
| `EventId`       | Event                        | SPEC-022     |
| `CorrelationId` | Correlation                  | SPEC-003     |

## Risk

`R0` (no consequence), `R1` (minor), `R2` (moderate), `R3` (high),
`R4` (critical). Canonical wire values: `R0`..`R4`. (SPEC-006)

## Privacy

`PUBLIC`, `HOUSEHOLD`, `PERSONAL`, `SENSITIVE`, `BUSINESS_CONFIDENTIAL`,
`SECURITY`, `SECRET`. (SPEC-001)

## Route

`DETERMINISTIC`, `REFLEX`, `CHEAP_API`, `FRONTIER_API`, `SPECIALIST_AGENT`,
`CLARIFY`, `REJECT`. (SPEC-009)

## PrincipalType

`HUMAN`, `SERVICE`, `AGENT`, `DEVICE`, `SYSTEM`. (SPEC-001/005)

## EvidenceKind

`VOICE`, `ROOM`, `BLE`, `MOBILE`, `CAMERA`. Evidence combines into
confidence; it is never cryptographic authentication (INV-003, SPEC-005
behavior 3, EP-003 acceptance obligation 2).

## ConfidenceLevel

`LOW`, `MEDIUM`, `HIGH`. Deterministic bands over fused presence evidence
(EP-003; ADR-007).

## DeviceKind

`PHONE`, `TABLET`, `DESKTOP`, `LAPTOP`, `SPEAKER`, `CAMERA`, `DISPLAY`,
`SERVER`, `APPLIANCE`, `UNKNOWN`. Provider-neutral device classes
(SPEC-012/017; ADR-007).

## TrustLevel

`UNVERIFIED`, `LOCAL`, `VERIFIED`. Device trust ladder; never a substitute
for cryptographic step-up (SPEC-005 behaviors 3-4; ADR-007).

## LifecycleState

`PENDING`, `ACTIVE`, `SUSPENDED`, `DISABLED`, `ARCHIVED`. World-entity
lifecycle (SPEC-001; ADR-007).

## SessionState

`ACTIVE`, `EXPIRED`, `REVOKED`. Sessions are independently scoped from
people and devices (EP-003 acceptance obligation 1; ADR-007).

## CapabilityClass

`QUERY`, `COMMAND`, `WORKFLOW`, `STREAM`, `ADMINISTRATIVE`. (SPEC-003)

## ApprovalClass

`NONE`, `POLICY`, `HUMAN`, `STRONG_HUMAN`, `FOUR_EYES`. (SPEC-006)

## Reversal

`NONE`, `COMPENSATING`, `SNAPSHOT`, `IRREVERSIBLE`. (SPEC-006)

## Idempotency

`NOT_APPLICABLE`, `OPTIONAL`, `REQUIRED`. (SPEC-006)

## Availability

`AVAILABLE`, `DEGRADED`, `UNAVAILABLE`, `UNCERTIFIED`. (SPEC-022)

## Locality

`ANY`, `CONTROL_PLANE`, `HOME_EDGE`, `CLIENT_DEVICE`, `HARDWARE_NODE`.
(SPEC-016)

## Connector tier

`TIER1` (authenticated MCP/REST + discovery), `TIER2` (+ idempotency,
events/webhooks, replay, reconciliation), `TIER3` (+ durable workflows,
governance, A2A, artifacts). (SPEC-022)

## ConnectorRuntime

`RUST`, `PYTHON`, `TYPESCRIPT`, `WASM`, `SIDECAR`, `APPLIANCE`. (SPEC-022)

## MemoryType

`WORKING`, `EPISODIC`, `SEMANTIC`, `ENTITY`, `PROCEDURAL`, `DECISION`,
`SKILL`, `SYSTEM`. (SPEC-002)

## Sensitivity

`PUBLIC`, `HOUSEHOLD`, `PERSONAL`, `SENSITIVE`, `BUSINESS_CONFIDENTIAL`,
`SECURITY`, `SECRET`. Memory-record data classification (SPEC-002, SPEC-020;
ADR-008). Wire strings match the privacy ladder so memory filtering and
redaction reuse the same policy classes (INV-014).

## MemoryStatus

`PROPOSED`, `ACTIVE`, `SUPERSEDED`, `REJECTED`, `DELETED`. Memory record
lifecycle (SPEC-002 behaviors 5, 8; ADR-008). `PROPOSED` records are
policy-evaluated proposals, never canonical facts.

## RetentionUnit

`HOURS`, `DAYS`, `WEEKS`, `MONTHS`, `YEARS`, `INDEFINITE`. Retention policy
durations (SPEC-002, SPEC-020; ADR-008). `INDEFINITE` covers legal hold and
no-expiry retention.

## NotificationChannel

`MOBILE_PUSH`, `DESKTOP`, `SPEAKER`, `SMS`, `EMAIL`, `PHONE`, `WATCH`,
`CAR`. (SPEC-014)

## EventType

Dotted lowercase slug, e.g. `memory.record.created`. Event type of an
`EventEnvelope` (SPEC-023; ADR-009). New types are added by ADR and schema
update, never invented at runtime.

## EventDataClass

`PUBLIC`, `HOUSEHOLD`, `PERSONAL`, `SENSITIVE`, `BUSINESS_CONFIDENTIAL`,
`SECURITY`, `SECRET`. Event data classification (SPEC-023 behavior 3,
SPEC-020; ADR-009). Wire strings match the privacy ladder so event
filtering and redaction reuse the same policy classes (INV-014).

## OutboxStatus

`PENDING`, `PUBLISHING`, `PUBLISHED`, `FAILED`. Transactional outbox
lifecycle (SPEC-023 behaviors 1-2; ADR-009). A row becomes `PUBLISHED`
only after the transport acknowledges durable storage.

## InboxStatus

`NEW`, `PROCESSING`, `DONE`, `FAILED`. Consumer inbox lifecycle
(SPEC-023 behavior 4; ADR-009). Consumers deduplicate by event ID so
replay does not create duplicate logical effects.

## DurableConsumer

A consumer with a durable `ConsumerCheckpoint` (consumer, stream,
subject, last sequence) that resumes after restart (SPEC-023 behavior 4;
ADR-009). Idempotent by construction.

## Workflow

A durable, deterministic, versioned unit of long-running work (SPEC-023
behavior 5; ADR-010). Owned by `packages/workflows`
(`@nexus/workflows`). Time and I/O flow only through the workflow
context; every side effect lives in an activity (behavior 6).
`WorkflowKind`: `OBJECTIVE`, `APPROVAL`, `CONNECTOR_CERTIFICATION`,
`INCIDENT_REMEDIATION`, `DEPLOYMENT`. `WorkflowState`: `REQUESTED`,
`EVALUATED`, `AWAITING_APPROVAL`, `APPROVED`, `EXECUTING`, `VERIFYING`,
`SUCCEEDED`, `FAILED`, `REJECTED`, `COMPENSATING`, `COMPENSATED`,
`CANCELLED`, `TIMED_OUT` (the last two are explicit Temporal-owned
terminals; EP-006 acceptance obligation 3).

## Activity

The only surface that touches the outside world (SPEC-023 behavior 6;
ADR-010). `ActivityKind`: `EXTERNAL_EFFECT`, `VERIFY`, `COMPENSATE`.
Every activity carries an idempotency key and a bounded,
error-classified retry policy (SPEC-006 behaviors 2, 5, 7, 8).
`RetryErrorClass`: `TRANSIENT`, `RATE_LIMIT`, `UNAVAILABLE`, `TIMEOUT`,
`PERMANENT` (never retried).

## Signal

An immutable, durable, idempotent message to a workflow (SPEC-023
behavior 7; ADR-010). Every signal carries a `signalId` (UUIDv7);
duplicate signals collapse on the canonical `signalKey` (workflow +
type + signalId). `SignalType`: `APPROVAL`, `CANCEL`, `RESUME`. New
signal types require an ADR.

## Query

A deterministic, read-only view of workflow state (ADR-010). Answers
derive from the durable event history, so replay answers identically.
`QueryType`: `WORKFLOW_STATUS`, `PENDING_APPROVAL`, `ACTIVITY_STATE`,
`ACTION_RECEIPT`.

## Schedule

Temporal-owned scheduled execution (SPEC-023 canonical term; ADR-010).
Vocabulary reserved for later nodes.

## ApprovalWorkflow

A durable human-approval gate (SPEC-023; ADR-010). An approval is an
immutable `ApprovalAssertion` carrying the exact action digest, the
signer principal, and the authentication strength/context; it binds to
the exact action payload digest, never to free text. `ApprovalDecision`:
`APPROVE`, `REJECT`. `CancelAction`: `CANCEL` (fail closed) or
`COMPENSATE` (rollback).

## Compensation

An explicit rollback capability registered per effect, executed in
reverse order (SPEC-006 behavior 8; ADR-010). Every `EXTERNAL_EFFECT`
activity declares a compensation step.

## AuthenticationStrength

`NONE`, `SINGLE_FACTOR`, `MULTI_FACTOR`, `STEP_UP` (SPEC-005 canonical
term; ADR-010; Rust mirror ADR-011). Ordered strength ladder
`NONE < SINGLE_FACTOR < MULTI_FACTOR < STEP_UP`. SPEC-005 behavior 4
requires a cryptographic step-up for R3 and R4 actions; R4 never
accepts model approval.

## TokenClass

`ACCESS`, `REFRESH`, `ID` (SPEC-005; ADR-011). ACCESS tokens are
short-lived bearer tokens; REFRESH tokens are rotation-only credentials,
never used as bearer; ID tokens carry identity claims.

## PasskeyState

`PENDING_CHALLENGE`, `REGISTERED`, `REVOKED` (SPEC-005; ADR-011).
Passkeys are WebAuthn-based possession factors; a challenge must be
satisfied before a credential registers.

## DeviceEnrollmentState

`PENDING_VERIFICATION`, `ENROLLED`, `REJECTED`, `REVOKED` (SPEC-005;
ADR-011). Trust is evidence, never cryptographic authentication
(INV-003); enrollment completes only after verification evidence is
accepted.

## StepUpState

`PENDING`, `SATISFIED`, `EXPIRED`, `CANCELLED` (SPEC-005 behavior 4;
ADR-011). A step-up challenge proves the operator is present and has
satisfied the configured strength before a high-risk action proceeds.

## RecoveryMaterialKind

`SEALED_ENVELOPE`, `SPLIT_SHARES`, `RECOVERY_CODE` (SPEC-005 behavior 6;
ADR-011). Offline recovery material is sealed at the boundary and
referenced by secret reference, never stored plaintext.

## GrantFlow

`AUTHORIZATION_CODE`, `CLIENT_CREDENTIALS`, `REFRESH_TOKEN` (SPEC-005;
ADR-011). OIDC/OAuth2 authorization grant families; service identities
use client credentials.

## RecoveryKitState

`PROVISIONED`, `SEALED`, `VERIFIED`, `REVOKED` (SPEC-005 behavior 6;
ADR-011). Recovery kits are sealed after provisioning and verified by
successful recovery exercises.

## ActionDigest

Canonical lowercase SHA-256 hex string (64 chars) of the exact action
payload being approved (ADR-010). The approval binds to this digest,
never to human text.

## Provider-neutrality rule

No provider brand (Alexa, Google, Apple, Samsung, Philips, Tuya, AWS, Azure,
GCP, ...) appears in a canonical class name. Provider objects are external
bounded-context records referenced by stable external identities; they never
become domain primary keys (SPEC-001 requirement 7).

## ActionLifecycleState

`REQUESTED`, `EVALUATED`, `AWAITING_APPROVAL`, `APPROVED`, `EXECUTING`,
`VERIFYING`, `SUCCEEDED`, `FAILED`, `COMPENSATING`, `COMPENSATED`,
`REJECTED` (SPEC-006 behavior 4; ADR-012). Every consequential action
moves through this deterministic lifecycle; the Action Gateway and
receipts reference the state at each boundary.

## GrantState

`ACTIVE`, `REVOKED`, `EXPIRED` (SPEC-005 behavior 5; ADR-012).
Capability grants never outlive expiry and never widen scope.

## ApprovalDecision

`APPROVED`, `REJECTED` (SPEC-005; ADR-012). Approval assertions bind an
approver to an exact action digest; digest mismatch or expiry is a
rejection.

## ReceiptState

`ISSUED`, `SUPERSEDED` (SPEC-005 behavior 9; ADR-012). Authorization
receipts are redacted, versioned, and may be superseded by later
compensation/verification records.

## DenialReason

`RELATIONSHIP`, `POLICY`, `INSUFFICIENT_STRENGTH`, `NO_CAPABILITY`,
`MISSING_APPROVAL`, `VERIFICATION_FAILED` (SPEC-006; ADR-012). Stable
machine reasons for gateway denials; `RiskClass` is the existing `Risk`
(R0..R4) ladder.

## TrustZone

`PUBLIC`, `GUEST`, `LOCAL`, `PRIVATE_MESH` (SPEC-020; SPEC-005 behavior
7; ADR-013). Every service, device, and mesh node belongs to exactly one
zone; zone boundaries determine mTLS policy, WireGuard segment
membership, and secret exposure.

## TokenState

`ACTIVE`, `REVOKED`, `EXPIRED` (SPEC-005 behavior 5; ADR-013).
Capability tokens are short-lived, audience/resource/action/tenant
restricted, and non-transferable; `REVOKED` and `EXPIRED` are terminal.

## SecretState

`ACTIVE`, `ROTATING`, `REVOKED` (SPEC-005 behavior 6; ADR-013).
`ROTATING` means a new version is being installed; `REVOKED` means the
reference no longer resolves.

## CertificateState

`ACTIVE`, `EXPIRED`, `REVOKED` (SPEC-005 behavior 7; ADR-013).
Certificates are short-lived; `EXPIRED` is terminal after `not_after`,
`REVOKED` is terminal before `not_after`.

## ServiceIdentityState

`ACTIVE`, `SUSPENDED`, `REVOKED` (ADR-013). A service identity is the
canonical service principal bound to an mTLS certificate; `SUSPENDED`
stops new issuance without destroying the record, `REVOKED` terminates
it.

## MeshNodeState

`PENDING`, `REGISTERED`, `ONLINE`, `OFFLINE`, `REVOKED` (ADR-013).
`PENDING` means a node requested membership but is not yet registered;
`REGISTERED` means it holds a WireGuard key pair and can connect;
`ONLINE`/`OFFLINE` are operational observations; `REVOKED` is terminal.

## Secret Reference

A `store:key[@version]` reference to a secret by name (SPEC-005
behavior 6; ADR-013). Values never enter domain records, logs, or model
context; resolution happens in infrastructure.

## Service Identity SAN (canonical certificate binding)

`nexus://tenant/<tenant_id>/service/<identity_id>` (ADR-014). Every
certificate issued by the Nexus CA carries this deterministic URI SAN as
the authoritative identity binding; the transport DNS SAN
`<identity_id>.<tenant_id>.svc.nexus.internal` is derived from the same
record for standard TLS hostname verification. One identity, two
encodings; new namespaces require an ADR.

## HealthState

`HEALTHY`, `DEGRADED`, `UNAVAILABLE`, `UNKNOWN` (SPEC-022; ADR-015).
Health state is an operational observation of a capability or
connector, never a certification claim.

## Certification

`UNCERTIFIED`, `LAB`, `CERTIFIED`, `DEPRECATED` (SPEC-022
`ProviderCertification`; ADR-015). A connector whose features are not
certified must not advertise them as available; the capability registry
omits uncertified or unavailable features from discovery.

## SchemaRef

A canonical JSON Schema 2020-12 reference restricted to `schemas/...`
or `https://schemas.nexus.local/...` URIs (SPEC-003 behavior 1;
ADR-015). Capabilities advertise `input_schema` and `output_schema` by
`SchemaRef`; foreign URIs are rejected at construction.

## Invocation Context

The context carried by every capability and connector request:
`request_id`, `correlation_id`, `causation_id`, `origin_system`,
`external_actor_id`, `external_actor_type`, `tenant_id`, `channel`,
`device_id`, `objective_id`, `task_id` (SPEC-003 canonical term;
ADR-015). Connector tenant and account bindings resolve from
authenticated identity and can never be selected by untrusted request
metadata.
