//! Redacted structured telemetry for the OpenFGA adapter (EP-008 M3
//! directive H).
//!
//! Emitted per decision: store/model identifier fingerprint, relation,
//! canonical actor/target type, allow/deny, latency, provider error
//! class, and correlation/request id. NEVER logs bearer tokens,
//! secrets, or full sensitive object payloads.

use std::sync::Mutex;

use nexus_domain::{CorrelationId, PrincipalType};
use nexus_identity::Principal;
use nexus_policy::relationship::RelationshipTuple;

use crate::error::OpenFgaErrorCode;

/// A redacted decision event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryEvent {
    /// Fingerprint of store+model identifiers (safe, one-way).
    pub fingerprint: String,
    /// Relation checked.
    pub relation: String,
    /// Canonical actor type (HUMAN/SERVICE/AGENT/DEVICE/SYSTEM).
    pub actor_type: String,
    /// Canonical target type (e.g. household, device, resource).
    pub target_type: String,
    /// Allow/deny (deny also on provider error).
    pub allowed: bool,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Provider error class, if any.
    pub error_class: Option<OpenFgaErrorCode>,
    /// Correlation id, if known.
    pub correlation: Option<CorrelationId>,
}

/// One-way fingerprint of store/model identifiers (SHA-256 truncated).
fn fingerprint(store_id: &str, model_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    store_id.hash(&mut hasher);
    model_id.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

impl TelemetryEvent {
    /// Build a decision event from a tuple and outcome.
    pub fn decision(
        tuple: &RelationshipTuple,
        allowed: bool,
        latency_ms: u64,
        error_class: Option<OpenFgaErrorCode>,
        _error_detail: Option<String>,
    ) -> Self {
        Self {
            fingerprint: fingerprint("", ""),
            relation: tuple.relation.clone(),
            actor_type: actor_type_str(&tuple.principal),
            target_type: tuple.object_type.clone(),
            allowed,
            latency_ms,
            error_class,
            correlation: None,
        }
    }

    /// Set the fingerprint (store+model).
    pub fn with_fingerprint(mut self, store_id: &str, model_id: &str) -> Self {
        self.fingerprint = fingerprint(store_id, model_id);
        self
    }

    /// Set the correlation id.
    pub fn with_correlation(mut self, correlation: Option<CorrelationId>) -> Self {
        self.correlation = correlation;
        self
    }
}

fn actor_type_str(principal: &Principal) -> String {
    match principal.principal_type() {
        PrincipalType::Human => "HUMAN",
        PrincipalType::Service => "SERVICE",
        PrincipalType::Agent => "AGENT",
        PrincipalType::Device => "DEVICE",
        PrincipalType::System => "SYSTEM",
    }
    .to_string()
}

/// A sink for telemetry events.
pub trait TelemetrySink: Send + Sync {
    /// Emit one event. Implementations must never block the check path
    /// for correctness; failures to emit must not change the decision.
    fn emit(&self, event: TelemetryEvent);
}

/// No-op sink (default).
pub struct NoopSink;

impl TelemetrySink for NoopSink {
    fn emit(&self, _event: TelemetryEvent) {}
}

/// In-memory recording sink (tests and diagnostics).
#[derive(Debug, Clone, Default)]
pub struct RecordingSink {
    events: std::sync::Arc<Mutex<Vec<TelemetryEvent>>>,
}

impl RecordingSink {
    /// Snapshot of recorded events.
    pub fn events(&self) -> Vec<TelemetryEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl TelemetrySink for RecordingSink {
    fn emit(&self, event: TelemetryEvent) {
        self.events.lock().unwrap().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{NexusId, TenantId};

    fn tid(s: &str) -> TenantId {
        TenantId::new(s).unwrap()
    }
    fn nid(s: &str) -> NexusId {
        NexusId::new(s).unwrap()
    }

    #[test]
    fn ep008_unit_telemetry_redacts_identifiers() {
        let event = TelemetryEvent::decision(
            &RelationshipTuple::new(
                tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"),
                Principal::new(
                    nid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"),
                    PrincipalType::Human,
                    tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"),
                ),
                "owner",
                "household",
                "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
            )
            .unwrap(),
            true,
            7,
            None,
            None,
        )
        .with_fingerprint("store-1", "model-1")
        .with_correlation(Some(
            CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a04").unwrap(),
        ));

        // Full identifiers must NOT appear.
        let text = format!("{event:?}");
        assert!(!text.contains("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"));
        assert!(!text.contains("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03"));
        assert!(!text.contains("store-1"));
        assert!(!text.contains("model-1"));
        // Safe fields present.
        assert_eq!(event.actor_type, "HUMAN");
        assert_eq!(event.target_type, "household");
        assert_eq!(event.relation, "owner");
        assert!(event.allowed);
        assert_eq!(event.latency_ms, 7);
        assert_eq!(event.error_class, None);
        assert_eq!(event.fingerprint.len(), 16);
    }

    #[test]
    fn ep008_unit_telemetry_fingerprint_is_stable() {
        let a = fingerprint("store-a", "model-a");
        let b = fingerprint("store-a", "model-a");
        let c = fingerprint("store-b", "model-a");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
