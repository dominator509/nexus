# SPEC-007 - Observability, Incident Correlation, and Operations

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define OpenTelemetry signals, GlitchTip incidents, health, fleet status, SLOs, redaction, dashboards, and operator workflows.

## Canonical terms

TraceId, Span, Metric, Structured Log, Incident, SLO, SLI, Health, Readiness, Degraded, Support Bundle. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. All first-party services emit OpenTelemetry traces, metrics, and structured logs with service, version, environment, node, tenant hash, request, correlation, and capability fields.
2. Secrets, tokens, prompts, raw private content, audio, image bytes, and full customer records are never logged.
3. GlitchTip groups application errors and receives release, environment, trace, and redacted user references.
4. Health reports process liveness; readiness reports mandatory dependency ability; capability health reports provider-specific status and certification.
5. Dashboards cover request rate, errors, latency, saturation, workflow backlog, event lag, cache hit ratio, provider cost, action decisions, connector health, security incidents, backup age, and fleet versions.
6. Alerts have owner, severity, threshold, runbook, suppression, test signal, and resolution condition.
7. Support bundles are user-approved, encrypted, redacted, bounded, and show the exact files before export.
8. Telemetry failure cannot block a low-risk local home command, but it marks the system degraded and queues audit synchronization.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Logging prompts by default
- A single opaque health boolean
- Silent alert rules
- Telemetry as source of truth

## Required tests

- Redaction unit and property tests
- OTLP integration
- GlitchTip release test
- Synthetic alert
- Support bundle privacy test
- Telemetry outage degraded-mode test

## Acceptance

An operator can answer within two minutes what is failing, who or what is affected, what changed, and which runbook or rollback applies.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
