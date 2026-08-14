//! Versioned REST contract (SPEC-003 required behavior 1).

use crate::error::FabricError;
use nexus_domain::TenantId;
use nexus_identity::Principal;
use serde::{Deserialize, Serialize};

/// Canonical REST request carrying authenticated context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestRequest {
    /// Tenant resolved from AUTHENTICATED identity (never untrusted
    /// metadata; SPEC-003 acceptance obligation 3).
    pub tenant_id: TenantId,
    pub principal: Principal,
    pub correlation_id: String,
    pub path: String,
    pub method: String,
    pub body: serde_json::Value,
}

/// Canonical REST response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

/// An owned REST endpoint (path + method + handler).
#[derive(Debug, Clone)]
pub struct RestEndpoint {
    pub path: String,
    pub method: String,
    pub handler: fn(RestRequest) -> Result<RestResponse, FabricError>,
}

impl RestEndpoint {
    pub fn new(
        path: impl Into<String>,
        method: impl Into<String>,
        handler: fn(RestRequest) -> Result<RestResponse, FabricError>,
    ) -> Self {
        Self {
            path: path.into(),
            method: method.into(),
            handler,
        }
    }
}

/// Provider-neutral REST API port.
pub trait RestApi {
    /// Register an endpoint. Duplicate path+method is a conflict.
    fn register(&mut self, endpoint: RestEndpoint) -> Result<(), FabricError>;
    /// Dispatch a request to the matching endpoint; unknown routes are
    /// typed NOT_FOUND; tenant/principal come from the authenticated
    /// request only.
    fn dispatch(&self, request: RestRequest) -> Result<RestResponse, FabricError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_handler(_req: RestRequest) -> Result<RestResponse, FabricError> {
        Ok(RestResponse {
            status: 200,
            body: serde_json::json!({"ok": true}),
        })
    }

    #[test]
    fn ep012_unit_rest_endpoint_construction() {
        let ep = RestEndpoint::new("/v1/status", "GET", ok_handler);
        assert_eq!(ep.path, "/v1/status");
        assert_eq!(ep.method, "GET");
    }

    #[test]
    fn ep012_unit_rest_request_carries_authenticated_tenant() {
        let tenant: TenantId = "018f0f6f-9c1e-7b6e-8000-000000000003".parse().unwrap();
        let principal = Principal::new(
            "018f0f6f-9c1e-7b6e-8000-00000000000a".parse().unwrap(),
            nexus_domain::PrincipalType::Human,
            tenant.clone(),
        );
        let req = RestRequest {
            tenant_id: tenant.clone(),
            principal,
            correlation_id: "corr-1".into(),
            path: "/v1/status".into(),
            method: "GET".into(),
            body: serde_json::json!({}),
        };
        assert_eq!(req.tenant_id, tenant);
        assert_eq!(req.correlation_id, "corr-1");
    }
}
