//! EP-023 M3 integration suite: the REAL nexus-frigate adapter against
//! a REAL Frigate 0.17.2 instance with a REAL controlled media source
//! (FFmpeg canary -> mediamtx RTSP -> go2rtc -> Frigate detect).
//!
//! These tests use the production `RestTransport` + `FrigateAdapter` -
//! no mocks, no fixtures - and assert real provider behavior.
//!
//! Phase-filtered test names let the gate script (ep023-m3-tests.sh)
//! drive state transitions with cargo filters:
//!   - `availability_streaming` / `availability_source_dead` /
//!     `availability_recovered`: source lifecycle truth table
//!   - `restart_same_identity`: Frigate container restart
//!   - everything else: stack-up invariants
//!
//! Env: `FRIGATE_BASE_URL` (e.g. http://127.0.0.1:5000) is REQUIRED.
//!
//! These are LIVE-STACK tests: they require the real Frigate stack and
//! are marked `#[ignore]` so the ambient workspace verify battery
//! (`cargo test --workspace --tests`) stays green without the stack.
//! The M3 gate (scripts/ep023-m3-tests.sh) runs them with `--ignored`
//! against the real container, so the proofs remain mandatory.

use std::env;

use nexus_frigate::FrigateTransport;
use nexus_frigate::{redact_url, CameraAvailability, FrigateAdapter, RestTransport};
use nexus_vision::provider::{CameraProvider, FrigateProvider};
use nexus_vision::stream::VerificationStatus;
use nexus_vision::vocabulary::{CameraCapability, CameraId};

fn base_url() -> String {
    env::var("FRIGATE_BASE_URL").unwrap_or_else(|_| {
        panic!(
            "FRIGATE_BASE_URL is required for ep023_integration tests \
             (start the real Frigate stack via scripts/ep023-m3-tests.sh)"
        )
    })
}

fn adapter() -> FrigateAdapter<RestTransport> {
    FrigateAdapter::new(RestTransport::new(base_url()))
}

