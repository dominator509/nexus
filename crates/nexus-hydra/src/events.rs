//! EP-028 Hydra durable event consumer (SPEC-015 behavior 2: Nexus
//! accesses Hydra through authenticated MCP, REST, and durable events;
//! SPEC-015 required test: Hydra capability and event contract).

use nexus_domain::{CorrelationId, EventId, TenantId};
use serde::{Deserialize, Serialize};

use crate::error::HydraError;

/// Versioned durable event envelope from Hydra. Event payloads are
/// referenced, never inlined as domain contracts (free-form provider
/// payloads are normalized at the infrastructure boundary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydraEventEnvelope {
    pub event_id: EventId,
    /// Canonical event type (vocabulary-locked by the owning schema).
    pub event_type: String,
    pub tenant_id: TenantId,
    pub correlation: Option<CorrelationId>,
    /// Reference to the normalized payload, not raw provider bytes.
    pub payload_ref: String,
    /// RFC3339 timestamp when the event occurred.
    pub occurred_at: String,
    /// Event contract version (versioned transport contract).
    pub version: u32,
}

/// Provider-neutral consumer of Hydra durable events. Implementations
/// (M2+) authenticate, redact, and normalize at the boundary.
pub trait HydraEventConsumer {
    fn consume(&self, envelope: HydraEventEnvelope) -> Result<(), HydraError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    #[test]
    fn ep028_unit_event_envelope_roundtrips_serde() {
        let env = HydraEventEnvelope {
            event_id: EventId::from_str("018f0f6f-9c1e-7b6e-8000-000000000004").unwrap(),
            event_type: "hydra.lead.updated".into(),
            tenant_id: tenant(),
            correlation: None,
            payload_ref: "events/lead-updated-1.json".into(),
            occurred_at: "2026-08-19T00:00:00Z".into(),
            version: 1,
        };
        let json = serde_json::to_string(&env).unwrap();
        let back: HydraEventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back, env);
        assert_eq!(back.version, 1);
    }

    #[test]
    fn ep028_unit_event_consumer_fails_closed() {
        struct DenyConsumer;
        impl HydraEventConsumer for DenyConsumer {
            fn consume(&self, _envelope: HydraEventEnvelope) -> Result<(), HydraError> {
                Err(HydraError::policy("consumer not certified"))
            }
        }
        let env = HydraEventEnvelope {
            event_id: EventId::from_str("018f0f6f-9c1e-7b6e-8000-000000000004").unwrap(),
            event_type: "hydra.lead.updated".into(),
            tenant_id: tenant(),
            correlation: None,
            payload_ref: "events/lead-updated-1.json".into(),
            occurred_at: "2026-08-19T00:00:00Z".into(),
            version: 1,
        };
        let err = DenyConsumer.consume(env).unwrap_err();
        assert_eq!(err.code, crate::error::HydraErrorCode::Policy);
    }
}
