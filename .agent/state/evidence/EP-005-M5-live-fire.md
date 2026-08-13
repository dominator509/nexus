# EP-005 M5 Live-Fire Evidence

Generated: 2026-08-12
Node: EP-005 (event nervous system: NATS JetStream, outbox, replay, correlation, durable consumers)
Agent: hermes-nexus-main

## Acceptance obligations and proof

1. Database mutation and outbox insert are atomic
   - `OutboxRepository::append` takes the `nexus-data` `UnitOfWork`
     transaction boundary (crates/nexus-events/src/outbox.rs) so a state
     change and its outbox row commit or roll back together
     (SPEC-023 behavior 1). EP-004 M3 real-postgres tests prove the
     UnitOfWork boundary; EP-005 owns the outbox contract.

2. JetStream publish acknowledgement precedes outbox completion
   - `ep005_integration_publish_ack_precedes_outbox_completion`: real
     nats:2.14.3 container; `NatsEventPublisher::publish` returns Ok only
     after JetStream durable-storage ack. A killed dependency never
     returns Ok (`ep005_failure_unavailable_dependency_on_killed_container`).
   - Command: `cargo test --locked -p nexus-nats ep005_integration`
   - Result: 6 passed (real ephemeral nats:2.14.3 containers, dynamic host ports)

3. Consumers deduplicate and resume after restart
   - `ep005_integration_consumer_after_checkpoint_skips_acked`: durable
     resume from `ConsumerCheckpoint.last_sequence`; nothing after the
     checkpoint is reprocessed.
   - `ep005_integration_consumer_receives_and_explicitly_acks`: explicit
     acks clear server-observed `num_ack_pending` (3 -> 0).
   - `ep005_failure_unacked_messages_remain_pending`: unacked deliveries
     stay pending on the server (fail-closed redelivery, no silent loss).

4. Correlation and causation survive publish, replay, and projection
   - `ep005_integration_envelope_round_trips_fully`: full EventEnvelope
     equality after encode/publish/consume/decode (correlation_id,
     event_type, data_class, payload all intact).

## Forced-failure proofs (real mechanisms, no mocks)

- Container kill mid-operation -> typed UNAVAILABLE, never false Ok
- Corrupt controlled payload -> quarantined, never delivered
- Out-of-namespace subject -> denied by construction (message_count 0)
- Unacked deliveries remain pending on server
- Container cleaned up after failed dependency
- Command: `cargo test --locked -p nexus-nats ep005_failure`
- Result: 5 passed
- Contract-layer failures (11 tests):
  `cargo test --locked -p nexus-events ep005_failure` -> 11 passed

## Operational proof

- Readiness proven by connecting through the PUBLISHED host port
  (dynamic `127.0.0.1::4222` mapping), never localhost assumptions.
- Clean shutdown: `ep005_integration_clean_shutdown_leaves_no_orphans`
  and `ep005_failure_container_cleaned_up_after_failure` assert zero
  leftover containers after drop.
- Diagnostics and bounded recovery: `tests/events/OPS.md`.

## Component pinning

- nats:2.14.3 (pinned VERSIONS.lock.yaml / COMPONENT_REGISTRY.yaml),
  async-nats 0.47.0, tokio 1.x (dev-only; adapter owns no runtime).

## No production deployment

Nothing in this node was deployed. All proofs ran against ephemeral
test containers on dynamically allocated host ports.
