# CAPABILITY TAXONOMY

## Naming

Capabilities use lowercase dotted names: `<domain>.<resource>.<verb>`. Vendor names are forbidden in domain capability names. Examples: `home.light.set`, `crm.opportunity.move_stage`, `email.message.send`, `agent.code.implement`, `network.device.quarantine`.

## Classes

- Query: read-only, no external mutation.
- Command: bounded mutation with idempotency and verification.
- Workflow: durable multi-step objective, possibly with approvals.
- Stream: subscribed events or media.
- Administrative: configuration, identity, permissions, secrets, or provider lifecycle.

## Required descriptor fields

Stable ID, semantic version, description, class, input and output schema URIs, required scopes, allowed principals, risk class, approval class, reversal class, idempotency behavior, timeout, concurrency, data classes, locality, network access, provider health, cost policy, event types, and certification state.

## Risk classes

- R0 observation: public or non-sensitive read.
- R1 low: reversible household or personal operation.
- R2 moderate: external communication, persistent change, or limited business mutation.
- R3 high: locks, alarms, purchases, production deployment, sensitive disclosure, account changes, or broad external messaging.
- R4 critical: money movement, credential and permission administration, destructive data operation, legal attestation, life-safety, or robot motion with material hazard.

R3 and R4 require cryptographic step-up unless a narrower policy explicitly permits a time-bound preauthorization. R4 cannot be approved by an LLM or voice evidence.
