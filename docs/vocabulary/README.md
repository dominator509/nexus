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
| `SkillId`       | Skill package                | SPEC-010     |

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

## SdkLanguage

`RUST`, `TYPESCRIPT`, `PYTHON` (SPEC-022 behavior 4; ADR-016). Marks
which language surface a connector SDK binding exposes. All three
bindings implement the same contract corpus
(`CONTRACT_VERSION` in `nexus-connector-sdk`) and must pass the same
conformance suite.

## SidecarTransport

`REST`, `SOAP`, `GRAPHQL`, `SQL`, `ODBC`, `JDBC`, `CLI`, `FILES`,
`EMAIL`, `WEBHOOK`, `BROWSER`, `DESKTOP` (SPEC-022 behavior 5;
ADR-016). The sandboxed Connector Sidecar wraps exactly one transport
family; browser and desktop GUI are last resort and never hold direct
authority.

## LegacyTransport

`REST`, `SOAP`, `SQL`, `CLI`, `FILES`, `EMAIL`, `BROWSER` (SPEC-022
behavior 5; ADR-016). Legacy source families wrapped by the
`LegacyPoller`, normalized into versioned, correlated events with
stable cursors.

## WebhookDeliveryState

`PENDING`, `DELIVERED`, `FAILED`, `REPLAY` (SPEC-022 behavior 2;
ADR-016). Signed webhook delivery states; replay detection is part of
the delivery contract.

## WebhookVerification

`VALID`, `INVALID`, `REPLAY` (SPEC-022 behavior 2; ADR-016). Result of
verifying a signed webhook delivery; an invalid or replayed delivery
never becomes an event.

## ApiTransport

`REST`, `WEBSOCKET`, `MCP_STREAMABLE_HTTP`, `A2A` (SPEC-003; ADR-017).
The fabric transport families. The MCP family is Streamable HTTP per
specification 2025-11-25; A2A is protocol 1.0.1.

## McpProtocolVersion

`2025-11-25` (SPEC-003 required behavior 2; ADR-017). The locked MCP
Streamable HTTP specification target; unknown versions fail closed.

## A2AProtocolVersion

`1.0.1` (SPEC-003 required behavior 3; ADR-017). The locked A2A protocol
target.

## StreamState

`PENDING`, `RUNNING`, `COMPLETED`, `CANCELLED`, `FAILED` (SPEC-003
canonical term `Stream`; ADR-017). A2A task stream lifecycle; a
cancelled or failed stream never becomes a completed task.

## WebSocketState

`CONNECTING`, `OPEN`, `CLOSING`, `CLOSED` (SPEC-003; ADR-017). WebSocket
session lifecycle states.

## McpContentKind

`TEXT`, `IMAGE`, `AUDIO`, `RESOURCE`, `EMBEDDED` (SPEC-003 required
behavior 2; ADR-017). MCP structured content kinds.

## A2ATaskState

`SUBMITTED`, `WORKING`, `INPUT_REQUIRED`, `COMPLETED`, `CANCELLED`,
`FAILED` (SPEC-003 required behavior 3; ADR-017). A2A task lifecycle
states; A2A is for opaque agent tasks, never ordinary data reads.

## AgentCardState

`REGISTERED`, `SUSPENDED`, `REVOKED` (SPEC-003 canonical term `Agent
Card`; ADR-017). Agent card lifecycle; a revoked card is removed from
discovery.

## ArtifactState

`SEALED`, `SUPERSEDED`, `REVOKED` (SPEC-003 canonical term `Artifact
Manifest`; ADR-017). Artifacts are immutable by hash; a new version
creates a new manifest and preserves lineage.

## CapsuleState

`ACTIVE`, `EXPIRED`, `REVOKED` (SPEC-003 canonical term `Context
Capsule`; ADR-017). Context capsules contain only authorized,
task-relevant, cited data and expire after the task or declared
retention.

## EffortTier

`DETERMINISTIC`, `NON_THINKING`, `HIGH`, `MAX`, `SPECIALIST` (SPEC-009
required behavior 2; ADR-018). Ordered effort tiers; MAX is never the
default for trivial work.

## ProviderKind