fn camera() -> CameraId {
    CameraId::new("nexus_front").expect("canonical camera id")
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_version_matches_pinned() {
    let resp =
        reqwest::blocking::get(format!("{}/api/version", base_url())).expect("GET /api/version");
    assert!(resp.status().is_success());
    let version = resp.text().expect("version body");
    // Frigate reports "0.17.2-<commitsha>" for release builds; the
    // pinned image is 0.17.2 (digest d4351369...7010).
    assert!(
        version.trim().starts_with("0.17.2"),
        "unexpected Frigate version: {version}"
    );
    // The adapter's own health probe must pass against the real API.
    adapter().health().expect("real health probe");
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_discovers_real_camera_with_stable_identity() {
    let adapter = adapter();
    let cameras = adapter.list_cameras().expect("discovery");
    assert!(
        cameras.contains(&camera()),
        "real config must contain nexus_front, got {cameras:?}"
    );
    let again = adapter.list_cameras().expect("discovery again");
    assert_eq!(cameras, again);
    assert_eq!(camera().as_str(), "nexus_front");
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_capabilities_from_real_config() {
    let adapter = adapter();
    let caps = adapter.capabilities(&camera()).expect("capabilities");
    assert!(caps.contains(&CameraCapability::ObjectDetection));
    assert!(caps.contains(&CameraCapability::Recording));
    assert!(caps.contains(&CameraCapability::LiveStream));
    // Two-way audio is NEVER advertised from metadata (directive M).
    assert!(!caps.contains(&CameraCapability::TwoWayAudio));
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_availability_streaming_with_live_producer() {
    let adapter = adapter();
    let state = adapter.availability(&camera()).expect("availability");
    assert_eq!(
        state,
        CameraAvailability::Streaming,
        "real go2rtc producer must be LIVE while the source is running"
    );
    let stream = adapter.stream(&camera()).expect("stream ref");
    assert_eq!(stream.status, VerificationStatus::Unverified);
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_availability_source_dead_never_streaming() {
    // Run only while the FFmpeg source is stopped (gate phase 2). The
    // real go2rtc producer for a dead source is a bare {"url": ...}
    // entry -> adapter must report DEGRADED, never STREAMING.
    let adapter = adapter();
    let state = adapter.availability(&camera()).expect("availability");
    assert_ne!(
        state,
        CameraAvailability::Streaming,
        "dead source must never be reported STREAMING"
    );
    assert_eq!(state, CameraAvailability::Degraded);
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_availability_recovered_streaming() {
    // Run only after the source restarts (gate phase 3).
    let adapter = adapter();
    let state = adapter.availability(&camera()).expect("availability");
    assert_eq!(state, CameraAvailability::Streaming);
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_snapshot_is_real_jpeg() {
    let mut transport = RestTransport::new(base_url());
    let bytes = transport
        .latest_frame("nexus_front")
        .expect("latest.jpg through production transport");
    assert!(
        bytes.len() > 1000,
        "snapshot too small: {} bytes",
        bytes.len()
    );
    assert_eq!(&bytes[0..3], &[0xFF, 0xD8, 0xFF], "not a JPEG");
    let out = "/tmp/ep023-m3-snapshot.jpg";
    std::fs::write(out, &bytes).expect("write snapshot");
    eprintln!("snapshot bytes={} written={}", bytes.len(), out);
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_events_api_is_real() {
    let adapter = adapter();
    let events = adapter.events(&camera(), 0).expect("real events query");
    eprintln!("ep023 events returned {} real records", events.len());
    for event in events {
        assert!(event.confidence > 0.0 && event.confidence <= 1.0);
    }
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_restart_same_identity_and_snapshot() {
    // Run only after `docker restart` of the Frigate container (gate
    // phase 4). The same camera must map to the same canonical CameraId
    // and a fresh snapshot must succeed (directive J).
    let adapter = adapter();
    let cameras = adapter.list_cameras().expect("rediscovery");
    assert!(cameras.contains(&camera()));
    let state = adapter.availability(&camera()).expect("availability");
    assert_eq!(state, CameraAvailability::Streaming);
    let mut transport = RestTransport::new(base_url());
    let bytes = transport
        .latest_frame("nexus_front")
        .expect("snapshot after restart");
    assert_eq!(&bytes[0..3], &[0xFF, 0xD8, 0xFF], "not a JPEG");
    let out = "/tmp/ep023-m3-snapshot-restart.jpg";
    std::fs::write(out, &bytes).expect("write snapshot");
    eprintln!("restart snapshot bytes={}", bytes.len());
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m3-tests.sh"]
fn ep023_integration_frigate_redaction_under_real_config() {
    let secret = "m3secret";
    let url = format!("rtsp://m3user:{secret}@192.0.2.55:554/never");
    let redacted = redact_url(&url);
    assert!(
        !redacted.contains(secret),
        "redacted url leaked: {redacted}"
    );
    assert!(
        redacted.contains("***@"),
        "redacted url malformed: {redacted}"
    );

    // The never-connecting camera (test-only credentials in real
    // config) is configured with NO go2rtc stream declared: truthful
    // state is Available (configured + provider healthy, no stream
    // declared), never Streaming from metadata alone and never forced
    // DEGRADED merely because another test expected it (directive H).
    // Frigate normalizes the config surface: roles [] -> [record,
    // detect] and live.streams auto-populates for cameras with ffmpeg
    // inputs, so LiveStream IS declared. What must never happen is an
    // overclaim: disabled detect/record must not surface as
    // ObjectDetection/Recording, and TwoWayAudio is never advertised
    // from config (directive M).
    let dead_camera = CameraId::new("nexus_secure").expect("id");
    let state = adapter().availability(&dead_camera).expect("state");
    assert_eq!(state, CameraAvailability::Available);
    let caps = adapter().capabilities(&dead_camera).expect("caps");
    assert!(
        caps.contains(&CameraCapability::LiveStream),
        "Frigate declares live.streams for nexus_secure; caps={caps:?}"
    );
    assert!(
        !caps.contains(&CameraCapability::ObjectDetection),
        "detect disabled must not advertise ObjectDetection; caps={caps:?}"
    );
    assert!(
        !caps.contains(&CameraCapability::Recording),
        "record disabled must not advertise Recording; caps={caps:?}"
    );
    assert!(
        !caps.contains(&CameraCapability::TwoWayAudio),
        "TwoWayAudio never from config; caps={caps:?}"
    );
}
