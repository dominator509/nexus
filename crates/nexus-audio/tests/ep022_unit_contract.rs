//! EP-022 M1 contract suite (SPEC-012).
//!
//! Non-vacuous `ep022_unit_*` tests proving vocabulary locking (top-ten
//! hardware matrix), endpoint construction/validation/serialization,
//! deterministic room/person/privacy/availability routing, conversation
//! transfer context preservation, AEC profile validation, and typed
//! error behavior. The M1 gate runs this suite through the real
//! `cargo test -p nexus-audio ep022_unit` machinery with a vacuity
//! guard.

use nexus_audio::{
    require_hardware_class, require_role, AecProfile, AudioEndpoint, AudioEndpointId, AudioError,
    AudioErrorCode, AudioRoomId, BluetoothDeviceRef, ConversationContext, ConversationTransfer,
    DeterministicRouter, DeterministicTransfer, EchoCancellationProfile, EndpointAvailability,
    EndpointRole, EndpointRouter, HardwareClass, RouterPolicy, RoutingInput, RoutingOutput,
    VoiceSatellite, VoiceSatelliteId, HARDWARE_CLASSES,
};
use nexus_domain::PersonId;

fn person(n: u8) -> PersonId {
    PersonId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f70{n:02}")).expect("valid UUIDv7")
}

fn endpoint_id(n: u8) -> AudioEndpointId {
    AudioEndpointId::new(format!("endpoint-{n:02}")).expect("valid id")
}

fn room_id(name: &str) -> AudioRoomId {
    AudioRoomId::new(name).expect("valid room")
}

fn available_endpoint(n: u8, role: EndpointRole, name: &str) -> AudioEndpoint {
    AudioEndpoint::new(endpoint_id(n), HardwareClass::Pi5, role, name)
}

#[test]
fn ep022_unit_vocabulary_accepts_all_hardware_classes() {
    assert_eq!(HARDWARE_CLASSES.len(), 12);
    for class in HARDWARE_CLASSES {
        let parsed = require_hardware_class(class.as_str()).expect("canonical class parses");
        assert_eq!(parsed, *class);
    }
    assert_eq!(HardwareClass::Esp32S3Box3.as_str(), "ESP32_S3_BOX_3");
    assert_eq!(HardwareClass::AssistSatellite.as_str(), "ASSIST_SATELLITE");
}

#[test]
fn ep022_unit_vocabulary_rejects_unknown_hardware_class() {
    let err = require_hardware_class("NOT_A_CLASS").expect_err("unknown class rejected");
    assert!(err.0.contains("unknown hardware class"));
    let audio: AudioError = err.into();
    assert_eq!(audio.code, AudioErrorCode::Vocabulary);
}

#[test]
fn ep022_unit_vocabulary_rejects_unknown_role() {
    let err = require_role("SIDEWAYS").expect_err("unknown role rejected");
    assert!(err.0.contains("unknown endpoint role"));
}

#[test]
fn ep022_unit_endpoint_construction_and_validation() {
    let endpoint = available_endpoint(1, EndpointRole::Bidirectional, "kitchen pi");
    assert_eq!(endpoint.role, EndpointRole::Bidirectional);
    assert_eq!(endpoint.availability, EndpointAvailability::Online);
    assert!(AudioEndpointId::new("").is_err());
    assert!(AudioEndpointId::new("x".repeat(129)).is_err());
    assert!(AudioRoomId::new("").is_err());
}

#[test]
fn ep022_unit_endpoint_serialization_roundtrip() {
    let endpoint = available_endpoint(2, EndpointRole::Input, "bedroom box")
        .with_room(room_id("bedroom"))
        .with_person(person(1));
    let wire = endpoint.to_wire();
    assert_eq!(wire["schema"], "nexus.audio.endpoint.v1");
    assert_eq!(wire["hardware_class"], "PI_5");
    assert_eq!(wire["room"], "bedroom");
    // The wire payload must never carry raw audio.
    assert!(wire.get("data").is_none());
    assert!(wire.get("audio").is_none());
}