`BIFROST`, `DEEPSEEK`, `OPENAI_COMPATIBLE`, `VENICE`, `XAI` (SPEC-009;
ADR-018). The model adapter families; Bifrost is preferred but
replaceable behind the ModelGateway contract.

## ProviderHealthState

`HEALTHY`, `DEGRADED`, `UNHEALTHY`, `UNKNOWN` (SPEC-009 canonical term
`ProviderHealth`; ADR-018). Observed provider health; unknown fails
closed.

## Escalation

`NONE`, `RETRY`, `FAILOVER`, `HUMAN`, `DISABLE` (SPEC-009 canonical
term `Escalation`; ADR-018). Deterministic escalation on provider
failure or policy denial.

## Microbrain

`SHADOW`, `FROZEN`, `CANARY`, `ACTIVE` (SPEC-009 canonical term
`Microbrain`; ADR-018). The microbrain lifecycle: shadow, frozen and
adversarial evals, then canary with DeepSeek fallback.

## ModelRouteClass

`DIRECT`, `CACHED`, `FALLBACK`, `ESCALATED` (SPEC-009 canonical term
`ModelRoute`; ADR-018). The resolved model route decision class.

## ModelGatewayClass

`REFLEX`, `BIFROST`, `DIRECT` (SPEC-009 canonical term `ModelGateway`;
ADR-018). Gateway implementation class.

## ReflexProviderClass

`DEEPSEEK_V4_FLASH`, `BIFROST`, `CUSTOM` (SPEC-009 canonical term
`ReflexProvider`; ADR-018). Primary reflex provider class; DeepSeek V4
Flash is V1 primary.

## CacheHitRatio

Hit prompt tokens divided by total prompt tokens (SPEC-009 canonical
term `CacheHitRatio`; ADR-018). The cacheable reflex traffic target is
at least 0.97.

## PromptSegment

`CONSTITUTION`, `SCHEMAS`, `CAPABILITY_TAXONOMY`, `RISK_POLICY`,
`EXAMPLES`, `TENANT_CONTEXT`, `SESSION_CONTEXT`, `DYNAMIC_REQUEST`
(SPEC-009 required behavior 4; ADR-018). Ordered from immutable
constitution through dynamic request; volatile IDs and timestamps stay
in the tail.

## ControlPlaneConfig

Canonical runtime configuration for the Nexus Control Plane Runtime
(SPEC-003/SPEC-006; ADR-019, EP-044): base domain/URL, bind address,
tenant, and capability list source. Provider-neutral; never carries
secrets.

## RuntimeHealth

Canonical `/healthz` response shape (SPEC-006 health contract; ADR-019,
EP-044). Must serialize as `{"status":"healthy"}` with HTTP 200 when the
runtime is healthy.

## RuntimeReadiness

Canonical `/readyz` response shape (SPEC-006; ADR-019, EP-044). Must
serialize as `{"ready":true}` with HTTP 200 when the runtime is ready.

## CapabilityList

Canonical `/v1/capabilities` response shape (SPEC-003; ADR-019, EP-044).
Must serialize as `{"capabilities":[...]}` with a non-empty list when the
runtime is ready.

## ControlPlaneServer

The runnable control-plane server boundary (ADR-019, EP-044): bind,
routes, serve, graceful shutdown. The composition root of the Nexus
runtime.

## RuntimeLifecycle

Graceful startup/shutdown contract for the runtime (ADR-019, EP-044):
bind once, serve, stop on signal, never leak processes.

## RuntimeSmoke

Canonical runtime smoke contract, owned by EP-044 (ADR-020): the runtime
smoke gate activates only at `at-least EP-044`; before the owner is DONE
the stage is `not-applicable-before EP-044`; after the owner is DONE the
smoke is mandatory and fails closed when the runtime is absent or
unhealthy. The smoke assertions themselves are never weakened.

## ReflexDecisionClass

`DETERMINISTIC`, `MODEL` (SPEC-009; ADR-021, EP-014). How a reflex
decision was produced. `DETERMINISTIC` means the model was bypassed;
`MODEL` means the decision came from a real provider and passed
validation.

## EffortSelectionClass

`POLICY_SELECTED`, `EXPLICIT` (SPEC-009 required behavior 2; ADR-021,
EP-014). How an effort tier was chosen. MAX is never the default for
trivial work.

