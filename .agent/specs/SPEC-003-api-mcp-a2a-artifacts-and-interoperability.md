# SPEC-003 - API, MCP, A2A, Artifacts, and Interoperability

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define the versioned Nexus control API, MCP Streamable HTTP, A2A agent collaboration, context capsules, and external contract rules.

## Canonical terms

Nexus Connector Contract, MCP, A2A, Agent Card, Context Capsule, Capability Descriptor, Query, Command, Workflow, Stream, Artifact Manifest, Invocation Context. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. REST uses OpenAPI 3.1 and JSON Schema 2020-12 generated from canonical schemas.
2. MCP targets specification 2025-11-25 with Streamable HTTP, Origin validation, authentication before tenant resolution, protocol negotiation, cancellation, structured content, and declared output schemas.
3. A2A targets protocol 1.0.1 and is used for opaque agent tasks, streaming status, artifacts, cancellation, and push notifications, not ordinary data reads.
4. Every external request carries principal, tenant, request ID, correlation ID, causation ID, idempotency key where applicable, and schema version.
5. Context capsules contain only authorized, task-relevant, cited data and expire after the task or declared retention.
6. Artifacts are immutable by hash; new versions create new manifests and preserve lineage.
7. Connector tenant and account bindings are resolved from authenticated identity and cannot be selected by untrusted request metadata.
8. The API never exposes raw provider credentials, arbitrary SQL, unrestricted shell, or generic vendor passthrough.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Direct database integration for agents
- A2A as authorization
- MCP tool that executes arbitrary strings
- GraphQL in V1

## Required tests

- OpenAPI and schema parity
- MCP conformance and Origin tests
- A2A TCK
- Cross-tenant rejection
- Artifact hash and lineage tests
- Context minimization tests

## Acceptance

A new client can discover capabilities, authenticate, query, submit an idempotent command, follow a workflow, receive events and artifacts, and never select another tenant.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
