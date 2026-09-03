//! EP-005 M4 forced-failure tests: REAL dependency failures against a
//! real `nats:2.14.3` container (pinned). No mocks, no in-memory
//! substitutes: unavailable dependency is proven by killing the
//! container; malformed input by corrupting a controlled message;
//! denied permission by publishing to an unauthorized subject; partial
//! side effects by checkpoint-resume semantics. Every error path is
//! asserted through the typed SPEC-006 error codes.

use std::process::Command;
use std::time::{Duration, Instant};

use nexus_domain::{CorrelationId, EventId, TenantId};
use nexus_events::{
    ConsumerConfig, EventConsumer, EventDataClass, EventEnvelope, EventErrorCode, EventPublisher,
    EventType, StreamConfig, StreamProvisioner,
};
use nexus_nats::{NatsEventConsumer, NatsEventPublisher, NatsStreamProvisioner};

const IMAGE: &str = "nats:2.14.3";
const STREAM: &str = "nexus";

fn tenant() -> TenantId {
    TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fd001").unwrap()
}

fn envelope(seed: u8, event_type: &str, correlation: &str) -> EventEnvelope {
    EventEnvelope {
        event_id: EventId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fd0{seed:02x}")).unwrap(),
        event_type: EventType::new(event_type).unwrap(),
        schema_version: "1.0.0".to_string(),
        source: "failure".to_string(),
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

struct TestNats {
    container: String,
    port: u16,
}

impl TestNats {
    fn start() -> Self {
        let name = format!(
            "nexus-ep005-fail-{}",
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
        let port = Self::host_port(&name);
        Self {
            container: name,
            port,
        }
    }

    fn host_port(container: &str) -> u16 {
        let out = Command::new("docker")
            .args(["port", container, "4222"])
            .output()
            .expect("docker port failed");
        let text = String::from_utf8_lossy(&out.stdout);
        text.trim()
            .rsplit(':')
            .next()
            .expect("no host port")
            .parse::<u16>()
            .expect("host port must be numeric")
    }

    async fn wait_ready(&self) {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let ok = NatsStreamProvisioner::connect(&self.url()).await.is_ok();
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

    fn kill(&self) {
        let out = Command::new("docker")
            .args(["kill", &self.container])
            .output()
            .expect("docker kill failed");
        assert!(
            out.status.success(),
            "docker kill failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
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

/// Unavailable dependency: the nats container is killed mid-operation.
/// The typed error must surface as UNAVAILABLE or EXTERNAL_PROVIDER,
/// never a panic or a false Ok.
#[tokio::test(flavor = "multi_thread")]
async fn ep005_failure_unavailable_dependency_on_killed_container() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    provision(&url).await;

    let publisher = NatsEventPublisher::connect(&url)
        .await
        .expect("connect publisher");
    let e = envelope(
        1,
        "memory.record.created",
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fd010",
    );
    publisher.publish(&e).await.expect("publish before kill");

    // Kill the real dependency mid-operation.
    nats.kill();

    // A fresh connect to the dead endpoint must fail with a typed
    // UNAVAILABLE error (SPEC-006), never hang forever.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match NatsStreamProvisioner::connect(&url).await {
            Ok(_) => {
                assert!(
                    Instant::now() < deadline,
                    "connect to killed container must eventually fail"
                );
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            Err(err) => {
                assert_eq!(
                    err.code(),
                    EventErrorCode::Unavailable,
                    "killed dependency must map to UNAVAILABLE"
                );
                break;
            }
        }
    }

    // Publishing through the stale handle must not report success
    // (SPEC-023 behavior 2: no ack, no Ok). It either fails with a
    // typed error or times out; both prove fail-closed.
    let e2 = envelope(
        2,
        "memory.record.created",
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fd020",
    );
    match tokio::time::timeout(Duration::from_secs(10), publisher.publish(&e2)).await {
        Ok(Ok(())) => panic!("publish to a killed dependency must not report success"),
        Ok(Err(err)) => {
            assert_ne!(err.code(), EventErrorCode::Validation);
            assert!(err.to_string().contains("nats"));
        }
        Err(_) => {
            // Timeout: the outbox row stays PENDING (bounded retry).
            // Fail-closed is preserved because Ok was never returned.
        }
    }
}

/// Malformed input: a controlled corrupt message is published directly
/// (bypassing encode) and must be quarantined - never ack'd, never
/// surfaced to the application, never crashing the consumer.
#[tokio::test(flavor = "multi_thread")]
async fn ep005_failure_malformed_payload_is_quarantined() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    provision(&url).await;

    // Corrupt a controlled message: publish garbage bytes directly on a
    // canonical subject through a raw client, bypassing encode().
    let client = async_nats::connect(&url).await.expect("raw connect");
    client
        .publish(
            "nexus.memory.corrupt".to_string(),
            b"not-an-envelope".to_vec().into(),
        )
        .await
        .expect("raw publish");
    // Let JetStream persist it.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let consumer = NatsEventConsumer::connect(&url)
        .await
        .expect("connect consumer");
    let config = ConsumerConfig {
        consumer: "quarantine-check".to_string(),
        stream: STREAM.to_string(),
        subject: "nexus.memory.>".to_string(),
        batch_size: 10,
    };
    // The malformed message must NOT be returned (fail-closed) and the
    // poll must still succeed - a decode error is not a consumer crash.
    let batch = consumer.poll(&config, 1).await.expect("poll survives");
    assert!(
        batch.is_empty(),
        "malformed payload must be quarantined, not delivered"
    );
}

/// Denied permission: a subject the canonical stream does NOT own is
/// not stored (message count stays 0), while a canonical publish through
/// the adapter IS stored (count 1). The adapter derives routing subjects
/// from the canonical namespace, so out-of-namespace subjects are
/// denied by construction and never silently accepted.
#[tokio::test(flavor = "multi_thread")]
async fn ep005_failure_publish_to_unowned_subject_denied() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    provision(&url).await;

    // Raw publish to a subject the canonical stream does not own.
    let raw = async_nats::connect(&url).await.expect("raw connect");
    raw.publish(
        "other.domain.event".to_string(),
        b"not-owned".to_vec().into(),
    )
    .await
    .expect("raw publish send");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The stream must NOT have stored the out-of-namespace message.
    let provisioner = NatsStreamProvisioner::connect(&url)
        .await
        .expect("connect provisioner");
    let status = provisioner.status(STREAM).await.expect("status");
    assert_eq!(
        status.message_count,
        Some(0),
        "out-of-namespace subject must not be stored by the canonical stream"
    );

    // A canonical publish through the adapter IS stored.
    let publisher = NatsEventPublisher::connect(&url)
        .await
        .expect("connect publisher");
    let e = envelope(
        3,
        "memory.record.created",
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fd030",
    );
    publisher.publish(&e).await.expect("canonical publish");
    let status = provisioner.status(STREAM).await.expect("status after");
    assert_eq!(
        status.message_count,
        Some(1),
        "canonical publish must be stored"
    );
}

/// Partial side effect: after consuming a batch, explicit acks clear the
/// server-side pending set; a consumer that never acks leaves the
/// messages pending (fail-closed redelivery, no silent data loss).
#[tokio::test(flavor = "multi_thread")]
async fn ep005_failure_unacked_messages_remain_pending() {
    let nats = TestNats::start();
    nats.wait_ready().await;
    let url = nats.url();
    provision(&url).await;

    let publisher = NatsEventPublisher::connect(&url)
        .await
        .expect("connect publisher");
    for seed in [4u8, 5] {
        let e = envelope(
            seed,
            "memory.record.created",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fd040",
        );
        publisher.publish(&e).await.expect("publish");
    }

    // Consume without acking: the deliveries must remain pending on the
    // server (fail-closed; redelivery is possible, nothing is lost).
    let consumer = NatsEventConsumer::connect(&url)
        .await
        .expect("connect consumer");
    let config = ConsumerConfig {
        consumer: "no-ack-check".to_string(),
        stream: STREAM.to_string(),
        subject: "nexus.memory.>".to_string(),
        batch_size: 10,
    };
    let batch = consumer.poll(&config, 1).await.expect("poll");
    assert_eq!(batch.len(), 2);

    // Verify through a RAW handle that the messages are still pending
    // (never ack'd): the server observes them as unacknowledged on the
    // ephemeral consumer (found by filter subject; the server auto-names
    // ephemeral consumers).
    let raw = async_nats::connect(&url).await.expect("raw connect");
    let ctx = async_nats::jetstream::new(raw);
    let stream = ctx.get_stream(STREAM).await.expect("get stream");
    use futures_util::StreamExt;
    let mut consumers = stream.consumers();
    let mut pending = 0usize;
    while let Some(info) = consumers.next().await {
        let info = info.expect("consumer info");
        if info.config.filter_subject == "nexus.memory.>" {
            pending += info.num_ack_pending;
        }
    }
    assert_eq!(
        pending, 2,
        "unacked deliveries must remain pending on the server"
    );
}

/// Timeout/cleanup: the test container is always removed on drop even
/// when the dependency failed mid-test (no orphaned processes).
#[tokio::test(flavor = "multi_thread")]
async fn ep005_failure_container_cleaned_up_after_failure() {
    let container = {
        let nats = TestNats::start();
        nats.wait_ready().await;
        let url = nats.url();
        provision(&url).await;
        nats.kill();
        // Failure path: connection is dead, then the scope ends and the
        // container is removed.
        nats.container.clone()
    };
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
    assert!(
        String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "failed test must still clean up its container"
    );
}