#[test]
fn ep022_unit_router_selects_available_person_endpoint() {
    let p = person(1);
    let candidates = vec![
        available_endpoint(1, EndpointRole::Input, "a")
            .with_availability(EndpointAvailability::Offline),
        available_endpoint(2, EndpointRole::Input, "b").with_person(p.clone()),
        available_endpoint(3, EndpointRole::Input, "c").with_room(room_id("kitchen")),
    ];
    let router = DeterministicRouter;
    let out = router
        .select(
            RoutingInput {
                candidates: &candidates,
                room: Some(&room_id("kitchen")),
                person: Some(&p),
                role: EndpointRole::Input,
            },
            RouterPolicy::default(),
        )
        .expect("routed");
    assert_eq!(out.endpoint_id, endpoint_id(2));
    assert_eq!(out.hardware_class, HardwareClass::Pi5);
}

#[test]
fn ep022_unit_router_prefers_room_when_no_person_binding() {
    let candidates = vec![
        available_endpoint(1, EndpointRole::Output, "a"),
        available_endpoint(2, EndpointRole::Output, "b").with_room(room_id("kitchen")),
    ];
    let router = DeterministicRouter;
    let out = router
        .select(
            RoutingInput {
                candidates: &candidates,
                room: Some(&room_id("kitchen")),
                person: None,
                role: EndpointRole::Output,
            },
            RouterPolicy::default(),
        )
        .expect("routed");
    assert_eq!(out.endpoint_id, endpoint_id(2));
}

#[test]
fn ep022_unit_endpoint_identity_is_canonical_ref_not_display_name() {
    // Two endpoints may share a display name; identity is the canonical
    // endpoint reference (endpoint_id), never the mutable friendly name.
    let a = available_endpoint(1, EndpointRole::Input, "kitchen pi");
    let b = available_endpoint(2, EndpointRole::Input, "kitchen pi");
    assert_ne!(a.endpoint_id, b.endpoint_id);
    assert_eq!(a.name, b.name);
    let router = DeterministicRouter;
    let out = router
        .select(
            RoutingInput {
                candidates: &[a.clone(), b.clone()],
                room: None,
                person: None,
                role: EndpointRole::Input,
            },
            RouterPolicy::default(),
        )
        .expect("routed deterministically");
    // Deterministic tie-break on stable endpoint id, not name.
    assert_eq!(out.endpoint_id, endpoint_id(1));
}

#[test]
fn ep022_unit_transfer_never_implicitly_upgrades_privacy() {
    // Transfer preserves the canonical privacy class exactly; it never
    // upgrades or downgrades privacy without a router decision.
    let p = person(4);
    let context =
        ConversationContext::new("sess-priv", p, "objective", "policy-private").expect("ctx");
    let transfer = DeterministicTransfer;
    let moved = transfer.transfer(&context, &endpoint_id(7)).expect("moved");
    assert_eq!(moved.privacy_policy_id, "policy-private");
    assert_eq!(moved.session_id, "sess-priv");
}

#[test]
fn ep022_unit_router_sensitive_never_shared_room_output() {
    let p = person(1);
    // The only available outputs are room-bound (shared room); with
    // sensitive content the router must pick a person-bound output or
    // fail closed rather than selecting the shared-room speaker.
    let candidates = vec![
        available_endpoint(1, EndpointRole::Output, "shared speaker").with_room(room_id("living")),
        available_endpoint(2, EndpointRole::Output, "personal earbuds").with_person(p.clone()),
    ];
    let router = DeterministicRouter;
    let out = router
        .select(
            RoutingInput {
                candidates: &candidates,
                room: Some(&room_id("living")),
                person: Some(&p),
                role: EndpointRole::Output,
            },
            RouterPolicy {
                prefer_person: true,
                sensitive: true,
            },
        )
        .expect("private output selected");
    assert_eq!(out.endpoint_id, endpoint_id(2));

    // With no private-capable output, the router fails closed.
    let only_shared = vec![
        available_endpoint(1, EndpointRole::Output, "shared speaker").with_room(room_id("living")),
    ];
    let err = router
        .select(
            RoutingInput {
                candidates: &only_shared,
                room: Some(&room_id("living")),
                person: Some(&p),
                role: EndpointRole::Output,
            },
            RouterPolicy {
                prefer_person: true,
                sensitive: true,
            },
        )
        .expect_err("sensitive shared-room output refused");
    assert_eq!(err.code, AudioErrorCode::NotFound);
}

