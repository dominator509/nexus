//! EP-005 M3 integration tests: event nervous system through REAL NATS
//! JetStream 2.14.3 (pinned VERSIONS.lock.yaml, COMPONENT_REGISTRY.yaml).
//!
//! Uses the `nats:2.14.3` image in a real ephemeral container - never an
//! in-memory substitute. Readiness is proven by connecting through the
//! PUBLISHED HOST PORT. Host ports are dynamically allocated so parallel
//! runs never collide. These tests exercise the nexus-events ports
//! through the `nexus-nats` adapter: stream provisioning, publish with
//! ack, consumption with durable checkpoints, explicit acknowledgement,
//! and correlation survival.
//!
//! Runtime: each test drives the adapter from a real Tokio multi-thread
//! runtime owned by the test harness (the composition root in
//! production). The adapter never owns a runtime.

use std::process::Command;
use std::time::{Duration, Instant};

use nexus_domain::{CorrelationId, EventId, TenantId};
use nexus_events::{
    ConsumerCheckpoint, ConsumerConfig, EventConsumer, EventDataClass, EventEnvelope,
    EventPublisher, EventType, StreamConfig, StreamProvisioner,
};
use nexus_nats::{NatsEventConsumer, NatsEventPublisher, NatsStreamProvisioner};

const IMAGE: &str = "nats:2.14.3";
const STREAM: &str = "nexus";

fn tenant() -> TenantId {
    TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc001").unwrap()
}

fn envelope(seed: u8, event_type: &str, correlation: &str) -> EventEnvelope {
    EventEnvelope {
        event_id: EventId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc0{seed:02x}")).unwrap(),
        event_type: EventType::new(event_type).unwrap(),
        schema_version: "1.0.0".to_string(),
        source: "integration".to_string(),
        subject: "ignored".to_string(),
        time: "2026-08-12T00:00:00Z".to_string(),
        tenant_id: tenant(),
        actor: "principal".to_string(),
        correlation_id: CorrelationId::new(correlation).unwrap(),
        causation_id: None,
        data_class: EventDataClass::Household,
        payload: serde_json::json!({ "seed": seed }),
    }
}

/// A running ephemeral nats-server with a dynamically published host port.
struct TestNats {
    container: String,
    port: u16,
}

impl TestNats {
    fn start() -> Self {
        let name = format!(
            "nexus-ep005-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let out = Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                &name,
                "-p",
                "127.0.0.1::4222",
                IMAGE,
                "-js",
            ])
            .output()
            .expect("docker run failed");
        assert!(
            out.status.success(),
            "docker run failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let container = name;
        let port = Self::host_port(&container);
        Self { container, port }
        // Callers await wait_ready() on the test runtime before use.
    }

    fn host_port(container: &str) -> u16 {
        let out = Command::new("docker")
            .args(["port", container, "4222"])
            .output()
            .expect("docker port failed");
        assert!(
            out.status.success(),
            "docker port failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8_lossy(&out.stdout);
        let port = text
            .trim()
            .rsplit(':')
            .next()
            .expect("no host port")
            .parse::<u16>()
            .expect("host port must be numeric");
        assert!(port > 0, "host port must not be 0");
        port
    }

    async fn wait_ready(&self) {
        // Prove readiness by connecting through the PUBLISHED host port.
        // Runs on the test's multi-thread Tokio runtime (composition
        // root); no nested runtime is ever created.
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let url = format!("127.0.0.1:{}", self.port);
            let ok = NatsStreamProvisioner::connect(&url).await.is_ok();
            if ok {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "nats did not become ready within 60s"
            );
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    fn url(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }
}

impl Drop for TestNats {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container])
            .output();
    }
}

async fn provision(url: &str) -> nexus_events::StreamStatus {
    let provisioner = NatsStreamProvisioner::connect(url)
        .await
        .expect("connect provisioner");
    let config = StreamConfig {
        stream: STREAM.to_string(),
        subjects: vec!["nexus.>".to_string()],
        max_messages: 100_000,
        max_age_seconds: 86_400,
    };
    provisioner
        .ensure_stream(&config)
        .await
        .expect("ensure stream")
}

/// Server-observed number of delivered-but-unacknowledged messages for
/// the stream consumer matching `subject`, read through a raw async-nats
/// handle (not the adapter) so the proof is independent of the code under
/// test. `poll` creates ephemeral consumers auto-named by the server, so
/// the consumer is found by its filter subject rather than a name.
async fn ack_pending(url: &str, subject: &str) -> usize {
    let client = async_nats::connect(url).await.expect("raw connect");
    let ctx = async_nats::jetstream::new(client);
    let stream = ctx.get_stream(STREAM).await.expect("raw get stream");
    use futures_util::StreamExt;
    let mut consumers = stream.consumers();
    let mut total = 0usize;
    while let Some(info) = consumers.next().await {
        let info = info.expect("raw consumer info");
        if info.config.filter_subject == subject {
            total += info.num_ack_pending as usize;
        }
    }
    total
}

