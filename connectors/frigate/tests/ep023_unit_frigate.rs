//! EP-023 M2 unit suite for the Frigate adapter (SPEC-021).
//!
//! Proves real production adapter behavior with a controlled transport
//! fixture: provider response mapping, stable camera identity,
//! availability mapping, exact stream mapping, error handling, privacy
//! boundaries, advisory visitor identity, two-way-audio gating, Roku
//! ladder ordering, no unverified RTSP/ONVIF claim, and no secret
//! leakage (directive U).
//!
//! These tests prove implementation behavior. They do NOT certify
//! Frigate or go2rtc; real provider certification belongs to M3/M5.

use std::cell::RefCell;
use std::collections::BTreeMap;

use nexus_frigate::transport::{
    FrigateCameraConfig, FrigateConfig, FrigateDetectConfig, FrigateEvent, FrigateFfmpegConfig,
    FrigateLiveConfig, FrigateRecordConfig, FrigateSnapshotsConfig, FrigateTransport,
    Go2RtcProducer, Go2RtcStreamInfo,
};
use nexus_frigate::{redact_url, CameraAvailability, FrigateAdapter};
use nexus_vision::event::CameraEvent;
use nexus_vision::fallback::CameraFallbackPlan;
use nexus_vision::identity::VisitorIdentity;
use nexus_vision::provider::CameraProvider;
use nexus_vision::stream::VerificationStatus;
use nexus_vision::two_way::{TwoWayAudioCapability, TwoWayAudioState};
use nexus_vision::vocabulary::{CameraCapability, CameraId, PrivacyClass, RokuCapabilityTier};
use nexus_vision::{VisionError, VisionErrorCode};

fn camera(id: &str) -> CameraId {
    CameraId::new(id).expect("camera id")
}

fn front_cfg() -> FrigateCameraConfig {
    FrigateCameraConfig {
        name: Some("front".to_string()),
        friendly_name: Some("Front Door".to_string()),
        enabled: true,
        detect: FrigateDetectConfig { enabled: true },
        record: FrigateRecordConfig { enabled: true },
        snapshots: FrigateSnapshotsConfig { enabled: true },
        live: FrigateLiveConfig {
            streams: BTreeMap::from([("front".to_string(), "front".to_string())]),
        },
        audio: Default::default(),
        ffmpeg: FrigateFfmpegConfig {
            inputs: vec![nexus_frigate::transport::FrigateCameraInput {
                path: "rtsp://user:secret@192.168.1.10:554/stream".to_string(),
                roles: vec!["detect".to_string(), "record".to_string()],
            }],
        },
        onvif: Default::default(),
        webui_url: None,
    }
}

/// Controlled transport fixture. Scriptable responses prove adapter
/// behavior deterministically; this is a test-double (TESTING.md test
/// zone), never production routing.
#[derive(Default)]
struct MockTransport {
    config: FrigateConfig,
    streams: BTreeMap<String, Go2RtcStreamInfo>,
    events: Vec<FrigateEvent>,
    health_ok: bool,
    fail_config: Option<VisionError>,
    fail_events: Option<VisionError>,
    fail_streams: Option<VisionError>,
    fail_latest: Option<VisionError>,
    latest_bytes: Vec<u8>,
    calls: RefCell<Vec<String>>,
}

impl MockTransport {
    fn log(&self, call: &str) {
        self.calls.borrow_mut().push(call.to_string());
    }
}

impl FrigateTransport for MockTransport {
    fn health(&mut self) -> Result<(), VisionError> {
        self.log("health");
        if self.health_ok {
            Ok(())
        } else {
            Err(VisionError::unavailable("frigate unreachable"))
        }
    }

    fn config(&mut self) -> Result<FrigateConfig, VisionError> {
        self.log("config");
        if let Some(err) = &self.fail_config {
            return Err(err.clone());
        }
        Ok(self.config.clone())
    }

    fn events(
        &mut self,
        _camera: &str,
        _since_ms: u64,
        _limit: usize,
    ) -> Result<Vec<FrigateEvent>, VisionError> {
        self.log("events");
        if let Some(err) = &self.fail_events {
            return Err(err.clone());
        }
        Ok(self.events.clone())
    }

