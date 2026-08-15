//! EP-044 control-plane server (ADR-019 `ControlPlaneServer`).
//!
//! Real runnable HTTP server serving the canonical runtime endpoints:
//! `GET /healthz`, `GET /readyz`, `GET /v1/capabilities`. Graceful
//! startup/shutdown: bind once, serve, stop on signal, never leak
//! processes. No placeholder endpoints; the handlers are real and
//! fail closed.

use std::sync::{Arc, Mutex};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};

use crate::capabilities::{CapabilityList, CapabilityListSource};
use crate::config::ControlPlaneConfig;
use crate::error::{RuntimeError, RuntimeErrorCode};
use crate::health::RuntimeHealth;
use crate::lifecycle::RuntimeLifecycle;
use crate::readiness::RuntimeReadiness;

/// Server construction error (fail closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneServerError(pub String);

impl std::fmt::Display for ControlPlaneServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "control plane server: {}", self.0)
    }
}

impl std::error::Error for ControlPlaneServerError {}

impl From<ControlPlaneServerError> for RuntimeError {
    fn from(value: ControlPlaneServerError) -> Self {
        RuntimeError::new(RuntimeErrorCode::Internal, value.0, None)
    }
}

/// Composed server state: real capability source + real lifecycle.
#[derive(Clone)]
pub struct ServerState {
    capabilities: Arc<Box<dyn CapabilityListSource + Send + Sync>>,
    lifecycle: Arc<Mutex<RuntimeLifecycle>>,
}

/// Canonical health handler: always reports the healthy shape.
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(RuntimeHealth::healthy()))
}

/// Canonical readiness handler: ready only when the lifecycle is ready.
async fn readiness_handler(
    axum::extract::State(state): axum::extract::State<ServerState>,
) -> impl IntoResponse {
    let ready = state.lifecycle.lock().expect("lifecycle lock").is_ready();
    if ready {
        (StatusCode::OK, Json(RuntimeReadiness::ready()))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(RuntimeReadiness::not_ready()),
        )
    }
}

/// Canonical capabilities handler: non-empty list or fail closed.
async fn capabilities_handler(
    axum::extract::State(state): axum::extract::State<ServerState>,
) -> impl IntoResponse {
    match state.capabilities.list() {
        Ok(keys) => {
            let list = CapabilityList::new(keys);
            if list.is_non_empty() {
                (StatusCode::OK, Json(list))
            } else {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(CapabilityList::new(vec![])),
                )
            }
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(CapabilityList::new(vec![])),
        ),
    }
}

/// The runnable control-plane server.
///
/// Owns the axum router over the canonical endpoints. The composition
/// root wires real sources and a real lifecycle; the server never
/// fabricates data.
#[derive(Clone)]
pub struct ControlPlaneServer {
    config: ControlPlaneConfig,
    state: ServerState,
}

impl ControlPlaneServer {
    pub fn new(
        config: ControlPlaneConfig,
        capabilities: Box<dyn CapabilityListSource + Send + Sync>,
    ) -> Self {
        Self {
            config,
            state: ServerState {
                capabilities: Arc::new(capabilities),
                lifecycle: Arc::new(Mutex::new(RuntimeLifecycle::new())),
            },
        }
    }

    pub fn config(&self) -> &ControlPlaneConfig {
        &self.config
    }

    pub fn lifecycle(&self) -> &Arc<Mutex<RuntimeLifecycle>> {
        &self.state.lifecycle
    }

    /// Build the axum router for the canonical endpoints.
    pub fn router(&self) -> Router {
        Router::new()
            .route("/healthz", get(health_handler))
            .route("/readyz", get(readiness_handler))
            .route("/v1/capabilities", get(capabilities_handler))
            .with_state(self.state.clone())
    }

    /// Graceful startup: bind once and serve until shutdown signal.
    ///
    /// Returns the serve error or Ok after graceful shutdown. The
    /// listener is bound once (no probe-bind/drop/re-bind TOCTOU).
    pub async fn serve(
        &self,
        shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> Result<(), ControlPlaneServerError> {
        let listener = tokio::net::TcpListener::bind(&self.config.bind_address)
            .await
            .map_err(|e| ControlPlaneServerError(format!("bind: {e}")))?;
        {
            let mut lc = self
                .state
                .lifecycle
                .lock()
                .map_err(|_| ControlPlaneServerError("lifecycle lock".into()))?;
            lc.mark_ready().map_err(|e| ControlPlaneServerError(e.0))?;
        }
        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(|e| ControlPlaneServerError(format!("serve: {e}")))?;
        {
            let mut lc = self
                .state
                .lifecycle
                .lock()
                .map_err(|_| ControlPlaneServerError("lifecycle lock".into()))?;
            let _ = lc.begin_shutdown();
            let _ = lc.finish_shutdown();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::ConfiguredCapabilityList;

    fn test_server() -> ControlPlaneServer {
        let cfg = ControlPlaneConfig::new(
            "nexus.test",
            "127.0.0.1:18443",
            "018f0f6f-9c1e-7b6e-8000-000000000001",
            "core",
        )
        .unwrap();
        let caps: Box<dyn CapabilityListSource + Send + Sync> = Box::new(
            ConfiguredCapabilityList::new("core", vec!["health".into(), "capabilities".into()]),
        );
        ControlPlaneServer::new(cfg, caps)
    }

    #[test]
    fn ep044_unit_server_router_builds() {
        let server = test_server();
        let _router = server.router();
        assert_eq!(server.config().base_url(), "https://nexus.test");
    }

    #[test]
    fn ep044_unit_server_lifecycle_rejects_double_ready() {
        let server = test_server();
        {
            let mut lc = server.lifecycle().lock().unwrap();
            lc.mark_ready().unwrap();
            let err = lc.mark_ready().unwrap_err();
            assert!(err.0.contains("cannot mark ready"));
        }
    }
}
