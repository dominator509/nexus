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

## NotificationChannel

`MOBILE_PUSH`, `DESKTOP`, `SPEAKER`, `SMS`, `EMAIL`, `PHONE`, `WATCH`,
`CAR`. (SPEC-014)

## Provider-neutrality rule

No provider brand (Alexa, Google, Apple, Samsung, Philips, Tuya, AWS, Azure,
GCP, ...) appears in a canonical class name. Provider objects are external
bounded-context records referenced by stable external identities; they never
become domain primary keys (SPEC-001 requirement 7).