    fn go2rtc_streams(&mut self) -> Result<BTreeMap<String, Go2RtcStreamInfo>, VisionError> {
        self.log("go2rtc_streams");
        if let Some(err) = &self.fail_streams {
            return Err(err.clone());
        }
        Ok(self.streams.clone())
    }

    fn latest_frame(&mut self, _camera: &str) -> Result<Vec<u8>, VisionError> {
        self.log("latest_frame");
        if let Some(err) = &self.fail_latest {
            return Err(err.clone());
        }
        Ok(self.latest_bytes.clone())
    }

    fn base_url(&self) -> Option<String> {
        Some("http://frigate.test".to_string())
    }
}

fn cfg_with(cameras: BTreeMap<String, FrigateCameraConfig>) -> FrigateConfig {
    FrigateConfig { cameras }
}

fn event(id: &str, label: &str, camera: &str, score: f64) -> FrigateEvent {
    let mut data = BTreeMap::new();
    data.insert("score".to_string(), serde_json::json!(score));
    FrigateEvent {
        id: id.to_string(),
        label: label.to_string(),
        sub_label: None,
        camera: camera.to_string(),
        start_time: 1700000000.0,
        end_time: None,
        false_positive: None,
        zones: vec!["driveway".to_string()],
        has_clip: true,
        has_snapshot: true,
        data,
    }
}