#[tokio::test(flavor = "multi_thread")]
async fn ep005_integration_stream_provisioning_is_idempotent() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    let first = provision(&url).await;
    assert!(first.exists, "stream must exist after provisioning");
    assert_eq!(first.stream, STREAM);
    // Second provisioning is idempotent (get_or_create).
    let second = provision(&url).await;
    assert!(second.exists, "stream must still exist");
    // Status reports the same stream.
    let provisioner = NatsStreamProvisioner::connect(&url)
        .await
        .expect("connect provisioner");
    let status = provisioner.status(STREAM).await.expect("status");
    assert!(status.exists);
}

#[tokio::test(flavor = "multi_thread")]
async fn ep005_integration_publish_ack_precedes_outbox_completion() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    provision(&url).await;
    let publisher = NatsEventPublisher::connect(&url)
        .await
        .expect("connect publisher");
    // Ok means JetStream acknowledged durable storage (SPEC-023
    // behavior 2); a failure would keep the outbox row PENDING.
    let e = envelope(
        1,
        "memory.record.created",
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc010",
    );
    publisher.publish(&e).await.expect("publish must ack");
}

#[tokio::test(flavor = "multi_thread")]
async fn ep005_integration_consumer_receives_and_explicitly_acks() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    provision(&url).await;
    let publisher = NatsEventPublisher::connect(&url)
        .await
        .expect("connect publisher");
    // Publish three events.
    for seed in [2u8, 3, 4] {
        let e = envelope(
            seed,
            "memory.record.created",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc020",
        );
        publisher.publish(&e).await.expect("publish");
    }
    // First poll from sequence 1: deliver all three, in order.
    let consumer = NatsEventConsumer::connect(&url)
        .await
        .expect("connect consumer");
    let config = ConsumerConfig {
        consumer: "memory-indexer".to_string(),
        stream: STREAM.to_string(),
        subject: "nexus.memory.>".to_string(),
        batch_size: 10,
    };
    let batch = consumer.poll(&config, 1).await.expect("first poll");
    assert_eq!(batch.len(), 3, "all three events must be delivered");
    assert_eq!(batch[0].payload["seed"], 2);
    assert_eq!(batch[2].payload["seed"], 4);

    // Before explicit acks the server observes three pending deliveries
    // on the ephemeral consumer (found by filter subject).
    let before = ack_pending(&url, "nexus.memory.>").await;
    assert_eq!(
        before, 3,
        "three deliveries must be pending explicit acknowledgement"
    );

    // Explicitly acknowledge each event through the adapter port.
    for e in &batch {
        consumer
            .ack(&config.consumer, e.event_id.as_str())
            .await
            .expect("ack must reach the server");
    }

    // After explicit acks the server observes zero pending deliveries.
    let after = ack_pending(&url, "nexus.memory.>").await;
    assert_eq!(
        after, 0,
        "explicit acks must clear all pending deliveries on the server"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ep005_integration_envelope_round_trips_fully() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    provision(&url).await;
    let publisher = NatsEventPublisher::connect(&url)
        .await
        .expect("connect publisher");
    let e = envelope(
        5,
        "business.contract.signed",
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc030",
    );
    publisher.publish(&e).await.expect("publish");
    let consumer = NatsEventConsumer::connect(&url)
        .await
        .expect("connect consumer");
    let config = ConsumerConfig {
        consumer: "contract-indexer".to_string(),
        stream: STREAM.to_string(),
        subject: "nexus.business.>".to_string(),
        batch_size: 10,
    };
    let batch = consumer.poll(&config, 1).await.expect("poll");
    assert_eq!(batch.len(), 1);
    // Full envelope equality: encode/publish/consume/decode is lossless.
    assert_eq!(
        batch[0], e,
        "envelope must survive encode/publish/consume/decode intact"
    );
    assert_eq!(
        batch[0].correlation_id.as_str(),
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc030"
    );
    assert_eq!(batch[0].event_type.as_str(), "business.contract.signed");
    assert_eq!(batch[0].data_class, EventDataClass::Household);
}

