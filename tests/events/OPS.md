# EP-005 M4 - Event Nervous System Operations Diagnostic & Bounded Recovery

Owned component: NATS JetStream event bus (`nats:2.14.3`, pinned in
`VERSIONS.lock.yaml` / `COMPONENT_REGISTRY.yaml`).

## Diagnostics

Run these from the repository root with the canonical environment
(`. scripts/env.sh`):

1. **Container health**
   `docker ps --filter name=nexus`

2. **Server reachability (through the published host port)**
   `cargo test --locked -p nexus-nats ep005_integration_stream_provisioning_is_idempotent`

3. **Canonical stream status** (stream `nexus`, subjects `nexus.>`)
   The integration suite `ep005_integration_stream_provisioning_is_idempotent`
   provisions idempotently and asserts `exists == true`.

4. **Pending deliveries (fail-closed redelivery check)**
   `cargo test --locked -p nexus-nats ep005_failure_unacked_messages_remain_pending`
   asserts unacked deliveries stay pending on the server.

## Bounded Recovery

- **Single restart (bounded, 1 attempt):** `docker restart <container>`.
- **Re-provision stream (idempotent):** re-run
  `cargo test --locked -p nexus-nats ep005_integration_stream_provisioning_is_idempotent`.
- **Checkpoint resume:** consumers resume from the application's durable
  checkpoint (`ConsumerCheckpoint.last_sequence`); re-running the
  consumer reprocesses nothing already acked
  (`ep005_integration_consumer_after_checkpoint_skips_acked`).

Recovery is always bounded: one restart, idempotent re-provision, and
checkpoint-driven resume. No unbounded retry loops; the outbox applies
bounded retry with redacted failure reasons (`OutboxRecord::fail`).

## Failure evidence

- `crates/nexus-events/tests/failure.rs` - contract-layer failures
  (malformed input, duplicate state, denied codes, redaction, bounded
  retry). Run: `cargo test --locked -p nexus-events ep005_failure`.
- `infra/nats/tests/failure_nats.rs` - real-dependency failures against
  `nats:2.14.3` (container kill, corrupt message quarantine, unowned
  subject denial, unacked pending deliveries, cleanup on failure).
  Run: `cargo test --locked -p nexus-nats ep005_failure`.
