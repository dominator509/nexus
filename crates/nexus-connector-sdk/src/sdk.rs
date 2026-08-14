//! Connector SDK surface (SPEC-022 behavior 4).
//!
//! `ConnectorSdk` is the shared contract corpus that the Rust,
//! TypeScript, and Python SDKs implement: typed capability discovery,
//! query, command (idempotent), health, and change-feed access through
//! the EP-010 capability ports. `SdkLanguage` marks which language
//! surface a binding exposes; the contract version is the shared
//! corpus version, so a Rust binding and a TypeScript binding can be
//! proven to speak the same contract.
//!
//! The SDK never grants authority: it discovers and invokes through
//! the capability registry/dispatcher and the EP-008 authorization
//! boundary. Discovery results are metadata only.

use serde::{Deserialize, Serialize};

use nexus_capabilities::changefeed::{ChangeBatch, ChangeCursor, ChangeFeedCapability};
use nexus_capabilities::command::{CommandCapability, CommandRequest, CommandResult};
use nexus_capabilities::context::InvocationContext;
use nexus_capabilities::descriptor::CapabilityDescriptor;
use nexus_capabilities::error::{CapabilityError, CapabilityErrorCode};
use nexus_capabilities::health::{HealthCapability, HealthReport};
use nexus_capabilities::query::{QueryCapability, QueryRequest, QueryResult};
use nexus_capabilities::registry::CapabilityRegistry;

use crate::error::{SdkError, SdkErrorCode};
use crate::vocabulary::SdkLanguage;

/// Shared contract corpus version (SPEC-022 behavior 4).
pub const CONTRACT_VERSION: &str = "1.0.0";

/// The shared connector SDK contract. Every language binding
/// (Rust/TypeScript/Python) implements this surface; the same
/// conformance suite must pass against each implementation.
pub trait ConnectorSdk: Send + Sync {
    /// Language of this binding.
    fn language(&self) -> SdkLanguage;

    /// Contract corpus version implemented by this binding.
    fn contract_version(&self) -> &'static str {
        CONTRACT_VERSION
    }

    /// Discover available capabilities for a tenant (metadata only).
    fn discover(
        &self,
        registry: &dyn CapabilityRegistry,
        context: InvocationContext,
    ) -> Result<Vec<CapabilityDescriptor>, SdkError>;

    /// Execute a typed query.
    fn query<Q: QueryCapability>(
        &self,
        request: QueryRequest,
        port: &Q,
    ) -> Result<QueryResult, SdkError>;

    /// Execute an idempotent command.
    fn command<C: CommandCapability>(
        &self,
        request: CommandRequest,
        port: &C,
    ) -> Result<CommandResult, SdkError>;

    /// Read capability health (observation only).
    fn health<H: HealthCapability>(
        &self,
        capability_id: String,
        context: InvocationContext,
        port: &H,
    ) -> Result<HealthReport, SdkError>;

    /// Read change-feed events.
    fn changefeed<F: ChangeFeedCapability>(
        &self,
        capability_id: String,
        cursor: Option<ChangeCursor>,
        context: InvocationContext,
        port: &F,
    ) -> Result<ChangeBatch, SdkError>;
}

/// Rust connector SDK binding (SPEC-022 behavior 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustConnectorSdk;

impl ConnectorSdk for RustConnectorSdk {
    fn language(&self) -> SdkLanguage {
        SdkLanguage::Rust
    }

    fn discover(
        &self,
        registry: &dyn CapabilityRegistry,
        context: InvocationContext,
    ) -> Result<Vec<CapabilityDescriptor>, SdkError> {
        registry
            .discover(&context.tenant_id, context.clone())
            .map_err(|e| map_capability_error(e, &context))
    }

    fn query<Q: QueryCapability>(
        &self,
        request: QueryRequest,
        port: &Q,
    ) -> Result<QueryResult, SdkError> {
        let context = request.context.clone();
        port.query(request)
            .map_err(|e| map_capability_error(e, &context))
    }

    fn command<C: CommandCapability>(
        &self,
        request: CommandRequest,
        port: &C,
    ) -> Result<CommandResult, SdkError> {
        let context = request.context.clone();
        port.command(request)
            .map_err(|e| map_capability_error(e, &context))
    }