#[tokio::test(flavor = "multi_thread")]
async fn ep005_integration_checkpoint_persists_and_resume_skips_processed() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    provision(&url).await;
    let publisher = NatsEventPublisher::connect(&url)
        .await
        .expect("connect publisher");
    for seed in [6u8, 7, 8] {
        let e = envelope(
            seed,
            "memory.record.created",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc040",
        );
        publisher.publish(&e).await.expect("publish");
    }
    let config = ConsumerConfig {
        consumer: "resume-check".to_string(),
        stream: STREAM.to_string(),
        subject: "nexus.memory.>".to_string(),
        batch_size: 10,
    };
    // First pass consumes everything and persists a real checkpoint.
    let consumer = NatsEventConsumer::connect(&url)
        .await
        .expect("connect consumer");
    let first = consumer.poll(&config, 1).await.expect("first poll");
    assert_eq!(first.len(), 3);
    consumer
        .save_checkpoint(&ConsumerCheckpoint {
            consumer: config.consumer.clone(),
            stream: config.stream.clone(),
            subject: config.subject.clone(),
            last_sequence: 3,
        })
        .await
        .expect("save checkpoint must persist to the KV store");
    for e in &first {
        consumer
            .ack(&config.consumer, e.event_id.as_str())
            .await
            .expect("ack");
    }
    drop(consumer);

    // Restart: a fresh connection reads the durable checkpoint from the
    // server-side KV store and resumes AFTER it - nothing new is
    // delivered (SPEC-023 behavior 4: resume from the last checkpoint).
    let restarted = NatsEventConsumer::connect(&url)
        .await
        .expect("reconnect consumer");
    let cp = restarted
        .checkpoint(&config.consumer)
        .await
        .expect("read checkpoint")
        .expect("checkpoint must exist after save");
    assert_eq!(cp.consumer, "resume-check");
    assert_eq!(cp.last_sequence, 3);
    let resumed = restarted
        .poll(&config, cp.last_sequence + 1)
        .await
        .expect("resume poll");
    assert_eq!(resumed.len(), 0, "nothing new after the checkpoint");
}

#[tokio::test(flavor = "multi_thread")]
async fn ep005_integration_checkpoint_round_trips_and_overwrites() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    provision(&url).await;
    let consumer = NatsEventConsumer::connect(&url)
        .await
        .expect("connect consumer");

    // A consumer with no saved checkpoint reads None (start from the
    // beginning) - never a fabricated zero.
    assert!(
        consumer
            .checkpoint("fresh-consumer")
            .await
            .expect("missing checkpoint")
            .is_none(),
        "unsaved checkpoint must read as None"
    );

    let cp1 = ConsumerCheckpoint {
        consumer: "round-trip".to_string(),
        stream: STREAM.to_string(),
        subject: "nexus.memory.>".to_string(),
        last_sequence: 7,
    };
    consumer.save_checkpoint(&cp1).await.expect("save cp1");
    let got = consumer
        .checkpoint("round-trip")
        .await
        .expect("read cp1")
        .expect("cp1 must exist");
    assert_eq!(got, cp1, "checkpoint must round-trip exactly");

    // Overwrite advances the checkpoint.
    let cp2 = ConsumerCheckpoint {
        last_sequence: 42,
        ..cp1.clone()
    };
    consumer.save_checkpoint(&cp2).await.expect("save cp2");
    let got = consumer
        .checkpoint("round-trip")
        .await
        .expect("read cp2")
        .expect("cp2 must exist");
    assert_eq!(got, cp2, "overwrite must advance the checkpoint");

    // Durability across a fresh connection: the store is server-side.
    let restarted = NatsEventConsumer::connect(&url)
        .await
        .expect("reconnect consumer");
    let got = restarted
        .checkpoint("round-trip")
        .await
        .expect("read after restart")
        .expect("checkpoint must survive restart");
    assert_eq!(got.last_sequence, 42);
}

#[tokio::test(flavor = "multi_thread")]
async fn ep005_integration_clean_shutdown_leaves_no_orphans() {
    let container = {
        let nats = TestNats::start();
        nats.wait_ready().await;
        let url = nats.url();
        provision(&url).await;
        let publisher = NatsEventPublisher::connect(&url)
            .await
            .expect("connect publisher");
        let consumer = NatsEventConsumer::connect(&url)
            .await
            .expect("connect consumer");
        let e = envelope(
            9,
            "memory.record.created",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc050",
        );
        publisher.publish(&e).await.expect("publish");
        let config = ConsumerConfig {
            consumer: "shutdown-check".to_string(),
            stream: STREAM.to_string(),
            subject: "nexus.memory.>".to_string(),
            batch_size: 10,
        };
        let batch = consumer.poll(&config, 1).await.expect("poll");
        assert_eq!(batch.len(), 1);
        for e in &batch {
            consumer
                .ack(&config.consumer, e.event_id.as_str())
                .await
                .expect("ack");
        }
        // Drop adapter handles, then drop the container by ending the
        // scope. The test runtime is dropped after this test returns;
        // no consumer task or container may be left behind.
        drop(publisher);
        drop(consumer);
        nats.container.clone()
    };
    // Prove no orphaned test container remains after clean shutdown.
    let out = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name={container}"),
            "--format",
            "{{.Names}}",
        ])
        .output()
        .expect("docker ps failed");
    let leftovers = String::from_utf8_lossy(&out.stdout);
    assert!(
        leftovers.trim().is_empty(),
        "orphaned test container still present: {leftovers}"
    );
}
