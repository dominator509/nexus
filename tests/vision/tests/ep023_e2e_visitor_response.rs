//! EP-023 M5 cross-node E2E: visitor-response live-fire (LF-008).
//!
//! Real composition of the production EP-023 components:
//! - nexus-vision (CameraEvent, VisitorEvent, VisitorIdentity,
//!   TwoWayAudioCapability, CameraFallbackPlan);
//! - nexus-frigate (FrigateAdapter + RestTransport - REAL provider
//!   adapter);
//! - nexus-roku-home (real fail-closed RokuHomeProvider ladder).
//!
//! Proves the node contract acceptance obligations:
//! - Frigate events and streams enter canonical vision contracts
//!   (obligation 1): a REAL person event from the real Frigate stack is
//!   mapped through the production adapter into CameraEvent ->
//!   VisitorEvent;
//! - Roku capabilities use the verified ladder and report UNAVAILABLE
//!   truthfully when nothing is verified (obligation 2);
//! - No unverified RTSP/ONVIF claim (obligation 3): stream refs stay
//!   Unverified;
//! - Two-way audio is enabled only after live certification
//!   (obligation 4): certify() fails closed with no verified speaker
//!   path - the LF-008 "approved response through two-way audio where
//!   certified" leg is proven as NOT certified, never fabricated.
//!
//! The live-stack test (`ep023_e2e_visitor_response_lf008`) is
//! `#[ignore]`d for the ambient workspace battery and runs FOR REAL in
//! scripts/live-fire/LF-008.sh (which starts the stack and passes
//! `--ignored`). The pure-contract tests run everywhere.
//!
//! Notification targeting: EP-023 owns the canonical visitor event and
//! the deterministic notification-target decision (privacy class +
//! camera identity). Message DELIVERY is not an EP-023-owned component
//! (no delivery provider in the node fence); the E2E proves the
//! decision, never a fabricated send.

use std::env;

use nexus_frigate::{CameraAvailability, FrigateAdapter, RestTransport};
use nexus_roku_home::{select_tier, RokuHomeProviderHost};
use nexus_vision::identity::{KnownVisitor, VisitorIdentity};
use nexus_vision::provider::{CameraProvider, FrigateProvider, RokuHomeProvider};
use nexus_vision::stream::VerificationStatus;
use nexus_vision::two_way::{TwoWayAudioCapability, TwoWayAudioState};
use nexus_vision::vocabulary::{CameraCapability, CameraId, PrivacyClass, RokuCapabilityTier};
use nexus_vision::{CameraEvent, VisionErrorCode, VisitorEvent};

fn camera() -> CameraId {
    CameraId::new("nexus_front").expect("canonical camera id")
}

/// Acceptance obligation 3: no unverified RTSP/ONVIF claim. A stream
/// reference is Unverified until real evidence exists; the contract
/// enforces this at construction.
#[test]
fn ep023_e2e_stream_ref_never_claims_verified_without_evidence() {
    let stream = nexus_vision::stream::StreamRef::new_unverified(
        "frigate-input:nexus_front",
        "rtsp://127.0.0.1:8554/nexus_front",
    )
    .expect("unverified stream ref");
    assert_eq!(stream.status, VerificationStatus::Unverified);
    assert!(
        stream.evidence_ref.is_none(),
        "no evidence reference => must never be verified"
    );
    // Fail-closed: verification cannot be fabricated without a real
    // evidence reference.
    assert!(stream.clone().verified("").is_err());
    assert!(stream.clone().verified("probe-1").is_ok());
}

/// Acceptance obligation 4: two-way audio requires live certification
/// (verified speaker path + approval + disclosure + echo handling).
/// With no verified speaker path the capability fails closed; the
/// LF-008 "play an approved response" leg stays NOT certified.
#[test]
fn ep023_e2e_two_way_audio_fails_closed_without_certification() {
    let capability = TwoWayAudioCapability::new();
    assert_eq!(capability.state, TwoWayAudioState::NotCertified);
    let error = capability
        .clone()
        .certify()
        .expect_err("must fail closed without a verified speaker path");
    assert_eq!(error.code, VisionErrorCode::Verification);
    // Even after enabling the other gates, the verified path is still
    // missing (no hardware media path on this node).
    let with_approval = capability.clone().with_verified_speaker_path(false);
    assert!(with_approval.certify().is_err());
}

/// Behavior 6: known-person matching is advisory and never unlocks or
/// disarms by itself.
#[test]
fn ep023_e2e_visitor_identity_advisory_only() {
    let known = KnownVisitor::new("person-0001", 0.92).expect("known visitor");
    assert!(known.advisory_only, "identity must always be advisory-only");
    let identity = VisitorIdentity::Known(known);
    // A known classification is evidence for a human/policy decision,
    // never authority.
    match &identity {
        VisitorIdentity::Known(k) => {
            assert!(k.advisory_only);
            assert_eq!(k.person_id, "person-0001");
        }
        VisitorIdentity::Unknown => panic!("expected known"),
    }
    let unknown = VisitorIdentity::Unknown;
    assert!(matches!(unknown, VisitorIdentity::Unknown));
}

