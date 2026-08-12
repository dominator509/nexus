//! Canonical subject namespace derivation (SPEC-023 fallback doctrine).
//!
//! One canonical stream and subject namespace. Subjects are derived
//! deterministically from the envelope: `nexus.<domain>.<event>.<tenant>`.
//! No adapter or caller invents subjects outside this namespace.

use nexus_domain::TenantId;
use nexus_events::EventEnvelope;

/// Canonical stream name for the event nervous system.
pub const CANONICAL_STREAM: &str = "nexus";

/// Derive the canonical subject for an envelope.
///
/// The event type is already a dotted slug (e.g. `memory.record.created`);
/// the first two segments form the domain namespace and the remainder the
/// event name, then the tenant id is appended for tenant-scoped routing.
pub fn subject_for(envelope: &EventEnvelope) -> String {
    let tenant = envelope.tenant_id.as_str();
    let mut parts: Vec<&str> = envelope.event_type.as_str().split('.').collect();
    let event = if parts.len() >= 2 {
        parts.split_off(2).join(".")
    } else {
        parts.pop().unwrap_or("event").to_string()
    };
    let domain = if parts.is_empty() {
        "general".to_string()
    } else {
        parts.join(".")
    };
    format!("nexus.{domain}.{event}.{tenant}")
}

/// Wildcard subject for all events of a domain namespace.
pub fn domain_wildcard(domain: &str) -> String {
    format!("nexus.{domain}.>")
}

/// Wildcard subject for all events of one tenant.
pub fn tenant_wildcard(tenant: &TenantId) -> String {
    format!("nexus.>.>.{}", tenant.as_str())
}

/// The tenant-scoped subject filter for a durable consumer.
pub fn consumer_subject(domain: &str, tenant: &TenantId) -> String {
    format!("nexus.{domain}.>.{}", tenant.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{CorrelationId, EventId, TenantId};
    use nexus_events::{EventDataClass, EventType};

    fn tenant() -> TenantId {
        TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb001").unwrap()
    }

    fn envelope(event_type: &str) -> EventEnvelope {
        EventEnvelope {
            event_id: EventId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb002").unwrap(),
            event_type: EventType::new(event_type).unwrap(),
            schema_version: "1.0.0".to_string(),
            source: "test".to_string(),
            subject: "ignored".to_string(),
            time: "2026-08-12T00:00:00Z".to_string(),
            tenant_id: tenant(),
            actor: "principal".to_string(),
            correlation_id: CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb003").unwrap(),
            causation_id: None,
            data_class: EventDataClass::Household,
            payload: serde_json::json!({}),
        }
    }

    #[test]
    fn ep005_unit_subject_for_uses_canonical_namespace() {
        let e = envelope("memory.record.created");
        assert_eq!(
            subject_for(&e),
            "nexus.memory.record.created.0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb001"
        );
    }

    #[test]
    fn ep005_unit_subject_for_single_segment_type() {
        // A single-segment type has no domain namespace; it lives under
        // the `general` domain per the subject derivation contract.
        let e = envelope("heartbeat");
        assert_eq!(
            subject_for(&e),
            "nexus.general.heartbeat.0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb001"
        );
    }

    #[test]
    fn ep005_unit_wildcards_and_consumer_subject() {
        assert_eq!(domain_wildcard("memory"), "nexus.memory.>");
        assert_eq!(
            tenant_wildcard(&tenant()),
            "nexus.>.>.0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb001"
        );
        assert_eq!(
            consumer_subject("memory", &tenant()),
            "nexus.memory.>.0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb001"
        );
    }
}
