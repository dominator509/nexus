//! Gateway telemetry (SPEC-007; EP-013 M2).
//!
//! Events are redacted: they carry correlation ids, provider ids,
//! and typed classes, never prompt text, model output, or provider
//! credentials. Telemetry is evidence, not authority.

use serde::{Deserialize, Serialize};

/// Telemetry event class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewayEventClass {
    RouteSelected,
    BudgetDenied,
    RateLimited,
    ProviderUnavailable,
    ProviderTimeout,
    ProviderError,
    Fallback,
    Retry,
    Allowed,
    Denied,
}

/// A redacted gateway telemetry event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayEvent {
    pub class: GatewayEventClass,
    pub correlation_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    pub provider_id: Option<String>,
    /// The typed SPEC-006 code when the event is a failure.
    pub error_code: Option<String>,
    /// Redacted message (never credentials or prompt content).
    pub message: String,
}

impl GatewayEvent {
    pub fn new(
        class: GatewayEventClass,
        correlation_id: impl Into<String>,
        tenant_id: impl Into<String>,
        principal_id: impl Into<String>,
        provider_id: Option<String>,
        error_code: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            correlation_id: correlation_id.into(),
            tenant_id: tenant_id.into(),
            principal_id: principal_id.into(),
            provider_id,
            error_code,
            message: message.into(),
        }
    }
}

/// In-memory telemetry sink (production logging lands in M3/M4 with
/// the canonical OpenTelemetry context). Recording is append-only.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GatewayTelemetry {
    events: Vec<GatewayEvent>,
}

impl GatewayTelemetry {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, event: GatewayEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[GatewayEvent] {
        &self.events
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn has_class(&self, class: GatewayEventClass) -> bool {
        self.events.iter().any(|e| e.class == class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep013_unit_telemetry_records_redacted_events() {
        let mut t = GatewayTelemetry::new();
        t.record(GatewayEvent::new(
            GatewayEventClass::RouteSelected,
            "c-1",
            "t-1",
            "p-1",
            Some("bifrost".to_string()),
            None,
            "route selected",
        ));
        assert_eq!(t.events().len(), 1);
        assert!(t.has_class(GatewayEventClass::RouteSelected));
        assert!(!t.has_class(GatewayEventClass::Denied));
        let v = serde_json::to_value(&t.events()[0]).unwrap();
        // The class is canonical and the message carries no secret.
        assert_eq!(v["class"], "ROUTE_SELECTED");
        assert_eq!(v["message"], "route selected");
    }

    #[test]
    fn ep013_unit_telemetry_event_class_round_trip() {
        assert_eq!(
            serde_json::to_value(GatewayEventClass::BudgetDenied).unwrap(),
            serde_json::json!("BUDGET_DENIED")
        );
    }
}
