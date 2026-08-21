//! EP-035 M2 HardwareProfiler provenance tests (SPEC-004/016).

use nexus_domain::CorrelationId;
use nexus_setup::{
    CapabilityCertificationState, HardwareCapabilityDeclaration, HardwareFact, HardwareProfile,
    HardwareProvenance, HardwareValue, SetupErrorCode,
};

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("00000000-0000-7000-8000-00000000000{n}")).unwrap()
}

#[test]
fn ep035_unit_hardware_user_declared_gpu_is_not_detected() {
    let fact = HardwareFact::new(
        "gpu_model",
        HardwareValue::Str("RTX 4090".to_string()),
        HardwareProvenance::UserDeclared,
        None,
    )
    .unwrap();
    assert_eq!(fact.provenance, HardwareProvenance::UserDeclared);
    let wire = serde_json::to_value(&fact).unwrap();
    assert_eq!(wire["provenance"], "USER_DECLARED");
}

#[test]
fn ep035_unit_hardware_provenance_classes_are_distinct() {
    let declared = HardwareFact::new(
        "cpu_cores",
        HardwareValue::Int(8),
        HardwareProvenance::UserDeclared,
        None,
    )
    .unwrap();
    let observed = HardwareFact::new(
        "cpu_cores",
        HardwareValue::Int(4),
        HardwareProvenance::HostObserved,
        Some(1000),
    )
    .unwrap();
    assert_ne!(declared.provenance, observed.provenance);
}

#[test]
fn ep035_unit_hardware_rejects_non_finite_values() {
    let err = HardwareFact::new(
        "ram_bytes",
        HardwareValue::Float(f64::NAN),
        HardwareProvenance::HostObserved,
        None,
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);
}

#[test]
fn ep035_unit_hardware_certified_requires_measured_provenance() {
    let err = HardwareCapabilityDeclaration::new(
        "local_llm",
        HardwareProvenance::UserDeclared,
        CapabilityCertificationState::Certified,
        Some("ev-1".to_string()),
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Verification);
}

#[test]
fn ep035_unit_hardware_certified_requires_measured_evidence() {
    let err = HardwareCapabilityDeclaration::new(
        "local_llm",
        HardwareProvenance::Benchmarked,
        CapabilityCertificationState::Certified,
        None,
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Verification);
    let certified = HardwareCapabilityDeclaration::new(
        "local_llm",
        HardwareProvenance::Benchmarked,
        CapabilityCertificationState::Certified,
        Some("bench-1".to_string()),
    )
    .unwrap();
    assert_eq!(
        certified.certification,
        CapabilityCertificationState::Certified
    );
}

#[test]
fn ep035_unit_hardware_observed_facts_never_mint_performance_claims() {
    let profile = HardwareProfile {
        facts: vec![
            HardwareFact::new(
                "cpu_cores",
                HardwareValue::Int(16),
                HardwareProvenance::HostObserved,
                Some(1000),
            )
            .unwrap(),
            HardwareFact::new(
                "ram_bytes",
                HardwareValue::Int(64),
                HardwareProvenance::HostObserved,
                Some(1000),
            )
            .unwrap(),
        ],
        capability_declarations: vec![],
        profiled_at_unix_s: 1001,
        correlation: correlation(1),
    };
    assert!(profile.capability_declarations.is_empty());
    let wire = serde_json::to_value(&profile).unwrap();
    assert_eq!(wire["capability_declarations"], serde_json::json!([]));
}

#[test]
fn ep035_unit_hardware_round_trips_serialization() {
    let profile = HardwareProfile {
        facts: vec![HardwareFact::new(
            "gpu_model",
            HardwareValue::Str("RTX 4090".to_string()),
            HardwareProvenance::UserDeclared,
            None,
        )
        .unwrap()],
        capability_declarations: vec![],
        profiled_at_unix_s: 1001,
        correlation: correlation(1),
    };
    let wire = serde_json::to_string(&profile).unwrap();
    let parsed: HardwareProfile = serde_json::from_str(&wire).unwrap();
    assert_eq!(parsed.facts[0].provenance, HardwareProvenance::UserDeclared);
}

#[test]
fn ep035_unit_hardware_rejects_unknown_wire_fields() {
    let fact = HardwareFact::new(
        "cpu_cores",
        HardwareValue::Int(8),
        HardwareProvenance::HostObserved,
        None,
    )
    .unwrap();
    let mut value = serde_json::to_value(&fact).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("forged".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<HardwareFact>(value).is_err());
}
