//! EP-023 M1 unit suite for the nexus-vision contract crate
//! (construction, validation, serialization, vocabulary rejection,
//! dependency-direction invariants).

use nexus_vision::event::{CameraEvent, ReviewItem, VisitorEvent};
use nexus_vision::fallback::{BrowserAutomationPolicy, CameraFallbackPlan};
use nexus_vision::identity::{KnownVisitor, VisitorIdentity};
use nexus_vision::provider::{CameraProvider, FrigateProvider, RokuHomeProvider};
use nexus_vision::stream::StreamRef;
use nexus_vision::two_way::{TwoWayAudioCapability, TwoWayAudioState};
use nexus_vision::vocabulary::{CameraCapability, CameraId, PrivacyClass, RokuCapabilityTier};
use nexus_vision::{VisionError, VisionErrorCode};

fn camera(id: &str) -> CameraId {
    CameraId::new(id).expect("camera id")
}

#[test]
fn ep023_unit_camera_id_validation() {
    let error = CameraId::new("").expect_err("empty rejected");
    assert_eq!(error.code, VisionErrorCode::Validation);
    let error = CameraId::new("x".repeat(129)).expect_err("oversized rejected");
    assert_eq!(error.code, VisionErrorCode::Validation);
    let parsed = CameraId::new("front-door-1").expect("valid accepted");
    assert_eq!(parsed.as_str(), "front-door-1");
}

#[test]
fn ep023_unit_capability_vocabulary_lock() {
    for (text, expected) in [
        ("OBJECT_DETECTION", CameraCapability::ObjectDetection),
        ("RECORDING", CameraCapability::Recording),
        ("LIVE_STREAM", CameraCapability::LiveStream),
        ("TWO_WAY_AUDIO", CameraCapability::TwoWayAudio),
        ("VISITOR_EVENTS", CameraCapability::VisitorEvents),
        ("ROKU_CONTROL", CameraCapability::RokuControl),
    ] {
        assert_eq!(CameraCapability::parse(text).expect("canonical"), expected);
        assert_eq!(expected.as_str(), text);
    }
    let error = CameraCapability::parse("FACE_MATCH").expect_err("unknown rejected");
    assert_eq!(error.code, VisionErrorCode::Vocabulary);
    // Serde roundtrip preserves canonical forms.
    let json = serde_json::to_string(&CameraCapability::ObjectDetection).expect("json");
    assert_eq!(json, "\"OBJECT_DETECTION\"");
    let back: CameraCapability = serde_json::from_str(&json).expect("roundtrip");
    assert_eq!(back, CameraCapability::ObjectDetection);
}

#[test]
fn ep023_unit_rokutier_ladder_order_fixed() {
    // SPEC-021 behavior 3: verified local, authenticated vendor,
    // Google Home bridge, browser automation, then unavailable.
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
    // Ordinal order enforces the ladder (better tier < worse tier).
    assert!(RokuCapabilityTier::LocalVerified < RokuCapabilityTier::VendorAuthenticated);
    assert!(RokuCapabilityTier::VendorAuthenticated < RokuCapabilityTier::GoogleHomeBridge);
    assert!(RokuCapabilityTier::GoogleHomeBridge < RokuCapabilityTier::BrowserAutomation);
    assert!(RokuCapabilityTier::BrowserAutomation < RokuCapabilityTier::Unavailable);
}

#[test]
fn ep023_unit_stream_ref_unverified_no_claim() {
    // Unverified streams make no operational claim (acceptance
    // obligation 3).
    let stream = StreamRef::new_unverified("cam-1", "rtsp://192.0.2.10/stream")
        .expect("unverified accepted");
    assert_eq!(stream.status.as_str(), "UNVERIFIED");
    assert!(stream.evidence_ref.is_none());
    // Bad schemes are rejected.
    let error = StreamRef::new_unverified("cam-1", "file:///etc/passwd").expect_err("scheme");
    assert_eq!(error.code, VisionErrorCode::Validation);
    // Verification cannot be fabricated: empty evidence refused.
    let error = stream
        .clone()
        .verified("")
        .expect_err("empty evidence refused");
    assert_eq!(error.code, VisionErrorCode::Verification);
    // Real evidence marks verified (go2rtc normalization proof).
    let verified = stream
        .verified("go2rtc-probe-cam-1-20260817")
        .expect("verified with evidence");
    assert_eq!(verified.status.as_str(), "VERIFIED_LOCAL");
    assert_eq!(
        verified.evidence_ref.as_deref(),
        Some("go2rtc-probe-cam-1-20260817")
    );
}