## ReflexProvider

The provider-neutral reflex port (SPEC-009 canonical term
`ReflexProvider`; ADR-021, EP-014). Resolves a `ReflexRequest` to a
validated `ReflexDecision`. Deterministic tasks bypass the model.

## DeepSeekFlashProvider

V1 primary ReflexProvider implementation (SPEC-009; ADR-021, EP-014).
Provider id `deepseek-v4-flash`; deterministic bypass, effort policy,
validation, and cache accounting; transport injected behind the
`ReflexTransport` port so no vendor SDK enters the production tree.

## EffortPolicy

Deterministic effort-tier selection (SPEC-009 required behavior 2;
ADR-021, EP-014). Trivial work is never MAX; default is HIGH;
deterministic tasks resolve to DETERMINISTIC.

## CacheLedger

Rolling token cache accounting (SPEC-009 canonical term `CacheLedger`;
ADR-021, EP-014). Cache-hit ratio is hit prompt tokens divided by total
prompt tokens; cacheable reflex traffic targets at least 0.97.

## NexusControlObjectValidator

Deterministic control-object validation (SPEC-009 behavior 3/10;
ADR-021, EP-014). Rejects extra or invalid fields; only validated
`NexusControlObject` output continues.

## RoutingDecisionClass

`ROUTED`, `FALLBACK`, `ESCALATED`, `REJECTED`, `SHADOW` (SPEC-009
canonical term ModelRoute; ADR-022, EP-015). How a routing decision was
produced. `REJECTED` never routes to a model.

## RouterStrategyClass

`POLICY`, `ROUTE_LLM`, `LLM_ROUTER`, `MICROBRAIN` (SPEC-009; ADR-022,
EP-015). The strategy that produced a routing decision. RouteLLM and
LLMRouter are replaceable strategies; the policy engine can override
learned routing for security.

## EscalationReason

`AMBIGUITY`, `RISK`, `PRIVACY`, `BUDGET`, `UNAVAILABLE`, `COST`,
`LATENCY`, `SECURITY`, `CERTIFICATION`, `OUT_OF_DISTRIBUTION` (SPEC-009
canonical term Escalation; ADR-022, EP-015). Deterministic escalation
causes.

## MicrobrainState

`DISABLED`, `SHADOW`, `CANARY`, `ACTIVE`, `PROMOTION_GATED` (SPEC-025;
ADR-022, EP-015). The Microbrain promotion lifecycle; the safe default
is `DISABLED`. Promotion is gated by the SPEC-025 training/evaluation
pipeline in later nodes.

## ShadowDecisionClass

`MATCH`, `DIVERGE`, `FAILED` (SPEC-025 canonical term ShadowDecision;
ADR-022, EP-015). A shadow comparison outcome. A failed shadow is never
trusted.

## ProviderFailureClass

`UNAVAILABLE`, `TIMEOUT`, `RATE_LIMITED`, `CONTRACT`, `EXTERNAL`,
`REJECTED`, `BUDGET_EXHAUSTED`, `SECURITY_DENIED` (SPEC-006; ADR-022,
EP-015 M5, LF-021). Typed classification of a provider attempt failure.
Only `UNAVAILABLE` and `TIMEOUT` are failover-eligible; contract, rate,
policy, budget, and security failures never cause provider hopping.

## FailoverStage

`PRIMARY_SELECTED`, `PRIMARY_ATTEMPTED`, `PRIMARY_FAILED`,
`FAILOVER_ELIGIBLE`, `SECONDARY_SELECTED`, `SECONDARY_ATTEMPTED`,
`SECONDARY_VALIDATED`, `ROUTE_COMPLETED`, `FAILED_CLOSED` (SPEC-006
audit; ADR-022, EP-015 M5, LF-021). Ordered provider-attempt audit
stages on `RouteAuditRecord` for a failover-routed request; the routing
decision record itself is the `route_requested`/`primary_selected`
event.

## NexusModelRouter

The provider-neutral model router port (SPEC-009; ADR-022, EP-015).
Resolves `RoutingFeatures` to a validated `RoutingDecision`.
Deterministic policy routing is the V1 default.

## RoutePolicy

