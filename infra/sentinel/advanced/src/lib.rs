//! EP-031 provider-neutral advanced sentinel contracts (SPEC-013).
//!
//! Advanced Sentinel builds on Sentinel Core: optional sensor
//! profiles (Suricata, Zeek, CrowdSec, Wazuh, osquery, honeypots),
//! alert correlation into incidents, bounded triage, investigation,
//! response planning, and verification. Enhanced profile adds
//! Suricata; Advanced adds Zeek; Endpoint adds Wazuh or osquery;
//! CrowdSec is optional reputation enforcement; honeypots are
//! optional high-signal sensors isolated from real data. This crate
//! owns the provider-neutral contract layer; connector
//! implementations live under connectors/suricata, connectors/zeek,
//! connectors/crowdsec, connectors/wazuh, connectors/osquery (M2+).
//! M1 owns the vocabulary, value objects, and fail-closed provider
//! ports and services.
//!
//! Permanent invariants (SPEC-013):
//! - Advanced sensors are optional profiles; unbound providers
//!   advertise nothing and fail closed (Reality rule).
//! - Alerts correlate into incidents instead of flooding users.
//! - High-confidence bounded quarantine can be preauthorized.
//! - Destructive response (wipes, factory resets, broad lockouts,
//!   credential rotation) remains human controlled and is never
//!   auto-applicable.
//! - Free-form provider payloads are normalized at the infrastructure
//!   boundary and never become domain contracts.
//!
//! Dependency direction: this crate depends only on nexus-domain,
//! nexus-sentinel (contract crates), and serde/serde_json. Provider
//! implementations never appear here.

#![forbid(unsafe_code)]

pub mod error;
pub mod model;
pub mod provider;
pub mod vocabulary;

pub use error::{AdvancedSentinelError, SentinelErrorCode};
pub use model::{
    HoneypotRecord, Incident, InvestigationCase, ResponsePlan, SecurityEvent, TriageCase,
    VerificationRecord,
};
pub use provider::{
    EndpointTelemetryProvider, HoneypotProvider, NetworkDetectionProvider, ResponsePlanner,
    SecurityInvestigator, SecurityTriage, SecurityVerifier, ThreatIntelProvider,
    UnboundEndpointTelemetryProvider, UnboundHoneypotProvider, UnboundNetworkDetectionProvider,
    UnboundResponsePlanner, UnboundSecurityInvestigator, UnboundSecurityTriage,
    UnboundSecurityVerifier, UnboundThreatIntelProvider,
};
pub use vocabulary::{
    AdvancedSensorProfile, AlertState, CorrelationConfidence, HoneypotId, HoneypotKind,
    HoneypotState, IncidentCorrelationId, IncidentState, InvestigationCaseId, InvestigationState,
    ResponseKind, ResponsePlanId, ResponsePlanState, SecurityEventId, TriageCaseId, TriagePriority,
    VerificationRecordId, VerificationState,
};
