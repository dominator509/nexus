//! Sandboxed legacy Connector Sidecar adapter (SPEC-022 behavior 5).
//!
//! The sidecar wraps legacy sources - REST, SOAP, GraphQL, SQL read
//! replicas, ODBC/JDBC, CLI, files, email, webhooks, browser, or
//! desktop GUI as a last resort - inside a sandbox. The adapter is
//! the port: the transport implementation and sandbox boundary are
//! proven in later EP-011 milestones. The sidecar never holds direct
//! authority; commands stay idempotent and events stay versioned.

use serde::{Deserialize, Serialize};

use nexus_capabilities::context::InvocationContext;

use crate::error::{SdkError, SdkErrorCode};
use crate::vocabulary::SidecarTransport;

/// A normalized request to a sandboxed sidecar transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarRequest {
    /// Capability/connector the request targets.
    pub capability_id: String,
    /// Transport family to wrap.
    pub transport: SidecarTransport,
    /// Canonical action name (never a raw provider method string).
    pub action: String,
    /// Schema-validated canonical input.
    pub input: serde_json::Value,
    /// Idempotency key for commands (SPEC-022 behavior 2).
    pub idempotency_key: Option<String>,
    /// Invocation context.
    pub context: InvocationContext,
}

/// Normalized sidecar response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarResponse {
    /// Capability/connector the response belongs to.
    pub capability_id: String,
    /// Schema-validated canonical output.
    pub output: serde_json::Value,
    /// Stable cursor for change/state tracking, when applicable.
    pub cursor: Option<String>,
}

/// Error produced by a sidecar adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarAdapterError(pub SdkError);

impl std::fmt::Display for SidecarAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sidecar adapter: {}", self.0)
    }
}

impl std::error::Error for SidecarAdapterError {}

/// Port for a sandboxed legacy connector sidecar transport.
///
/// Implementations wrap exactly one transport family and must
/// normalize provider payloads at the infrastructure boundary: free
/// form provider responses never become domain contracts. Failures
/// are typed and fail closed.
pub trait SidecarAdapter: Send + Sync {
    /// Transport family wrapped by this adapter.
    fn transport(&self) -> SidecarTransport;

    /// Execute one normalized sidecar request.
    fn execute(&self, request: SidecarRequest) -> Result<SidecarResponse, SidecarAdapterError>;
}

/// Construct a typed sidecar error from a raw failure.
pub fn sidecar_error(
    code: SdkErrorCode,
    message: impl Into<String>,
    request: &SidecarRequest,
) -> SidecarAdapterError {
    SidecarAdapterError(SdkError::new(
        code,
        message,
        Some(request.context.correlation_id.to_string()),
        Some(request.context.external_actor_id.clone()),
        Some(request.context.tenant_id.to_string()),
        Some(request.capability_id.clone()),
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

    fn req() -> SidecarRequest {
        SidecarRequest {
            capability_id: "legacy.erp".to_string(),
            transport: SidecarTransport::Soap,
            action: "read.invoice".to_string(),
            input: serde_json::json!({ "invoice_id": "INV-1" }),
            idempotency_key: Some("op-1".to_string()),
            context: ctx(),
        }
    }

    struct StubSoap;

    impl SidecarAdapter for StubSoap {
        fn transport(&self) -> SidecarTransport {
            SidecarTransport::Soap
        }
        fn execute(&self, request: SidecarRequest) -> Result<SidecarResponse, SidecarAdapterError> {
            if request.input["invoice_id"] == "INV-1" {
                Ok(SidecarResponse {
                    capability_id: request.capability_id,
                    output: serde_json::json!({ "total": 100 }),
                    cursor: None,
                })
            } else {
                Err(sidecar_error(
                    SdkErrorCode::ExternalProvider,
                    "soap fault",
                    &request,
                ))
            }
        }
    }

    #[test]
    fn ep011_unit_sidecar_adapter_transport_and_execute() {
        let adapter = StubSoap;
        assert_eq!(adapter.transport(), SidecarTransport::Soap);
        let response = adapter.execute(req()).unwrap();
        assert_eq!(response.output["total"], 100);
    }

    #[test]
    fn ep011_unit_sidecar_adapter_fails_closed_typed() {
        let adapter = StubSoap;
        let mut request = req();
        request.input = serde_json::json!({ "invoice_id": "INV-2" });
        let err = adapter.execute(request).unwrap_err();
        assert_eq!(err.0.code, SdkErrorCode::ExternalProvider);
        assert_eq!(err.0.resource.as_deref(), Some("legacy.erp"));
    }

    #[test]
    fn ep011_unit_sidecar_request_serializes_transport() {
        let json = serde_json::to_value(req()).unwrap();
        assert_eq!(json["transport"], "SOAP");
        assert_eq!(json["action"], "read.invoice");
        assert_eq!(json["idempotency_key"], "op-1");
    }
}
