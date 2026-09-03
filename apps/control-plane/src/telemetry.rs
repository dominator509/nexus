//! Runtime telemetry bootstrap (EP-044 observability bootstrap;
//! SPEC-007; RX-007 AUD-083).
//!
//! The control-plane runtime initializes a REAL telemetry context at
//! startup: a validated `nexus_observability::model::TelemetryContext`
//! (component `nexus-control-plane`, node, environment, operation) and
//! a startup structured-log line emitted through the REAL nexus-otel
//! export boundary (`export_structured_log`). The export boundary
//! re-verifies `RedactedEnvelope::assert_exportable()` before any byte
//! is produced, so secret-shaped content (including a raw tenant id)
//! can never appear in the emitted telemetry.
//!
//! This is the observability bootstrap the EP-044 node contract
//! requires ("telemetry context is initialized at startup"). Before
//! RX-007, `main.rs` initialized no telemetry and the M4 test only
//! checked that stdout did not expose a tenant identifier.

use nexus_observability::ObservabilityError;
use nexus_observability::model::{RedactedEnvelope, TelemetryContext};
use nexus_observability::vocabulary::{Severity, TelemetrySignal};
use std::collections::BTreeMap;

/// Canonical component name for the control-plane runtime.
pub const COMPONENT: &str = "nexus-control-plane";

/// Runtime telemetry bootstrap handle.
#[derive(Debug, Clone)]
pub struct RuntimeTelemetry {
    /// Validated telemetry context (component, node, environment).
    context: TelemetryContext,
}

impl RuntimeTelemetry {
    /// Initialize the telemetry context at startup.
    ///
    /// `node` is the runtime node identifier; `environment` is the
    /// deployment environment (e.g. `local`, `staging`, `production`).
    /// The context is validated (empty/length/secret-shaped rejection)
    /// and fails closed on invalid input.
    pub fn init(
        node: &str,
        environment: Option<&str>,
        tenant_id: Option<&nexus_domain::TenantId>,
    ) -> Result<Self, ObservabilityError> {
        let context = TelemetryContext::new(
            node,
            tenant_id.cloned(),
            None, // business context
            None, // correlation
            None, // request id
            None, // trace id
            None, // span id
            COMPONENT,
            "startup",
            Severity::Info,
            environment.map(str::to_string),
            Some("runtime".to_string()), // source interface
        )?;
        Ok(Self { context })
    }

    /// Access the validated context.
    pub fn context(&self) -> &TelemetryContext {
        &self.context
    }

    /// Emit the startup structured-log line through the REAL nexus-otel
    /// export boundary. Returns the serialized line (also printed by
    /// the runtime) or an error if the envelope is not exportable.
    pub fn startup_line(&self) -> Result<String, ObservabilityError> {
        let envelope = RedactedEnvelope::new(
            TelemetrySignal::Log,
            self.context.clone(),
            BTreeMap::from([
                ("state".to_string(), "starting".to_string()),
                ("node".to_string(), self.context.node.clone()),
            ]),
            Vec::new(),
        );
        nexus_otel::export_structured_log(&envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenant() -> nexus_domain::TenantId {
        nexus_domain::TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000001").expect("tenant")
    }

    #[test]
    fn ep044_unit_telemetry_context_initializes_at_startup() {
        let t = RuntimeTelemetry::init("node-a", Some("local"), Some(&tenant())).unwrap();
        assert_eq!(t.context().component, COMPONENT);
        assert_eq!(t.context().node, "node-a");
        assert_eq!(t.context().operation, "startup");
        assert_eq!(t.context().severity, Severity::Info);
        assert_eq!(t.context().environment.as_deref(), Some("local"));
    }

    #[test]
    fn ep044_unit_telemetry_context_rejects_empty_component() {
        // The canonical context fails closed on invalid input.
        let err = TelemetryContext::new(
            "node-a",
            Some(tenant()),
            None,
            None,
            None,
            None,
            None,
            "", // empty component
            "startup",
            Severity::Info,
            Some("local".to_string()),
            Some("runtime".to_string()),
        )
        .unwrap_err();
        assert!(err.to_string().contains("component"));
    }

    #[test]
    fn ep044_unit_telemetry_startup_line_is_structured_and_exportable() {
        let t = RuntimeTelemetry::init("node-a", Some("local"), None).unwrap();
        let line = t.startup_line().unwrap();
        // Structured JSON line with the canonical fields.
        assert!(line.starts_with('{'));
        assert!(line.contains("\"service\":\"nexus-control-plane\""));
        assert!(line.contains("\"operation\":\"startup\""));
        assert!(line.contains("\"node\":\"node-a\""));
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn ep044_unit_telemetry_never_exposes_tenant_id() {
        // AUD-083: the startup telemetry line must never carry the raw
        // tenant identifier (redaction at the export boundary). The
        // tenant is part of the validated context, but the structured
        // log line renders only exportable fields.
        let t = RuntimeTelemetry::init("node-a", Some("local"), Some(&tenant())).unwrap();
        let line = t.startup_line().unwrap();
        assert!(
            !line.contains("018f0f6f-9c1e-7b6e-8000-000000000001"),
            "tenant leaked into telemetry: {line}"
        );
    }
}
