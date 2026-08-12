//! EventEnvelope <-> JetStream message encoding.
//!
//! The canonical wire form is the JSON serialization of `EventEnvelope`
//! (snake_case, closed object). Decoding validates the envelope
//! invariants before it is handed to a consumer, so malformed messages
//! never reach application logic.

use nexus_events::{EventEnvelope, EventError, EventErrorCode};

/// Serialize an envelope to the JetStream payload bytes.
pub fn encode(envelope: &EventEnvelope) -> Result<Vec<u8>, EventError> {
    envelope.validate()?;
    serde_json::to_vec(envelope).map_err(EventError::from)
}

/// Deserialize and validate JetStream payload bytes.
pub fn decode(bytes: &[u8]) -> Result<EventEnvelope, EventError> {
    let envelope: EventEnvelope = serde_json::from_slice(bytes).map_err(|e| {
        EventError::new(
            EventErrorCode::Validation,
            format!("invalid event envelope: {e}"),
        )
    })?;
    envelope.validate()?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{CorrelationId, EventId, TenantId};
    use nexus_events::{EventDataClass, EventType};

    fn envelope() -> EventEnvelope {
        EventEnvelope {
            event_id: EventId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb010").unwrap(),
            event_type: EventType::new("memory.record.created").unwrap(),
            schema_version: "1.0.0".to_string(),
            source: "voice".to_string(),
            subject: "nexus.memory.record.created.t".to_string(),
            time: "2026-08-12T00:00:00Z".to_string(),
            tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb011").unwrap(),
            actor: "principal".to_string(),
            correlation_id: CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fb012").unwrap(),
            causation_id: None,
            data_class: EventDataClass::Security,
            payload: serde_json::json!({ "k": "v" }),
        }
    }

    #[test]
    fn ep005_unit_encode_decode_round_trips() {
        let e = envelope();
        let bytes = encode(&e).unwrap();
        let back = decode(&bytes).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn ep005_unit_decode_rejects_garbage() {
        let err = decode(b"not-json").unwrap_err();
        assert_eq!(err.code(), EventErrorCode::Validation);
    }

    #[test]
    fn ep005_unit_decode_rejects_unknown_fields() {
        let e = envelope();
        let mut v = serde_json::to_value(&e).unwrap();
        v.as_object_mut()
            .unwrap()
            .insert("evil".into(), serde_json::json!(1));
        let err = decode(&serde_json::to_vec(&v).unwrap()).unwrap_err();
        assert_eq!(err.code(), EventErrorCode::Validation);
    }

    #[test]
    fn ep005_unit_encode_rejects_invalid_envelope() {
        let mut e = envelope();
        e.schema_version = "9.9.9".to_string();
        let err = encode(&e).unwrap_err();
        assert_eq!(err.code(), EventErrorCode::Validation);
    }
}
