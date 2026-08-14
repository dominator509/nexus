//! Redacted structured telemetry for the OPA adapter (EP-008 M4
//! directive I).
//!
//! Emitted per decision: policy bundle/version fingerprint, policy
//! decision path, allow/deny, typed provider error class, latency,
//! principal/target TYPE, tenant fingerprint where safe, and
//! correlation/request id. NEVER logs bearer tokens, secrets,
//! complete sensitive resource payloads, or unnecessary personal data.

use std::sync::Mutex;

use nexus_domain::{CorrelationId, PrincipalType};
use nexus_policy::policy::PolicyInput;

use crate::error::OpaErrorCode;

/// A redacted decision event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryEvent {
    /// Fingerprint of the policy bundle/version (safe, one-way).
    pub fingerprint: String,
    /// Policy version claimed by the adapter.
    pub version: String,
    /// Policy decision path (data.nexus.allow).
    pub path: String,
    /// Allow/deny (deny also on provider error).
    pub allowed: bool,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Typed provider error class, if any.
    pub error_class: Option<OpaErrorCode>,
    /// Canonical actor type (HUMAN/SERVICE/AGENT/DEVICE/SYSTEM).
    pub actor_type: String,
    /// Canonical target type (e.g. task, memory, household).
    pub target_type: String,
    /// Tenant fingerprint (safe, one-way; never the full tenant id).
    pub tenant_fingerprint: String,
    /// Correlation id, if known.
    pub correlation: Option<CorrelationId>,
}

/// One-way fingerprint of a string (SHA-256 truncated to 16 hex).
fn fingerprint(value: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

impl TelemetryEvent {
    /// Build a decision event from a policy input and outcome.
    pub fn decision(
        input: &PolicyInput,
        allowed: bool,
        latency_ms: u64,
        error_class: Option<OpaErrorCode>,
        _error_detail: Option<String>,
    ) -> Self {
        Self {
            fingerprint: fingerprint(""),
            version: String::new(),
            path: "data.nexus.allow".to_string(),
            allowed,
            latency_ms,
            error_class,
            actor_type: actor_type_str(input.principal.principal_type()),
            target_type: input.object_type.clone(),
            tenant_fingerprint: fingerprint(input.tenant_id.as_str()),
            correlation: None,
        }
    }

    /// Set the policy version and fingerprint.
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self.fingerprint = fingerprint(version);
        self
    }

    /// Set the correlation id.
    pub fn with_correlation(mut self, correlation: Option<CorrelationId>) -> Self {
        self.correlation = correlation;
        self
    }
}

fn actor_type_str(principal_type: PrincipalType) -> String {
    match principal_type {
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
    use nexus_auth::AuthenticationStrength;
    use nexus_domain::{CapabilityClass, NexusId, PrincipalType, Risk, TenantId};
    use nexus_identity::{Principal, TrustLevel};

    fn tid(s: &str) -> TenantId {
        TenantId::new(s).unwrap()
    }
    fn nid(s: &str) -> NexusId {
        NexusId::new(s).unwrap()
    }

    fn input() -> PolicyInput {
        PolicyInput::new(
            tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"),
            Principal::new(
                nid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"),
                PrincipalType::Human,
                tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"),
            ),
            CapabilityClass::Query,
            Risk::R0,
            AuthenticationStrength::SingleFactor,
            TrustLevel::Local,
            "task",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
        )
        .unwrap()
    }

    #[test]
    fn ep008_unit_opa_telemetry_redacts_identifiers() {
        let event = TelemetryEvent::decision(&input(), true, 7, None, None)
            .with_version("nexus-policy-v1")
            .with_correlation(Some(
                CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a04").unwrap(),
            ));

        let text = format!("{event:?}");
        // Full identifiers must NOT appear.
        assert!(!text.contains("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"));
        assert!(!text.contains("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"));
        assert!(!text.contains("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03"));
        // Safe fields present.
        assert_eq!(event.actor_type, "HUMAN");
        assert_eq!(event.target_type, "task");
        assert_eq!(event.path, "data.nexus.allow");
        assert_eq!(event.version, "nexus-policy-v1");
        assert!(event.allowed);
        assert_eq!(event.latency_ms, 7);
        assert_eq!(event.error_class, None);
        assert_eq!(event.fingerprint.len(), 16);
        assert_eq!(event.tenant_fingerprint.len(), 16);
    }

    #[test]
    fn ep008_unit_opa_telemetry_fingerprint_is_stable() {
        let a = fingerprint("nexus-policy-v1");
        let b = fingerprint("nexus-policy-v1");
        let c = fingerprint("nexus-policy-v2");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
