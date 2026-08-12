# SPEC-001 - Core Domain, Identity References, and World Model

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define stable identifiers, entities, relationships, state observations, interaction context, objectives, and provider-neutral value objects.

## Canonical terms

NexusId, TenantId, HouseholdId, BusinessId, PersonId, DeviceId, NodeId, CapabilityId, ObjectiveId, TaskId, ArtifactId, MemoryId, Observation, StateClaim, InteractionContext, WorldEntity, WorldEdge. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. All IDs are opaque UUIDv7 values represented as lowercase canonical strings.
2. World entities have type, owner namespace, display name, lifecycle state, labels, external identities, provenance, and version.
3. World edges have typed relationship, direction, validity interval, provenance, confidence, and policy visibility.
4. Observations are immutable statements from a source and never overwrite truth without reconciliation.
5. InteractionContext contains authenticated principal, device, channel, room, presence evidence, privacy class, correlation, and current objective references.
6. Hydra CRM entities, Home Assistant entities, Frigate review items, and provider objects remain external bounded-context records referenced by stable external identities.
7. No vendor-specific identifier appears as a domain primary key.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Concrete database queries
- Authentication protocols
- Provider SDK types
- Dedicated graph vendor semantics

## Required tests

- UUID and serialization properties
- Entity version concurrency tests
- Relationship validity tests
- Observation reconciliation tests
- No vendor types in domain dependency test

## Acceptance

Pure domain tests pass and infrastructure crates cannot be imported by the domain crate.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
