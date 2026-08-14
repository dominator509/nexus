//! Legacy poller (SPEC-022 behavior 5).
//!
//! The `LegacyPoller` port wraps legacy sources that only expose
//! polling (REST, SOAP, SQL, CLI, files, email, browser as last
//! resort) and normalizes their outputs into versioned, correlated
//! events with stable cursors. Polling is stateful: the cursor is the
//! only continuity contract; a poller never claims exactly-once
//! delivery.

use serde::{Deserialize, Serialize};

use nexus_capabilities::context::InvocationContext;

use crate::error::{SdkError, SdkErrorCode};
use crate::vocabulary::{LegacyTransport, WebhookEvent};

/// One normalized polled batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolledBatch {
    /// Capability/connector the batch belongs to.
    pub capability_id: String,
    /// Versioned events normalized from the legacy source.
    pub events: Vec<WebhookEvent>,
    /// Stable cursor for the next poll.
    pub next_cursor: String,
}

/// Error produced by a legacy poller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyPollerError(pub SdkError);

impl std::fmt::Display for LegacyPollerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "legacy poller: {}", self.0)
    }
}

impl std::error::Error for LegacyPollerError {}

/// Port for a legacy polling source.
pub trait LegacyPoller: Send + Sync {
    /// Legacy transport family wrapped by this poller.
    fn transport(&self) -> LegacyTransport;

    /// Poll the legacy source from `cursor`, returning normalized
    /// events plus the next cursor. A poll failure is typed and never
    /// converted into an empty success.
    fn poll(
        &self,
        capability_id: String,
        cursor: Option<String>,
        context: InvocationContext,
    ) -> Result<PolledBatch, LegacyPollerError>;
}

/// Construct a typed poller error.
pub fn poller_error(
    code: SdkErrorCode,
    message: impl Into<String>,
    capability_id: &str,
    context: &InvocationContext,
) -> LegacyPollerError {
    LegacyPollerError(SdkError::new(
        code,
        message,
        Some(context.correlation_id.to_string()),
        Some(context.external_actor_id.clone()),
        Some(context.tenant_id.to_string()),
        Some(capability_id.to_string()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{CorrelationId, NexusId, PrincipalType, TenantId};

    fn ctx() -> InvocationContext {
        InvocationContext::new(
            NexusId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
            "test",
            "user:alice",
            PrincipalType::Human,
            TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap(),
            Some("mcp".to_string()),
            None,
            None,
            None,
        )
        .unwrap()
    }

    struct StubCsvPoller;

    impl LegacyPoller for StubCsvPoller {
        fn transport(&self) -> LegacyTransport {
            LegacyTransport::Files
        }
        fn poll(
            &self,
            capability_id: String,
            cursor: Option<String>,
            context: InvocationContext,
        ) -> Result<PolledBatch, LegacyPollerError> {
            if cursor.as_deref() == Some("end") {
                return Ok(PolledBatch {
                    capability_id,
                    events: vec![],
                    next_cursor: "end".to_string(),
                });
            }
            Ok(PolledBatch {
                capability_id,
                events: vec![WebhookEvent {
                    event_id: "evt-poll-1".to_string(),
                    event_type: "legacy.record.created".to_string(),
                    version: "1".to_string(),
                    correlation_id: context.correlation_id.to_string(),
                    payload: serde_json::json!({ "row": 1 }),
                }],
                next_cursor: "end".to_string(),
            })
        }
    }

    #[test]
    fn ep011_unit_legacy_poller_normalizes_batch() {
        let poller = StubCsvPoller;
        assert_eq!(poller.transport(), LegacyTransport::Files);
        let batch = poller.poll("legacy.csv".to_string(), None, ctx()).unwrap();
        assert_eq!(batch.events.len(), 1);
        assert_eq!(batch.events[0].event_type, "legacy.record.created");
        assert_eq!(batch.next_cursor, "end");
    }

    #[test]
    fn ep011_unit_legacy_poller_empty_batch_at_cursor() {
        let poller = StubCsvPoller;
        let batch = poller
            .poll("legacy.csv".to_string(), Some("end".to_string()), ctx())
            .unwrap();
        assert!(batch.events.is_empty());
        assert_eq!(batch.next_cursor, "end");
    }
}
