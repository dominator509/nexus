# OBSERVABILITY

## Signals

OpenTelemetry traces, metrics, and structured logs are the universal instrumentation format. The collector exports to self-hosted stores by default and optional external sinks through provider adapters. GlitchTip receives release-aware application errors and incident context.

## Required resource fields

`service.name`, `service.version`, `deployment.environment`, `nexus.node_id`, `nexus.node_class`, `nexus.release_profile`, `nexus.capability_id`, `nexus.request_id`, `nexus.correlation_id`, `nexus.workflow_id`, `nexus.provider_id`, and redacted tenant or person references where authorized.

## Redaction

Field names and semantic types for password, secret, token, key, cookie, authorization, prompt, audio, transcript, body, attachment, camera image, private content, and raw provider payload are dropped or hashed before export. Redaction tests operate on string, debug, nested JSON, exception, and span attributes.

## Metrics

- Request rate, errors, latency, saturation.
- Action decisions, approvals, executions, verification mismatches, compensations.
- NATS outbox backlog, publish failures, stream bytes, consumer lag.
- Temporal workflow starts, completion, failures, retries, activity latency, task queue backlog.
- Model cache hit and miss tokens, TTFT, tokens, cost, route, effort, validation failure, fallback.
- Connector health, command latency, rate limit, idempotency conflict, event lag.
- Memory proposals, acceptance, retrieval latency, context size, embedding backlog, deletion backlog.
- Voice wake, false accept, STT latency, TTS latency, interruption, endpoint health.
- Home command latency, state verification, edge offline duration.
- Sentinel alerts, incidents, quarantine, false positive, DNS and flow anomalies.
- Backup age, duration, bytes, restore verification, integrity failure.
- Fleet versions, CPU, RAM, disk, accelerator, temperature, and connectivity.

## SLOs

- Known local home command p95 perceived completion below 1 second on certified edge hardware.
- Control API read p95 below 250 ms within a deployment region.
- Action Gateway p99 below 20 ms excluding external policy dependency outages.
- Event propagation p95 below 500 ms under reference load.
- Cacheable reflex prompt-token hit ratio at least 0.97 over rolling 24 hours.
- Critical audit and action receipt loss is zero.
- Required backup age below 26 hours and weekly restore verification green.

## Dashboards

Golden signals, models and cost, event and workflow, identity and policy, memory and context, home and voice, camera and security, business and social, communications, storage and recovery, fleet and updates, and provider certification.

## Alerts

Every alert has severity, condition, duration, owner, runbook, data classification, deduplication key, suppression, test command, and recovery condition. Synthetic alerts are exercised in EP-038 and EP-043.

## Acceptance

An operator can move from user symptom to trace, event, workflow, action receipt, connector, provider, version, and rollback in under two minutes without seeing secret or private content.