#[test]
fn ep023_unit_camera_event_fields() {
    let mut event = CameraEvent::new(
        camera("front-door-1"),
        1_700_000_000_000,
        "person",
        0.98,
        7,
        PrivacyClass::Private,
    )
    .expect("valid event")
    .with_zone("porch")
    .with_media(
        StreamRef::new_unverified("clip-1", "https://nvr.local/clips/1.mp4").expect("media"),
    );
    event.zones.push("steps".to_string());
    assert_eq!(event.camera_id.as_str(), "front-door-1");
    assert_eq!(event.object, "person");
    assert_eq!(event.zones, vec!["porch", "steps"]);
    assert_eq!(event.retention_days, 7);
    assert_eq!(event.privacy_class, PrivacyClass::Private);
    assert_eq!(event.media_refs.len(), 1);
    // Serialization roundtrip preserves the canonical event shape.
    let json = serde_json::to_string(&event).expect("json");
    let back: CameraEvent = serde_json::from_str(&json).expect("roundtrip");
    assert_eq!(back, event);
    // Empty object and out-of-range confidence rejected.
    let error = CameraEvent::new(camera("c"), 0, "", 0.5, 1, PrivacyClass::Private)
        .expect_err("empty object");
    assert_eq!(error.code, VisionErrorCode::Validation);
    let error = CameraEvent::new(camera("c"), 0, "person", 1.5, 1, PrivacyClass::Private)
        .expect_err("confidence");
    assert_eq!(error.code, VisionErrorCode::Validation);
}

#[test]
fn ep023_unit_review_item_validation() {
    let item = ReviewItem::new("rev-1", camera("c"), 100, 30, PrivacyClass::Shared)
        .expect("valid")
        .with_media(StreamRef::new_unverified("s", "https://nvr.local/s.mp4").expect("media"));
    assert_eq!(item.review_id, "rev-1");
    assert_eq!(item.media_refs.len(), 1);
    let error = ReviewItem::new("", camera("c"), 0, 1, PrivacyClass::Shared).expect_err("empty id");
    assert_eq!(error.code, VisionErrorCode::Validation);
}

#[test]
fn ep023_unit_visitor_event_validation() {
    let event = VisitorEvent::new("v-1", camera("c"), 100, true, 0.9, PrivacyClass::Private)
        .expect("valid");
    assert!(event.known_person);
    let json = serde_json::to_string(&event).expect("json");
    let back: VisitorEvent = serde_json::from_str(&json).expect("roundtrip");
    assert_eq!(back, event);
    let error = VisitorEvent::new("v-2", camera("c"), 0, false, -0.1, PrivacyClass::Private)
        .expect_err("confidence");
    assert_eq!(error.code, VisionErrorCode::Validation);
}

#[test]
fn ep023_unit_known_visitor_advisory_only() {
    // SPEC-021 behavior 6: known-person matching is advisory and can
    // never unlock or disarm by itself.
    let known = KnownVisitor::new("person-7", 0.85).expect("valid");
    assert!(known.advisory_only, "advisory_only must be enforced");
    assert_eq!(known.person_id, "person-7");
    let error = KnownVisitor::new("", 0.5).expect_err("empty person id");
    assert_eq!(error.code, VisionErrorCode::Validation);
    let error = KnownVisitor::new("person-7", 2.0).expect_err("confidence");
    assert_eq!(error.code, VisionErrorCode::Validation);
}

#[test]
fn ep023_unit_visitor_identity_never_authorizes() {
    // Both classifications are advisory: identity evidence never
    // authorizes unlock or disarm (SPEC-021 behavior 6).
    let known = VisitorIdentity::Known(KnownVisitor::new("person-7", 0.9).expect("known"));
    let unknown = VisitorIdentity::Unknown;
    assert!(known.is_advisory_only());
    assert!(unknown.is_advisory_only());
    // Serde tagged roundtrip.
    let json = serde_json::to_string(&known).expect("json");
    let back: VisitorIdentity = serde_json::from_str(&json).expect("roundtrip");
    assert_eq!(back, known);
}

