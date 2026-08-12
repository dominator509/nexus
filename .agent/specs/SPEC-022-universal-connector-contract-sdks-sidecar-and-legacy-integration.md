# SPEC-022 - Universal Connector Contract, SDKs, Sidecar, and Legacy Integration

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define the minimum connector, tiers, manifests, health, auth, idempotency, events, SDKs, and legacy wrapping.

## Canonical terms

ConnectorManifest, ConnectorBinding, ConnectorTier, HealthReport, CapabilitySet, ChangeCursor, Webhook, ConnectorSidecar, ProviderCertification. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Tier 1 provides authenticated MCP or REST, typed capability discovery, queries, commands where supported, health, and stable schemas.
2. Tier 2 adds idempotency, events or signed webhooks or changes-since, replay, and state reconciliation. It is the minimum for important stateful systems.
3. Tier 3 adds durable workflows, governance, agent or A2A surface, artifact exchange, and independent operations.
4. Rust, Python, and TypeScript SDKs generate from the same schemas and pass one conformance suite.
5. Connector Sidecar can wrap REST, SOAP, GraphQL, SQL read replicas, ODBC or JDBC, CLI, files, email, webhooks, browser automation, or desktop GUI as a last resort.
6. Connectors declare origins, secrets by reference, scopes, data classes, cost, rate limits, risk, certification, and replacement behavior.
7. A connector cannot accept a generic credential or execute arbitrary SQL or shell unless a narrowly scoped administrative capability and sandbox explicitly permit it.
8. Events use canonical IDs, versions, correlation, and cursor semantics.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- One giant execute endpoint
- Vendor schema in Nexus domain
- Silent browser scraping
- Connector self-registration with admin scope

## Required tests

- SDK golden fixtures
- Tier conformance
- Cross-language parity
- Idempotency
- Webhook signature and replay
- Sidecar sandbox
- Legacy live-fire

## Acceptance

A new system can become Nexus-native by implementing the contract without edits to Nexus Core.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