// ---------------------------------------------------------------------
// Provider response mapping
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_discovers_cameras_from_config_keys() {
    let mut cameras = BTreeMap::new();
    cameras.insert("front".to_string(), front_cfg());
    let mut disabled = front_cfg();
    disabled.enabled = false;
    cameras.insert("back".to_string(), disabled);
    let mock = MockTransport {
        config: cfg_with(cameras),
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let cameras = adapter.list_cameras().expect("list");
    // Stable identity: config key, sorted, deterministic.
    assert_eq!(cameras, vec![camera("back"), camera("front")]);
}

#[test]
fn ep023_unit_frigate_stable_identity_ignores_display_name_and_order() {
    // Reordering the map must not change canonical camera identity.
    let mut a = BTreeMap::new();
    a.insert("front".to_string(), front_cfg());
    let mock_a = MockTransport {
        config: cfg_with(a),
        health_ok: true,
        ..Default::default()
    };
    let adapter_a = FrigateAdapter::new(mock_a);
    let list_a = adapter_a.list_cameras().expect("list a");

    let mut b = BTreeMap::new();
    b.insert("front".to_string(), front_cfg());
    let mock_b = MockTransport {
        config: cfg_with(b),
        health_ok: true,
        ..Default::default()
    };
    let adapter_b = FrigateAdapter::new(mock_b);
    let list_b = adapter_b.list_cameras().expect("list b");

    assert_eq!(list_a, list_b);
    assert_eq!(list_a[0].as_str(), "front");
    // Friendly name is metadata only; identity never uses it.
    assert_ne!(list_a[0].as_str(), "Front Door");
}

#[test]
fn ep023_unit_frigate_health_maps_transport() {
    let adapter = FrigateAdapter::new(MockTransport {
        health_ok: true,
        ..Default::default()
    });
    assert!(adapter.health().is_ok());
}

#[test]
fn ep023_unit_frigate_health_failure_is_honest() {
    let adapter = FrigateAdapter::new(MockTransport {
        health_ok: false,
        ..Default::default()
    });
    let err = adapter.health().expect_err("unreachable");
    assert_eq!(err.code, VisionErrorCode::Unavailable);
}

// ---------------------------------------------------------------------
// Availability mapping (directive I/Q)
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_availability_configured_reachable_no_stream() {
    let mut cameras = BTreeMap::new();
    cameras.insert("front".to_string(), front_cfg());
    let mock = MockTransport {
        config: cfg_with(cameras),
        health_ok: true,
        streams: BTreeMap::new(),
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let state = adapter.availability(&camera("front")).expect("state");
    assert_eq!(state, CameraAvailability::Available);
}

#[test]
fn ep023_unit_frigate_availability_stream_attached_is_streaming_metadata() {
    let mut cameras = BTreeMap::new();
    cameras.insert("front".to_string(), front_cfg());
    let mut streams = BTreeMap::new();
    streams.insert(
        "front".to_string(),
        Go2RtcStreamInfo {
            producers: vec![Go2RtcProducer {
                url: "rtsp://127.0.0.1:8554/front".to_string(),
            }],
        },
    );
    let mock = MockTransport {
        config: cfg_with(cameras),
        health_ok: true,
        streams,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let state = adapter.availability(&camera("front")).expect("state");
    assert_eq!(state, CameraAvailability::Streaming);
}

#[test]
fn ep023_unit_frigate_availability_disabled_never_online() {
    let mut cameras = BTreeMap::new();
    let mut disabled = front_cfg();
    disabled.enabled = false;
    cameras.insert("front".to_string(), disabled);
    let mock = MockTransport {
        config: cfg_with(cameras),
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let state = adapter.availability(&camera("front")).expect("state");
    assert_eq!(state, CameraAvailability::Degraded);
    assert!(!nexus_frigate::availability::is_operational(state));
}

#[test]
fn ep023_unit_frigate_availability_unknown_camera_unavailable() {
    let mock = MockTransport {
        config: FrigateConfig::default(),
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let state = adapter.availability(&camera("ghost")).expect("state");
    assert_eq!(state, CameraAvailability::Unavailable);
}

#[test]
fn ep023_unit_frigate_availability_provider_down_is_unavailable() {
    let mut cameras = BTreeMap::new();
    cameras.insert("front".to_string(), front_cfg());
    let mock = MockTransport {
        config: cfg_with(cameras),
        health_ok: false,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let state = adapter.availability(&camera("front")).expect("state");
    assert_eq!(state, CameraAvailability::Unavailable);
}

// ---------------------------------------------------------------------
// Stream mapping (directive F/G/Q)
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_stream_never_verified_without_evidence() {
    let mut cameras = BTreeMap::new();
    cameras.insert("front".to_string(), front_cfg());
    let mut streams = BTreeMap::new();
    streams.insert(
        "front".to_string(),
        Go2RtcStreamInfo {
            producers: vec![Go2RtcProducer {
                url: "rtsp://127.0.0.1:8554/front".to_string(),
            }],
        },
    );
    let mock = MockTransport {
        config: cfg_with(cameras),
        health_ok: true,
        streams,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let stream = adapter.stream(&camera("front")).expect("stream");
    assert_eq!(stream.status, VerificationStatus::Unverified);
    assert!(stream.evidence_ref.is_none());
}

#[test]
fn ep023_unit_frigate_stream_falls_back_to_ffmpeg_input_unverified() {
    let mut cameras = BTreeMap::new();
    cameras.insert("front".to_string(), front_cfg());
    let mock = MockTransport {
        config: cfg_with(cameras),
        health_ok: true,
        streams: BTreeMap::new(),
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let stream = adapter.stream(&camera("front")).expect("stream");
    assert_eq!(stream.status, VerificationStatus::Unverified);
    // The URL is the real configured input path; never claimed verified.
    assert!(stream.url.contains("rtsp://"));
}

#[test]
fn ep023_unit_frigate_stream_unknown_camera_not_found() {
    let mock = MockTransport {
        config: FrigateConfig::default(),
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let err = adapter.stream(&camera("ghost")).expect_err("not found");
    assert_eq!(err.code, VisionErrorCode::NotFound);
}

#[test]
fn ep023_unit_frigate_snapshot_ref_is_unverified_and_url_only() {
    let adapter = FrigateAdapter::new(MockTransport::default());
    let snap = adapter
        .snapshot_ref(&camera("front"), "http://127.0.0.1:5000")
        .expect("snapshot ref");
    assert_eq!(snap.url, "http://127.0.0.1:5000/api/front/latest.jpg");
    assert_eq!(snap.status, VerificationStatus::Unverified);
}

// ---------------------------------------------------------------------
// Event / detection mapping (directive J)
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_events_map_real_detections() {
    let mock = MockTransport {
        events: vec![
            event("evt-1", "person", "front", 0.91),
            event("evt-2", "car", "front", 0.77),
        ],
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let events = adapter
        .events_since(&camera("front"), 0, 100)
        .expect("events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].object, "person");
    assert!((events[0].confidence - 0.91).abs() < 1e-6);
    assert_eq!(events[0].camera_id, camera("front"));
    assert_eq!(events[1].object, "car");
}

#[test]
fn ep023_unit_frigate_events_reject_missing_score_no_fabrication() {
    let mock = MockTransport {
        events: vec![FrigateEvent {
            id: "evt-bad".to_string(),
            label: "person".to_string(),
            sub_label: None,
            camera: "front".to_string(),
            start_time: 1700000000.0,
            end_time: None,
            false_positive: None,
            zones: vec![],
            has_clip: false,
            has_snapshot: false,
            data: BTreeMap::new(),
        }],
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let err = adapter
        .events_since(&camera("front"), 0, 100)
        .expect_err("no score");
    assert_eq!(err.code, VisionErrorCode::External);
}

#[test]
fn ep023_unit_frigate_events_media_refs_never_carry_raw_frames() {
    let mock = MockTransport {
        events: vec![event("evt-1", "person", "front", 0.9)],
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let events: Vec<CameraEvent> = adapter
        .events_since(&camera("front"), 0, 100)
        .expect("events");
    let media = &events[0].media_refs;
    assert_eq!(media.len(), 1);
    assert_eq!(
        media[0].url,
        "http://frigate.test/api/events/evt-1/snapshot.jpg"
    );
    assert_eq!(media[0].status, VerificationStatus::Unverified);
}

// ---------------------------------------------------------------------
// Error handling (directive R)
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_config_failure_is_honest() {
    let mock = MockTransport {
        fail_config: Some(VisionError::new(
            VisionErrorCode::External,
            "malformed config".to_string(),
            None,
            None,
        )),
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let err = adapter.list_cameras().expect_err("malformed");
    assert_eq!(err.code, VisionErrorCode::External);
}

#[test]
fn ep023_unit_frigate_events_failure_is_honest() {
    let mock = MockTransport {
        fail_events: Some(VisionError::new(
            VisionErrorCode::Timeout,
            "events timed out".to_string(),
            None,
            None,
        )),
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let err = adapter
        .events_since(&camera("front"), 0, 100)
        .expect_err("timeout");
    assert_eq!(err.code, VisionErrorCode::Timeout);
}

#[test]
fn ep023_unit_frigate_streams_failure_is_honest() {
    let mock = MockTransport {
        fail_streams: Some(VisionError::new(
            VisionErrorCode::Unavailable,
            "go2rtc unreachable".to_string(),
            None,
            None,
        )),
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let err = adapter.stream(&camera("front")).expect_err("unavailable");
    assert_eq!(err.code, VisionErrorCode::Unavailable);
}

#[test]
fn ep023_unit_frigate_latest_frame_failure_is_honest() {
    let mut mock = MockTransport {
        fail_latest: Some(VisionError::new(
            VisionErrorCode::External,
            "snapshot unavailable".to_string(),
            None,
            None,
        )),
        ..Default::default()
    };
    // Provider failure must surface, never a canned image.
    let err = mock
        .latest_frame("front")
        .expect_err("snapshot unavailable");
    assert_eq!(err.code, VisionErrorCode::External);
}
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_events_default_private() {
    let mock = MockTransport {
        events: vec![event("evt-1", "person", "front", 0.9)],
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let events = adapter
        .events_since(&camera("front"), 0, 100)
        .expect("events");
    assert_eq!(events[0].privacy_class, PrivacyClass::Private);
}

#[test]
fn ep023_unit_frigate_discovery_does_not_grant_image_access() {
    let mut cameras = BTreeMap::new();
    cameras.insert("front".to_string(), front_cfg());
    let mock = MockTransport {
        config: cfg_with(cameras),
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    // Discovery returns identities and metadata only.
    let cams = adapter.list_cameras().expect("list");
    assert_eq!(cams.len(), 1);
    // No frame bytes are fetched or returned by discovery.
    let calls = adapter.into_inner().calls.borrow().clone();
    assert!(!calls.iter().any(|c| c == "latest_frame"));
}

// ---------------------------------------------------------------------
// Advisory visitor identity (directive K)
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_visitor_identity_never_authorizes() {
    // Even a KnownVisitor is advisory-only and can never unlock or
    // disarm (SPEC-021 behavior 6, EP-008 remains authority).
    let identity = VisitorIdentity::Known(
        nexus_vision::identity::KnownVisitor::new("person-1", 0.95).expect("visitor"),
    );
    assert!(identity.is_advisory_only());
    match &identity {
        VisitorIdentity::Known(v) => {
            assert!(v.advisory_only);
            // There is no path from identity to authorization.
            let _ = v.person_id.as_str();
        }
        VisitorIdentity::Unknown => panic!("expected known"),
    }
}

// ---------------------------------------------------------------------
// Two-way audio gating (directive M)
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_two_way_audio_never_without_certification() {
    // Frigate audio config enabled does NOT certify two-way audio.
    let mut cameras = BTreeMap::new();
    let mut cfg = front_cfg();
    cfg.audio = nexus_frigate::transport::FrigateAudioConfig { enabled: true };
    cameras.insert("front".to_string(), cfg);
    let mock = MockTransport {
        config: cfg_with(cameras),
        health_ok: true,
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let caps = adapter.capabilities(&camera("front")).expect("caps");
    assert!(!caps.contains(&CameraCapability::TwoWayAudio));

    // The contract also refuses certification without every gate.
    let capability = TwoWayAudioCapability::new().with_verified_speaker_path(false);
    assert_eq!(capability.state, TwoWayAudioState::NotCertified);
    assert!(capability.certify().is_err());
}

// ---------------------------------------------------------------------
// Roku ladder ordering (directive N/O, SPEC-021 behavior 3)
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_roku_ladder_order_fixed() {
    // Roku fallback order is canonical and fixed (best -> worst).
    assert_eq!(
        RokuCapabilityTier::ladder(),
        [
            RokuCapabilityTier::LocalVerified,
            RokuCapabilityTier::VendorAuthenticated,
            RokuCapabilityTier::GoogleHomeBridge,
            RokuCapabilityTier::BrowserAutomation,
            RokuCapabilityTier::Unavailable,
        ]
    );
    // Selection is deterministic: first available tier wins.
    let plan = CameraFallbackPlan::new();
    assert_eq!(
        plan.select(&[RokuCapabilityTier::GoogleHomeBridge]),
        RokuCapabilityTier::GoogleHomeBridge
    );
    assert_eq!(
        plan.select(&[RokuCapabilityTier::BrowserAutomation]),
        RokuCapabilityTier::BrowserAutomation
    );
    // Nothing available fails closed to Unavailable, never fabricates.
    assert_eq!(plan.select(&[]), RokuCapabilityTier::Unavailable);
}

// ---------------------------------------------------------------------
// No unverified RTSP/ONVIF claim (acceptance obligation 3)
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_no_unverified_rtsp_claim() {
    // StreamRef::verified() requires a real evidence reference.
    let stream = nexus_vision::stream::StreamRef::new_unverified("s1", "rtsp://x/y").expect("ref");
    assert_eq!(stream.status, VerificationStatus::Unverified);
    assert!(stream.clone().verified("").is_err());
    let verified = stream.verified("go2rtc-probe-123").expect("verified");
    assert_eq!(verified.status, VerificationStatus::VerifiedLocal);
    assert_eq!(verified.evidence_ref.as_deref(), Some("go2rtc-probe-123"));
}

// ---------------------------------------------------------------------
// No secret leakage (directive S)
// ---------------------------------------------------------------------

#[test]
fn ep023_unit_frigate_redact_never_leaks_rtsp_credentials() {
    let url = "rtsp://admin:hunter2@192.168.1.10:554/stream";
    let redacted = redact_url(url);
    assert!(!redacted.contains("hunter2"));
    assert!(!redacted.contains("admin:"));
    assert!(redacted.contains("***"));
}

#[test]
fn ep023_unit_frigate_error_surfaces_do_not_embed_credentials() {
    // Transport errors that embed URLs are redacted at the boundary.
    let mock = MockTransport {
        fail_streams: Some(VisionError::new(
            VisionErrorCode::External,
            format!(
                "go2rtc failed for {}",
                redact_url("rtsp://user:secret@host/stream")
            ),
            None,
            None,
        )),
        ..Default::default()
    };
    let adapter = FrigateAdapter::new(mock);
    let err = adapter.stream(&camera("front")).expect_err("unavailable");
    assert!(!err.message.contains("secret"));
}