#[test]
fn ep023_unit_two_way_audio_never_without_certification() {
    // Acceptance obligation 4: two-way audio is enabled only after
    // live certification (SPEC-021 behavior 7).
    let capability = TwoWayAudioCapability::new();
    assert_eq!(capability.state, TwoWayAudioState::NotCertified);
    // Every gate is mandatory.
    let error = capability
        .clone()
        .certify()
        .expect_err("no verified speaker path");
    assert_eq!(error.code, VisionErrorCode::Verification);
    // Every remaining gate is individually mandatory.
    let mut gated = TwoWayAudioCapability::new().with_verified_speaker_path(true);
    gated.approval_required = false;
    let error = gated.certify().expect_err("approval cannot be waived");
    assert_eq!(error.code, VisionErrorCode::Policy);
    let mut gated = TwoWayAudioCapability::new().with_verified_speaker_path(true);
    gated.disclosure_required = false;
    let error = gated.certify().expect_err("disclosure cannot be waived");
    assert_eq!(error.code, VisionErrorCode::Policy);
    let mut gated = TwoWayAudioCapability::new().with_verified_speaker_path(true);
    gated.echo_handling_required = false;
    let error = gated.certify().expect_err("echo handling cannot be waived");
    assert_eq!(error.code, VisionErrorCode::Policy);
    // All gates met: certified.
    let certified = TwoWayAudioCapability::new()
        .with_verified_speaker_path(true)
        .certify()
        .expect("certified");
    assert_eq!(certified.state, TwoWayAudioState::Certified);
}

#[test]
fn ep023_unit_fallback_ladder_selection() {
    // SPEC-021 behavior 3: first available ladder tier wins; nothing
    // available fails closed to Unavailable.
    let plan = CameraFallbackPlan::new();
    assert_eq!(plan.ladder, RokuCapabilityTier::ladder());
    assert_eq!(
        plan.select(&[
            RokuCapabilityTier::VendorAuthenticated,
            RokuCapabilityTier::GoogleHomeBridge,
        ]),
        RokuCapabilityTier::VendorAuthenticated
    );
    assert_eq!(
        plan.select(&[RokuCapabilityTier::GoogleHomeBridge]),
        RokuCapabilityTier::GoogleHomeBridge
    );
    assert_eq!(
        plan.select(&[RokuCapabilityTier::BrowserAutomation]),
        RokuCapabilityTier::BrowserAutomation
    );
    assert_eq!(plan.select(&[]), RokuCapabilityTier::Unavailable);
    assert_eq!(
        plan.select(&[RokuCapabilityTier::Unavailable]),
        RokuCapabilityTier::Unavailable
    );
    // LocalVerified wins over everything when available.
    assert_eq!(
        plan.select(&[
            RokuCapabilityTier::BrowserAutomation,
            RokuCapabilityTier::LocalVerified,
        ]),
        RokuCapabilityTier::LocalVerified
    );
    // Browser automation policy is isolated/monitored/rate-limited and
    // never a stable API (behavior 4).
    let policy = BrowserAutomationPolicy::default();
    assert!(policy.isolated && policy.monitored && policy.rate_limited && policy.never_stable_api);
}

#[test]
fn ep023_unit_provider_ports_fail_closed() {
    // Unbound providers never fabricate cameras, events, or streams.
    struct Unbound;
    impl CameraProvider for Unbound {}
    impl FrigateProvider for Unbound {}
    impl RokuHomeProvider for Unbound {}
    let unbound = Unbound;
    let error = unbound.list_cameras().expect_err("camera provider unbound");
    assert_eq!(error.code, VisionErrorCode::Unavailable);
    let error = unbound.stream(&camera("c")).expect_err("stream unbound");
    assert_eq!(error.code, VisionErrorCode::Unavailable);
    let error = unbound
        .events(&camera("c"), 0)
        .expect_err("frigate unbound");
    assert_eq!(error.code, VisionErrorCode::Unavailable);
    let error = unbound.health().expect_err("frigate health unbound");
    assert_eq!(error.code, VisionErrorCode::Unavailable);
    let error = unbound.inventory().expect_err("roku unbound");
    assert_eq!(error.code, VisionErrorCode::Unavailable);
    let error = unbound.tier(&camera("c")).expect_err("roku tier unbound");
    assert_eq!(error.code, VisionErrorCode::Unavailable);
}

#[test]
fn ep023_unit_error_redacted_surface() {
    // Errors never carry raw video or credentials.
    let error: VisionError = VisionError::new(
        VisionErrorCode::External,
        "frigate returned malformed payload",
        None,
        Some(Box::from("front-door-1")),
    );
    let dict = error.as_dict();
    assert_eq!(dict["code"], "EXTERNAL");
    let serialized = dict.to_string();
    assert!(!serialized.to_lowercase().contains("frame"));
    assert!(!serialized.to_lowercase().contains("api_key"));
    assert!(!serialized.to_lowercase().contains("password"));
}