Deterministic route selection (SPEC-009 required behavior 7; ADR-022,
EP-015). Safety floors: R4 never routes to a model; SECRET privacy and
R3 risk never route to CHEAP_API; local-only work stays local;
deterministic tasks bypass the model. Can override learned routing for
security.

## EscalationPolicy

Deterministic escalation (SPEC-009 canonical term Escalation; ADR-022,
EP-015). Fails closed (REJECT/CLARIFY) rather than routing unsafely.

## MicrobrainProvider

The Microbrain seam (SPEC-009 behavior 9; SPEC-025; ADR-022, EP-015).
Uses the SAME `ReflexProvider` contract as DeepSeek and can remain
disabled. Begins in shadow; promotion is gated.

## ContextPurpose

`TASK_EXECUTION`, `PLANNING`, `SEARCH`, `NOTIFICATION`,
`SYSTEM_MAINTENANCE` (SPEC-020; ADR-023, EP-016). Purpose limitation
classes for context construction and memory proposals. A capsule may
only carry data whose declared purpose permits the current use.

## GraphExpansionMode

`DIRECT`, `ONE_HOP`, `TWO_HOP` (SPEC-002 behavior 7; ADR-023, EP-016).
Bounded graph-aware context construction; never expands past the
declared hop bound.

## PrivacyFilterDecision

`ALLOW`, `REDACT`, `DENY` (SPEC-020, INV-007; ADR-023, EP-016).
Per-candidate privacy filter outcome; `REDACT` carries metadata only.

## ConsolidationMode

`MODEL_ASSISTED`, `DETERMINISTIC_FALLBACK`, `SKIPPED` (SPEC-002
behavior 5; ADR-023, EP-016). Semantic consolidation execution mode;
models can never write canonical memory directly - consolidation always
emits proposals for policy evaluation.

## ContextEngine

The context engine port (SPEC-002; ADR-023, EP-016). Builds a
purpose-limited, permission-filtered `ContextCapsule` for the model
router; only authorized, task-relevant, cited data is included.

## HybridRetriever

The hybrid retrieval port (SPEC-002 behavior 6; ADR-023, EP-016).
Combines exact, full-text, vector, graph, recency, importance,
confidence, and diversity signals; always tenant-isolated and
authorization-filtered.

## MemoryConsolidator

The semantic consolidation port (SPEC-002 behaviors 4-5; ADR-023,
EP-016). Turns working/episodic sources into semantic/entity proposals;
never writes canonical memory directly.

## PrivacyFilter

The privacy filter port (SPEC-020, INV-007; ADR-023, EP-016). Enforces
purpose limitation, sensitivity ceilings, permission, and namespace
isolation; private shared-room requests use private response routing.

## GraphExpansionPolicy

The bounded graph expansion port (SPEC-002 behavior 7; ADR-023,
EP-016). Expands context from a seed node within the declared hop mode
and node budget; never crosses a tenant, namespace, or security
boundary.

## MemoryWorkflowKind

`MEMORY_CONSOLIDATION`, `MEMORY_RETENTION`, `MEMORY_LEGAL_HOLD`,
`MEMORY_EXPORT`, `MEMORY_DELETION`, `MEMORY_REEMBED` (SPEC-002
requirement 8; ADR-023, EP-016 M3). Durable, audited workflow kinds
over the memory plane; distinct from EP-006 workflow kinds and never
mixed with them at the registry boundary.

## MemoryOperationKind

`PROPOSE`, `EVALUATE_PROPOSAL`, `ACTIVATE_CANONICAL`, `SUPERSEDE`,
`RETENTION_SWEEP`, `LEGAL_HOLD_APPLY`, `LEGAL_HOLD_RELEASE`,
`EXPORT_SNAPSHOT`, `DELETE_RECORD`, `REEMBED` (SPEC-002 requirement 8;
ADR-023, EP-016 M3). Durable activity-level operations the memory
workflows schedule; each maps to a bounded, idempotent,
error-classified activity (SPEC-006 behavior 7).

## MemoryWorkflowState