    fn health<H: HealthCapability>(
        &self,
        _capability_id: String,
        context: InvocationContext,
        port: &H,
    ) -> Result<HealthReport, SdkError> {
        port.health(context.clone())
            .map_err(|e| map_capability_error(e, &context))
    }

    fn changefeed<F: ChangeFeedCapability>(
        &self,
        capability_id: String,
        cursor: Option<ChangeCursor>,
        context: InvocationContext,
        port: &F,
    ) -> Result<ChangeBatch, SdkError> {
        port.changes_since(capability_id, cursor, context.clone())
            .map_err(|e| map_capability_error(e, &context))
    }
}

/// TypeScript connector SDK binding (SPEC-022 behavior 4). The
/// TypeScript surface is generated from the canonical schemas; this
/// Rust-side marker records the language and contract version so the
/// conformance corpus is traceable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeScriptConnectorSdk;

impl ConnectorSdk for TypeScriptConnectorSdk {
    fn language(&self) -> SdkLanguage {
        SdkLanguage::TypeScript
    }

    fn discover(
        &self,
        registry: &dyn CapabilityRegistry,
        context: InvocationContext,
    ) -> Result<Vec<CapabilityDescriptor>, SdkError> {
        registry
            .discover(&context.tenant_id, context.clone())
            .map_err(|e| map_capability_error(e, &context))
    }

    fn query<Q: QueryCapability>(
        &self,
        request: QueryRequest,
        port: &Q,
    ) -> Result<QueryResult, SdkError> {
        let context = request.context.clone();
        port.query(request)
            .map_err(|e| map_capability_error(e, &context))
    }

    fn command<C: CommandCapability>(
        &self,
        request: CommandRequest,
        port: &C,
    ) -> Result<CommandResult, SdkError> {
        let context = request.context.clone();
        port.command(request)
            .map_err(|e| map_capability_error(e, &context))
    }

    fn health<H: HealthCapability>(
        &self,
        _capability_id: String,
        context: InvocationContext,
        port: &H,
    ) -> Result<HealthReport, SdkError> {
        port.health(context.clone())
            .map_err(|e| map_capability_error(e, &context))
    }

    fn changefeed<F: ChangeFeedCapability>(
        &self,
        capability_id: String,
        cursor: Option<ChangeCursor>,
        context: InvocationContext,
        port: &F,
    ) -> Result<ChangeBatch, SdkError> {
        port.changes_since(capability_id, cursor, context.clone())
            .map_err(|e| map_capability_error(e, &context))
    }
}

/// Python connector SDK binding (SPEC-022 behavior 4). See
/// `TypeScriptConnectorSdk` for the generated-binding rationale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PythonConnectorSdk;

impl ConnectorSdk for PythonConnectorSdk {
    fn language(&self) -> SdkLanguage {
        SdkLanguage::Python
    }

    fn discover(
        &self,
        registry: &dyn CapabilityRegistry,
        context: InvocationContext,
    ) -> Result<Vec<CapabilityDescriptor>, SdkError> {
        registry
            .discover(&context.tenant_id, context.clone())
            .map_err(|e| map_capability_error(e, &context))
    }

    fn query<Q: QueryCapability>(
        &self,
        request: QueryRequest,
        port: &Q,
    ) -> Result<QueryResult, SdkError> {
        let context = request.context.clone();
        port.query(request)
            .map_err(|e| map_capability_error(e, &context))
    }

    fn command<C: CommandCapability>(
        &self,
        request: CommandRequest,
        port: &C,
    ) -> Result<CommandResult, SdkError> {
        let context = request.context.clone();
        port.command(request)
            .map_err(|e| map_capability_error(e, &context))
    }

    fn health<H: HealthCapability>(
        &self,
        _capability_id: String,
        context: InvocationContext,
        port: &H,
    ) -> Result<HealthReport, SdkError> {
        port.health(context.clone())
            .map_err(|e| map_capability_error(e, &context))
    }