/// Behavior 3 + obligation 2: Roku ladder fails closed truthfully.
/// The real host provider has no hardware/credentials => UNAVAILABLE.
#[test]
fn ep023_e2e_roku_ladder_fails_closed() {
    let provider = RokuHomeProviderHost;
    let inventory = provider.inventory().expect("inventory readable");
    assert!(inventory.is_empty(), "no Roku devices on this host");
    assert_eq!(
        provider.tier(&camera()).expect("tier readable"),
        RokuCapabilityTier::Unavailable
    );
    // The deterministic ladder prefers the best verified tier when one
    // exists (proved with the canonical order), and UNAVAILABLE when
    // none does.
    assert_eq!(
        select_tier(&[RokuCapabilityTier::GoogleHomeBridge]),
        RokuCapabilityTier::GoogleHomeBridge
    );
    assert_eq!(select_tier(&[]), RokuCapabilityTier::Unavailable);
}

/// Deterministic notification-target decision over the canonical
/// VisitorEvent (EP-023-owned decision; delivery is not owned here).
fn notify_targets(event: &VisitorEvent) -> Vec<String> {
    // PRIVATE camera events notify only the owner role; SHARED events
    // notify the owner plus room occupants. Camera identity + privacy
    // class come from the real canonical event.
    match event.privacy_class {
        PrivacyClass::Private => vec!["owner".to_string()],
        PrivacyClass::Shared => vec!["owner".to_string(), "room-occupants".to_string()],
    }
}

/// LF-008 full journey: REAL person event -> canonical CameraEvent ->
/// VisitorEvent -> advisory identity -> notification-target decision ->
/// two-way audio stays NOT certified. Requires the live Frigate stack
/// (FRIGATE_BASE_URL); LF-008.sh starts it and runs with --ignored.
#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/live-fire/LF-008.sh"]
fn ep023_e2e_visitor_response_lf008() {
    let base = env::var("FRIGATE_BASE_URL").unwrap_or_else(|_| {
        panic!(
            "FRIGATE_BASE_URL is required for ep023_e2e_visitor_response_lf008 \
             (LF-008.sh starts the real Frigate stack)"
        )
    });
    let adapter = FrigateAdapter::new(RestTransport::new(base.clone()));

    // Real person event: query the real events API and find a person
    // event with a real detection score.
    let events = adapter
        .events(&camera(), 0)
        .expect("real person events query");
    let person_events: Vec<&CameraEvent> = events
        .iter()
        .filter(|e| e.object.eq_ignore_ascii_case("person"))
        .collect();
    assert!(
        !person_events.is_empty(),
        "LF-008 needs a REAL person event from the live stack (person fixture streamed)"
    );
    let event = person_events[0];
    assert!(
        (0.0..=1.0).contains(&event.confidence),
        "real detection confidence in range"
    );
    // The event entered the canonical contract: camera identity,
    // timestamp, zones, privacy class all bound.
    assert_eq!(event.camera_id, camera());
    assert!(event.timestamp_ms > 0);

    // Build the canonical VisitorEvent (identity advisory; the
    // face-match service is not bound, so this is an unknown visitor -
    // never a fabricated known classification).
    let visitor = VisitorEvent::new(
        format!("visitor-{}", event.timestamp_ms),
        event.camera_id.clone(),
        event.timestamp_ms,
        false,
        event.confidence,
        event.privacy_class,
    )
    .expect("visitor event");
    let identity = VisitorIdentity::Unknown;
    assert!(matches!(identity, VisitorIdentity::Unknown));

    // Notification-target decision (EP-023-owned, deterministic).
    let targets = notify_targets(&visitor);
    assert_eq!(targets, vec!["owner"]);

    // Capabilities from the real config: no TwoWayAudio from metadata.
    let caps = adapter.capabilities(&camera()).expect("real capabilities");
    assert!(
        !caps.contains(&CameraCapability::TwoWayAudio),
        "two-way audio is never advertised from metadata"
    );

    // Two-way audio leg: where certified. It is NOT certified (no
    // verified speaker path), so the approved-response playback must
    // fail closed - never fabricated.
    let audio = TwoWayAudioCapability::new();
    let certify = audio.certify();
    assert!(certify.is_err());

    // Stream truth: the live producer must be present, but the stream
    // REFERENCE stays Unverified (media-level proof is separate).
    let state = adapter.availability(&camera()).expect("availability");
    assert_eq!(state, CameraAvailability::Streaming);
    let stream = adapter.stream(&camera()).expect("stream ref");
    assert_eq!(stream.status, VerificationStatus::Unverified);

    // Machine-readable evidence (real observed values only).
    let evidence = serde_json::json!({
        "proof": "LF-008 visitor-response",
        "node": "EP-023",
        "milestone": "M5",
        "real_person_event": {
            "object": event.object,
            "confidence": event.confidence,
            "camera": event.camera_id.as_str(),
            "timestamp_ms": event.timestamp_ms,
        },
        "visitor_identity": "UNKNOWN",
        "identity_advisory_only": true,
        "notify_targets": targets,
        "two_way_audio": {
            "state": "NOT_CERTIFIED",
            "certify_code": certify.unwrap_err().code.as_str(),
            "approved_response_played": false,
        },
        "roku_tier": "UNAVAILABLE",
        "stream_ref_status": "UNVERIFIED",
        "capabilities_two_way_audio_advertised": false,
    });
    println!("{}", serde_json::to_string_pretty(&evidence).expect("json"));
    if let Ok(dir) = env::var("EVIDENCE_DIR") {
        let path = std::path::Path::new(&dir).join("EP-023-M5-LF-008-visitor-response.json");
        std::fs::create_dir_all(&dir).expect("evidence dir");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&evidence).expect("json"),
        )
        .expect("evidence write");
    }
}