`REQUESTED`, `EVALUATING`, `AWAITING_APPROVAL`, `EXECUTING`,
`VERIFYING`, `SUCCEEDED`, `FAILED`, `CANCELLED`, `TIMED_OUT`
(SPEC-002 requirement 8; ADR-023, EP-016 M3). Durable memory workflow
lifecycle; terminal outcomes mirror SPEC-006 ActionLifecycle.

## LegalHoldDecision

`APPLY`, `RELEASE` (SPEC-002 requirement 8; ADR-023, EP-016 M3).
APPLY freezes a record against retention deletion; RELEASE restores
normal retention. A legal hold preserves storage; it never implies
context relevance.

## RetentionDisposition

`KEEP`, `DELETE`, `LEGAL_HOLD` (SPEC-002 requirement 8; ADR-023,
EP-016 M3). Disposition decided by a retention sweep.

## AgentTaskState

`REQUESTED`, `ASSIGNED`, `RUNNING`, `PAUSED`, `WAITING_INPUT`,
`REVIEWING`, `CANCELLED`, `SUCCEEDED`, `FAILED` (SPEC-010; ADR-024,
EP-017). Agent task lifecycle; terminal outcomes mirror SPEC-006
ActionLifecycle and are final.

## AgentAdapterKind

`CODEX`, `CLAUDE_CODE`, `HERMES`, `OPENCLAW` (SPEC-010; ADR-024,
EP-017). Vocabulary-locked harness adapter identity; concrete adapter
implementations live in the EP-017 M2 crate boundary.

## AgentCapability

`ORCHESTRATE`, `IMPLEMENT`, `REVIEW`, `TEST`, `EXECUTE`, `SUMMARIZE`,
`ARTIFACT` (SPEC-010 behavior 2; ADR-024, EP-017). Agents request
capabilities rather than named peers; Nexus selects on quality, cost,
trust, availability, and historical success.

## DelegationState

`PROPOSED`, `ACCEPTED`, `ACTIVE`, `COMPLETED`, `REVOKED`, `FAILED`
(SPEC-010 canonical term `Delegation`; ADR-024, EP-017). Delegation is
recorded by Nexus; direct agent-to-agent authority is forbidden.

## AgentBudgetClass

`TOTAL_TOKENS`, `TOTAL_COST`, `MAX_CONCURRENT`, `MAX_DURATION_SECS`
(SPEC-010; ADR-024, EP-017). Fixed declared limits Nexus owns and
enforces fail-closed.

## SkillTrustLevel

`INSPECT_ONLY`, `SANDBOXED`, `TRUSTED`, `SYSTEM` (SPEC-010 canonical
term `Skill Trust`; ADR-025, EP-018). Community skills begin
inspect-only or sandboxed; higher tiers are earned through evals and
human promotion. The tier ceiling (`permission_ceiling`) bounds the
maximum permission a skill may request: `INSPECT_ONLY` -> `NONE`,
`SANDBOXED` -> `READ`, `TRUSTED` -> `EXECUTE`, `SYSTEM` -> `SECRETS`.
A request is never a grant; trust is one input to authorization, never
authorization itself.

## SkillPermission

`NONE`, `READ`, `WRITE`, `EXECUTE`, `NETWORK`, `SECRETS` (SPEC-010
behavior 7; ADR-025, EP-018). Declared REQUIRED permissions a skill
requests. Effective authority is the intersection of the closure's
declared requirements, the caller's grants, the tenant policy
allowance, and the trust ceiling; composition never widens authority.

## SignatureAlgorithm

`ED25519`, `ECDSA_P256` (SPEC-010; ADR-025, EP-018). Vocabulary-locked
signature algorithms for signed skill packages. Unknown algorithms are
rejected at parse time; structural validation (hex encoding, key and
signature lengths) is contract-level, cryptographic verification is
owned by the M2/M3 behavior boundary.

## SkillProposalState

`PROPOSED`, `EVAL_PENDING`, `EVAL_PASSED`, `EVAL_FAILED`,
`AWAITING_PROMOTION`, `PROMOTED`, `REJECTED`, `ROLLED_BACK`
(SPEC-010 behavior 8 `Skill Factory`; ADR-025, EP-018). Canonical
lifecycle transitions only, fail closed, no terminal resurrection.
A model/agent may PROPOSE a skill; it may not self-approve installation
(promotion requires a distinct human approver).