    fn changefeed<F: ChangeFeedCapability>(
        &self,
        capability_id: String,
        cursor: Option<ChangeCursor>,
        context: InvocationContext,
        port: &F,
    ) -> Result<ChangeBatch, SdkError> {
        port.changes_since(capability_id, cursor, context.clone())
            .map_err(|e| map_capability_error(e, &context))
    }
}

fn map_code(code: CapabilityErrorCode) -> SdkErrorCode {
    match code {
        CapabilityErrorCode::Validation => SdkErrorCode::Validation,
        CapabilityErrorCode::Authentication => SdkErrorCode::Authentication,
        CapabilityErrorCode::Authorization => SdkErrorCode::Authorization,
        CapabilityErrorCode::Policy => SdkErrorCode::Policy,
        CapabilityErrorCode::Unavailable => SdkErrorCode::Unavailable,
        CapabilityErrorCode::Timeout => SdkErrorCode::Timeout,
        CapabilityErrorCode::Conflict => SdkErrorCode::Conflict,
        CapabilityErrorCode::NotFound => SdkErrorCode::NotFound,
        CapabilityErrorCode::RateLimit => SdkErrorCode::RateLimit,
        CapabilityErrorCode::ExternalProvider => SdkErrorCode::ExternalProvider,
        CapabilityErrorCode::Verification => SdkErrorCode::Verification,
        CapabilityErrorCode::Compensation => SdkErrorCode::Compensation,
        CapabilityErrorCode::Internal => SdkErrorCode::Internal,
    }
}

fn boxed_to_string(value: Option<Box<str>>) -> Option<String> {
    value.map(|v| v.to_string())
}

fn map_capability_error(e: CapabilityError, context: &InvocationContext) -> SdkError {
    SdkError::new(
        map_code(e.code),
        e.to_string(),
        Some(boxed_to_string(e.correlation).unwrap_or_else(|| context.correlation_id.to_string())),
        boxed_to_string(e.actor),
        boxed_to_string(e.tenant),
        boxed_to_string(e.resource),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyRegistry;

    impl CapabilityRegistry for EmptyRegistry {
        fn register(
            &self,
            _descriptor: CapabilityDescriptor,
            _context: InvocationContext,
        ) -> Result<(), CapabilityError> {
            Ok(())
        }
        fn unregister(
            &self,
            _capability_id: &str,
            _context: InvocationContext,
        ) -> Result<(), CapabilityError> {
            Ok(())
        }
        fn discover(
            &self,
            _tenant_id: &nexus_domain::TenantId,
            _context: InvocationContext,
        ) -> Result<Vec<CapabilityDescriptor>, CapabilityError> {
            Ok(vec![])
        }
        fn resolve(
            &self,
            _capability_id: &str,
            _tenant_id: &nexus_domain::TenantId,
            _context: InvocationContext,
        ) -> Result<CapabilityDescriptor, CapabilityError> {
            Err(CapabilityError::new(
                CapabilityErrorCode::NotFound,
                "not found",
                None,
                None,
                None,
                None,
            ))
        }
    }

    fn ctx() -> InvocationContext {
        InvocationContext::new(
            nexus_domain::NexusId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            nexus_domain::CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
            "test",
            "user:alice",
            nexus_domain::PrincipalType::Human,
            nexus_domain::TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap(),
            Some("mcp".to_string()),
            None,
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn ep011_unit_sdk_bindings_expose_contract_version() {
        let rust = RustConnectorSdk;
        let ts = TypeScriptConnectorSdk;
        let py = PythonConnectorSdk;
        assert_eq!(rust.language(), SdkLanguage::Rust);
        assert_eq!(ts.language(), SdkLanguage::TypeScript);
        assert_eq!(py.language(), SdkLanguage::Python);
        assert_eq!(rust.contract_version(), CONTRACT_VERSION);
        assert_eq!(ts.contract_version(), CONTRACT_VERSION);
        assert_eq!(py.contract_version(), CONTRACT_VERSION);
    }

    #[test]
    fn ep011_unit_sdk_discover_empty_registry() {
        let rust = RustConnectorSdk;
        let registry = EmptyRegistry;
        let result = rust.discover(&registry, ctx());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
