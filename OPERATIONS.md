# OPERATIONS

## Daily operations

Use the Fleet view and `nexusctl status`. Required signals are control-plane health, PostgreSQL replication and disk, NATS stream lag, Temporal backlog, identity status, policy versions, secret lease health, edge reachability, backup age, provider health, cache ratio, costs, incidents, and release drift.

## Common failures

| Symptom | Diagnostic | Safe action |
| --- | --- | --- |
| Dashboard unavailable | Caddy health, control-plane readiness, Keycloak health | Fail over ingress or roll back release |
| Home commands delayed | edge CPU, Home Assistant WebSocket, local policy cache, workflow queue | Use local degraded profile; do not route known commands to cloud |
| DeepSeek latency or errors | provider health, cache hit, gateway circuit, budget | Fail over to certified provider or local route |
| NATS lag | stream status, consumer pending, disk, outbox | Pause noncritical producers and recover consumer |
| Workflow backlog | worker health, task queue, activity errors | Scale or restart workers; workflows remain durable |
| Voice failures | endpoint, mute, VAD, wake, STT, TTS, AEC, Bluetooth | Move session to mobile or API fallback |
| Camera unavailable | VLAN, camera, go2rtc, Frigate, vendor fallback | Notify loss; never expose camera publicly |
| Sentinel alert storm | sensor health, rule version, baseline, duplication | Increase observation, preserve evidence, avoid broad block |
| Backup stale | job, storage health, encryption key, capacity | Run backup and restore verification before update |
| Connector denied | token scope, binding, provider health, certification | Reauthorize least privilege; never use owner token |

## Backup and restore

Nightly encrypted database and configuration backup, configurable artifact backup, periodic full manifest, and weekly automated scratch restore. Monthly operator restore drill and pre-update backup. Recovery keys are stored separately. Restore creates a new deployment identity and reconnects edges through verified trust procedures.

## Scheduled jobs

Memory consolidation, retention, event compaction, workflow schedules, certificate rotation, secret lease rotation, dependency advisories, provider health, social queues, backup, restore verification, object integrity, hardware status, cache analysis, and security baselines.

## Incident triage

1. Acknowledge and preserve incident ID.
2. Determine safety, data, identity, external effects, and affected scope.
3. Contain using the smallest reversible action.
4. Preserve logs, traces, events, configuration, versions, and hashes.
5. Diagnose and reproduce in isolation.
6. Prepare remediation and rollback.
7. Obtain required approval.
8. Canary, verify, promote or roll back.
9. Notify affected users according to policy.
10. Close with cause, controls, memory, tests, and skill candidate where useful.

## Maintenance

Updates are staged, signed, backed up, and reversible. Sidecar updates have separate compatibility matrices. Hardware firmware updates require vendor notes, backup or recovery path, lab canary, and capability retest.

## Operational safety

Never run destructive database, firewall, identity, secret, storage, or device commands outside a documented workflow and approval. Never troubleshoot by exposing private services to the internet.
