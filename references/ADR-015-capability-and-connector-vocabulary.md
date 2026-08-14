# ADR-015 - Capability and Connector Vocabulary

Status: Accepted
Date: 2026-08-14
Owner: hermes-nexus-main

## Context

EP-010 owns capability discovery, health, command, query, event, and
connector-tier contracts: `CapabilityRegistry`, `CapabilityDescriptor`,
`ConnectorManifest`, `QueryCapability`, `CommandCapability`,
`WorkflowCapability`, `HealthCapability`, and `ChangeFeedCapability`
(node contract `.agent/node-contracts/EP-010.md`), owned by the Rust
crate `crates/nexus-capabilities`. SPEC-003 and SPEC-022 lock the
canonical terms `Capability Descriptor`, `Query`, `Command`,
`Workflow`, `Stream`, `Invocation Context`, `ConnectorManifest`,
`ConnectorBinding`, `ConnectorTier`, `HealthReport`, `CapabilitySet`,
`ChangeCursor`, `Webhook`, `ConnectorSidecar`, and
`ProviderCertification`. The existing nexus-domain vocabulary already
carries `CapabilityClass`, `ApprovalClass`, `Risk`, `Reversal`,
`Idempotency`, `Availability`, `Locality`, `Tier`, `ConnectorRuntime`,
`Privacy`, and `PrincipalType`, and the canonical schemas
`capability-descriptor.schema.json`, `connector-manifest.schema.json`,
and `invocation-context.schema.json` already exist under `schemas/`
(created by the bootstrap pack). EP-005 M1 doctrine requires every new
public name to come from an accepted vocabulary or be added by an ADR
and a schema update in the same milestone.

## Decision

Add the following vocabulary-locked classes, owned by
`crates/nexus-capabilities` and documented in
`docs/vocabulary/README.md`:

- `HealthState` (SPEC-022 `HealthReport`): `HEALTHY`, `DEGRADED`,
  `UNAVAILABLE`, `UNKNOWN`. Health state is an operational
  observation, never a certification claim.
- `Certification` (SPEC-022 `ProviderCertification`; schema
  `connector-manifest`): `UNCERTIFIED`, `LAB`, `CERTIFIED`,
  `DEPRECATED`. A connector whose features are not certified must not
  advertise them as available; the registry omits uncertified or
  unavailable features from discovery (node contract acceptance
  obligation 4).
- `SchemaRef` (SPEC-003 behavior 1): a canonical JSON Schema 2020-12
  reference restricted to `schemas/...` or
  `https://schemas.nexus.local/...` URIs. Capabilities advertise
  `input_schema` and `output_schema` by `SchemaRef` so generated
  bindings and cross-language clients resolve one canonical
  definition. Foreign URIs are rejected at construction.

Reused vocabulary (no new names): `CapabilityClass` (`QUERY`,
`COMMAND`, `WORKFLOW`, `STREAM`, `ADMINISTRATIVE`), `Idempotency`,
`Availability`, `Locality`, `Tier`, `ConnectorRuntime`, `Risk`,
`ApprovalClass`, `Reversal`, `Privacy`, `PrincipalType`, and the typed
IDs `CapabilityId`, `NexusId`, `TenantId`, `CorrelationId`, `DeviceId`,
`ObjectiveId`, `TaskId`.

New struct types (`CapabilityDescriptor`, `CapabilityVersion`,
`ConnectorManifest`, `ConnectorId`, `ConnectorBinding`,
`InvocationContext`, `HealthReport`, `ChangeCursor`, `ChangeEvent`,
`ChangeBatch`, `QueryRequest`, `QueryResult`, `CommandRequest`,
`CommandResult`, `WorkflowRequest`, `WorkflowHandle`, `WorkflowResult`,
`CapabilityError`) are interface records, not vocabulary classes;
their field names are snake_case wire-stable via serde, matching the
canonical schemas.

## Consequences

- Capabilities advertise stable schemas, scopes, risk, idempotency,
  health, and availability through `CapabilityDescriptor`, whose
  validation mirrors the canonical `capability-descriptor.schema.json`
  constraints (id pattern, description length, unique scopes, version
  form).
- Read, proposal, command, and workflow classes remain distinct:
  `QUERY`, `COMMAND`, `WORKFLOW`, `STREAM`, and `ADMINISTRATIVE` are
  distinct `CapabilityClass` variants and each class maps to a
  dedicated typed port (`QueryCapability`, `CommandCapability`,
  `WorkflowCapability`, `HealthCapability`, `ChangeFeedCapability`).
  There is no generic `execute(String)` anywhere in the contract; the
  type system prevents invoking a query through a command port.
- Unavailable provider features are not advertised: the registry port
  returns only descriptors whose `availability` is `AVAILABLE`, and
  `Certification::Uncertified`/`Unavailable` features never appear in
  discovery results.
- Connector tenant and account bindings are resolved from
  authenticated identity: `InvocationContext` carries
  `external_actor_id`, `external_actor_type`, and `tenant_id`, and
  `ConnectorBinding` pairs a connector with a tenant and account
  reference; request metadata can never select another tenant.
- Errors are typed per SPEC-006 (`CapabilityErrorCode` with
  validation, authentication, authorization, policy, unavailable,
  timeout, conflict, not found, rate limit, external provider,
  verification, compensation, internal classes), preserve correlation,
  actor, tenant, and resource references, and never carry secrets or
  raw provider payloads.
- New synonyms or lifecycle states require an ADR + vocabulary update,
  mirroring ADR-011 (auth), ADR-012 (policy), and ADR-013 (trust).
