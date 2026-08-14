# ADR-016 - Connector SDK and Sidecar Vocabulary

Status: Accepted
Date: 2026-08-14
Owner: hermes-nexus-main

## Context

EP-011 owns the Rust, Python, and TypeScript connector SDKs plus a
sandboxed legacy Connector Sidecar (node contract
`.agent/node-contracts/EP-011.md`), in the Rust crate
`crates/nexus-connector-sdk`. SPEC-022 locks the canonical terms
`ConnectorManifest`, `ConnectorBinding`, `ConnectorTier`,
`HealthReport`, `CapabilitySet`, `ChangeCursor`, `Webhook`,
`ConnectorSidecar`, and `ProviderCertification`, and behavior 4
requires all three SDKs to generate from the same schemas and pass one
conformance suite. The existing nexus-domain vocabulary already
carries `ConnectorRuntime` (including `Sidecar`), `Tier`,
`CapabilityClass`, `Idempotency`, `Availability`, `Risk`, and the typed
IDs. EP-005 M1 doctrine requires every new public name to come from an
accepted vocabulary or be added by an ADR and a schema update in the
same milestone.

## Decision

Add the following vocabulary-locked classes, owned by
`crates/nexus-connector-sdk` and documented in
`docs/vocabulary/README.md`:

- `SdkLanguage` (SPEC-022 behavior 4): `RUST`, `TYPESCRIPT`,
  `PYTHON`. Marks which language surface a binding exposes; all
  bindings implement the same contract corpus
  (`SDK_CONTRACT_VERSION`/`CONTRACT_VERSION`).
- `SidecarTransport` (SPEC-022 behavior 5): `REST`, `SOAP`, `GRAPHQL`,
  `SQL`, `ODBC`, `JDBC`, `CLI`, `FILES`, `EMAIL`, `WEBHOOK`,
  `BROWSER`, `DESKTOP`. The sidecar wraps exactly one transport family
  inside its sandbox; browser and desktop GUI are last resort and
  never hold direct authority.
- `LegacyTransport` (SPEC-022 behavior 5): the legacy source families
  wrapped by the `LegacyPoller` (`REST`, `SOAP`, `SQL`, `CLI`,
  `FILES`, `EMAIL`, `BROWSER`).
- `WebhookDeliveryState` (SPEC-022 behavior 2): `PENDING`,
  `DELIVERED`, `FAILED`, `REPLAY`. Signed webhook delivery states;
  replay detection is part of the contract.
- `WebhookVerification`: `VALID`, `INVALID`, `REPLAY`.

Reused vocabulary (no new names): `ConnectorRuntime`,
`CapabilityClass`, `Idempotency`, `Availability`, `Risk`, `Privacy`,
`PrincipalType`, and the typed IDs `NexusId`, `TenantId`,
`CorrelationId`, `CapabilityId`.

New interface records (not vocabulary classes; snake_case wire-stable
via serde): `ConnectorSdk` trait, `RustConnectorSdk`,
`TypeScriptConnectorSdk`, `PythonConnectorSdk`, `SidecarAdapter`,
`SidecarRequest`, `SidecarResponse`, `LegacyPoller`, `PolledBatch`,
`WebhookNormalizer`, `RawWebhook`, `NormalizedWebhook`,
`WebhookEvent`, `WebhookSignature`, `CredentialBroker`,
`CredentialReference`, `SdkError` (SPEC-006 codes).

## Consequences

- All three SDK bindings implement the same `ConnectorSdk` trait and
  share `CONTRACT_VERSION`, so a conformance corpus can prove
  cross-language parity (SPEC-022 behavior 4).
- The sidecar and legacy poller wrap legacy sources inside a sandbox
  without direct authority; commands stay idempotent and events stay
  versioned with stable cursors.
- Credentials stay in the broker: `CredentialReference` (namespaced
  `vault:`/`broker:`) is the only thing that travels in manifests,
  requests, and telemetry; values never enter logs, prompts, or model
  context (SPEC-020, node contract acceptance obligation 4).
- Errors are typed per SPEC-006 (`SdkErrorCode` with validation,
  authentication, authorization, policy, unavailable, timeout,
  conflict, not found, rate limit, external provider, verification,
  compensation, internal), preserve correlation/actor/tenant/resource
  references, and never carry secrets.
- New synonyms or lifecycle states require an ADR + vocabulary update,
  mirroring ADR-011 through ADR-015.