#[test]
fn ep022_unit_router_no_available_candidate_not_found() {
    let candidates = vec![available_endpoint(1, EndpointRole::Input, "a")
        .with_availability(EndpointAvailability::Offline)];
    let router = DeterministicRouter;
    let err = router
        .select(
            RoutingInput {
                candidates: &candidates,
                room: None,
                person: None,
                role: EndpointRole::Input,
            },
            RouterPolicy::default(),
        )
        .expect_err("no candidate");
    assert_eq!(err.code, AudioErrorCode::NotFound);
}

#[test]
fn ep022_unit_transfer_preserves_conversation_context() {
    let p = person(2);
    let mut context = ConversationContext::new(
        "session-1",
        p.clone(),
        "turn on the lights",
        "policy-private",
    )
    .expect("context")
    .with_room(room_id("kitchen"))
    .with_correlation("corr-1".into());
    context.append_transcript("user", "turn on the lights");
    context.append_transcript("assist", "on it");

    let transfer = DeterministicTransfer;
    let target = endpoint_id(9);
    let moved = transfer
        .transfer(&context, &target)
        .expect("transfer preserves context");
    assert_eq!(moved.session_id, "session-1");
    assert_eq!(moved.principal, p);
    assert_eq!(moved.objective, "turn on the lights");
    assert_eq!(moved.privacy_policy_id, "policy-private");
    assert_eq!(moved.room, Some(room_id("kitchen")));
    assert_eq!(moved.transcript.len(), 2);
    assert_eq!(moved.correlation_id.as_deref(), Some("corr-1"));
}

#[test]
fn ep022_unit_transfer_rejects_empty_context_fields() {
    let p = person(3);
    assert!(ConversationContext::new("", p.clone(), "objective", "policy").is_err());
    assert!(ConversationContext::new("s", p.clone(), "", "policy").is_err());
    assert!(ConversationContext::new("s", p, "objective", "").is_err());
}

#[test]
fn ep022_unit_aec_profile_validation() {
    assert!(EchoCancellationProfile::new(AecProfile::Full, 0, true).is_ok());
    assert!(EchoCancellationProfile::new(AecProfile::EchoCancellation, 2, false).is_ok());
    let err =
        EchoCancellationProfile::new(AecProfile::None, 3, true).expect_err("aggressiveness bound");
    assert_eq!(err.code, AudioErrorCode::Validation);
    assert_eq!(AecProfile::parse("FULL").expect("parses"), AecProfile::Full);
    assert!(AecProfile::parse("MAX").is_err());
}

#[test]
fn ep022_unit_satellite_and_bluetooth_identity_validation() {
    assert!(VoiceSatelliteId::new("").is_err());
    assert!(BluetoothDeviceRef::new("").is_err());
    let satellite = VoiceSatellite::new(
        VoiceSatelliteId::new("sat-kitchen").expect("id"),
        HardwareClass::Esp32S3Box3,
        "kitchen box",
    );
    assert_eq!(satellite.hardware_class, HardwareClass::Esp32S3Box3);
}

#[test]
fn ep022_unit_ports_fail_closed_unavailable() {
    // Unbound ports fail closed with typed UNAVAILABLE; they never
    // fabricate a result.
    struct NoopRouter;
    impl EndpointRouter for NoopRouter {
        fn select(
            &self,
            _input: RoutingInput<'_>,
            _policy: RouterPolicy,
        ) -> Result<RoutingOutput, AudioError> {
            Err(AudioError::unavailable("no implementation bound"))
        }
    }
    let router = NoopRouter;
    let err = router
        .select(
            RoutingInput {
                candidates: &[],
                room: None,
                person: None,
                role: EndpointRole::Input,
            },
            RouterPolicy::default(),
        )
        .expect_err("fail closed");
    assert_eq!(err.code, AudioErrorCode::Unavailable);
    let surface = err.as_dict();
    assert_eq!(surface["code"], "UNAVAILABLE");
    assert!(surface.get("data").is_none());
    assert!(surface.get("audio").is_none());
}
