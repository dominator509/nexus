//! Real sidecar HTTP server (directive B/I).
//!
//! The server binds 127.0.0.1 only (loopback; never 0.0.0.0), serves
//! the canonical sidecar REST surface over real HTTP, enforces the
//! strict envelope/limits/version/tenant/dispatch/credential checks
//! before provider invocation, emits redacted telemetry, and shuts
//! down cleanly on signal.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};

use nexus_connector_sdk::vocabulary::SidecarTransport;

use crate::credential::CredentialScope;
use crate::dispatch::CapabilityClassTable;
use crate::envelope::{RequestEnvelope, RequestOperation};
use crate::error::{SidecarError, SidecarErrorKind};
use crate::limits::Limits;
use crate::provider::ProviderClient;
use crate::telemetry::{TelemetryEntry, TelemetryEvent, TelemetrySink, fingerprint};
use crate::tenant::TenantBinding;
use crate::version::{PROTOCOL_VERSION, reconcile_protocol_versions};
use crate::webhook::{WebhookIngress, WebhookVerdict};

/// Sidecar process configuration.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// Bind address (must be loopback; enforced by the binary).
    pub bind: SocketAddr,
    /// Resource limits.
    pub limits: Limits,
    /// Bound tenant (directive F).
    pub tenant: TenantBinding,
    /// Connector/capability dispatch table (directive G).
    pub dispatch: CapabilityClassTable,
    /// Credential reference scope (directive N).
    pub credentials: CredentialScope,
    /// Provider base URL.
    pub provider_base_url: String,
    /// Webhook ingress (directive P/Q). None disables webhook ingress
    /// (still fails closed with a typed rejection). Shared mutable
    /// replay state: the dedupe set must persist across requests.
    pub webhook: Option<Arc<std::sync::Mutex<WebhookIngress>>>,
    /// Owned legacy poller boundary (directive R/S). None makes POLL
    /// fail closed (no poller provisioned).
    pub poller: Option<crate::poller::PollSource>,
    /// Concurrency semaphore (directive T). Capacity is the configured
    /// concurrency bound; a dispatch waits for a permit with a bounded
    /// timeout, then fails closed with a typed overload.
    pub concurrency: Arc<tokio::sync::Semaphore>,
}

/// Shared server state for spawned request handlers.
struct ServerShared {
    config: SidecarConfig,
    provider: ProviderClient,
    sink: TelemetrySink,
}

impl ServerShared {
    fn connector_id(&self) -> String {
        self.config
            .dispatch
            .connectors()
            .first()
            .cloned()
            .unwrap_or_default()
    }
}

/// Sidecar server handle.
pub struct SidecarServer {
    shared: Arc<ServerShared>,
}

impl SidecarServer {
    /// Construct the server (fails closed on invalid config).
    pub fn new(config: SidecarConfig) -> Result<Self, SidecarError> {
        let provider = ProviderClient::new(&config.provider_base_url, config.limits)?;
        let sink = TelemetrySink::stderr();
        Ok(Self {
            shared: Arc::new(ServerShared {
                config,
                provider,
                sink,
            }),
        })
    }

    /// The connector id this sidecar serves.
    pub fn connector_id(&self) -> String {
        self.shared.connector_id()
    }

    /// The configured bind address.
    pub fn config_bind(&self) -> SocketAddr {
        self.shared.config.bind
    }

    /// Replace the bind address (used by the binary to pin the
    /// ephemeral port after probing).
    pub fn with_bind(mut self, addr: SocketAddr) -> Self {
        Arc::get_mut(&mut self.shared)
            .expect("server must not be shared when rebinding")
            .config
            .bind = addr;
        self
    }

    /// The sink (for pre/post server lifecycle events).
    pub fn sink(&self) -> TelemetrySink {
        self.shared.sink.clone()
    }

    /// Start the loopback listener and serve until shutdown.
    pub async fn serve(self) -> Result<(), SidecarError> {
        let listener = tokio::net::TcpListener::bind(self.shared.config.bind)
            .await
            .map_err(|e| {
                SidecarError::new(
                    SidecarErrorKind::Internal,
                    format!("bind failed: {e}"),
                    None,
                    None,
                    None,
                )
            })?;
        let local = listener.local_addr().map_err(|e| {
            SidecarError::new(
                SidecarErrorKind::Internal,
                format!("local addr failed: {e}"),
                None,
                None,
                None,
            )
        })?;
        let connector_id = self.connector_id();
        self.shared.sink.emit(&TelemetryEntry {
            event: TelemetryEvent::SidecarStarted,
            connector_fingerprint: Some(fingerprint(&connector_id)),
            capability_id: None,
            class: None,
            transport: Some("REST".to_string()),
            result_class: None,
            latency_ms: None,
            correlation_id: None,
            tenant_fingerprint: Some(fingerprint(self.shared.config.tenant.tenant())),
            detail: Some(format!(
                "bind={local} provider={}",
                self.shared.provider.base()
            )),
        });

        loop {
            let (stream, _peer) = match listener.accept().await {
                Ok(pair) => pair,
                Err(_) => continue,
            };
            let shared = self.shared.clone();
            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    let shared = shared.clone();
                    async move { handle_request(req, shared).await }
                });
                let io = hyper_util::rt::TokioIo::new(stream);
                let conn = hyper::server::conn::http1::Builder::new().serve_connection(io, service);
                let _ = conn.await;
            });
        }
    }
}

/// Dispatch a single HTTP request (directive I/C/D/F/G/T).
async fn handle_request(
    req: Request<Incoming>,
    shared: Arc<ServerShared>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let transport = transport_for_path(&path);

    // Method/path hardening (directive I): exact canonical surface
    // only; no debug/admin endpoints; no filesystem mapping.
    if !method_ok(&method, &path) {
        let err = SidecarError::new(
            SidecarErrorKind::Validation,
            "method not allowed",
            None,
            None,
            Some(path.clone()),
        );
        shared.sink.emit(&rejected(&err, None, &path, transport));
        return Ok(json_error(StatusCode::METHOD_NOT_ALLOWED, &err));
    }
    if !route_ok(&path) {
        let err = SidecarError::new(
            SidecarErrorKind::Validation,
            "unknown path",
            None,
            None,
            Some(path.clone()),
        );
        shared.sink.emit(&rejected(&err, None, &path, transport));
        return Ok(json_error(StatusCode::NOT_FOUND, &err));
    }

    // Loopback-only health probe (directive B/M): no body parsing.
    if path == "/v1/fixture/healthz" {
        return Ok(json_success(serde_json::json!({ "status": "ok" })));
    }

    // Protocol version from header (directive H).
    let header_version = req
        .headers()
        .get("x-nexus-protocol-version")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    // Content-Type hardening (directive D): the canonical surface
    // requires JSON bodies; anything else is rejected before reading.
    // GET health probes carry no body and skip the check.
    if method == Method::POST {
        let content_type = req
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type
            .to_ascii_lowercase()
            .starts_with("application/json")
        {
            let err = SidecarError::new(
                SidecarErrorKind::Validation,
                "unsupported content type (application/json required)",
                None,
                None,
                Some(path.clone()),
            );
            shared.sink.emit(&rejected(&err, None, &path, transport));
            return Ok(json_error(StatusCode::UNSUPPORTED_MEDIA_TYPE, &err));
        }
    }

    // Body limit + read timeout (directive D/U).
    let body_bytes = match read_body(req.into_body(), shared.config.limits).await {
        Ok(b) => b,
        Err(err) => {
            shared.sink.emit(&rejected(&err, None, &path, transport));
            return Ok(json_error(
                StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
                &err,
            ));
        }
    };

    // Strict envelope parse (directive C/E).
    let envelope = match RequestEnvelope::parse(&body_bytes) {
        Ok(e) => e,
        Err(err) => {
            shared.sink.emit(&rejected(&err, None, &path, transport));
            return Ok(json_error(
                StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
                &err,
            ));
        }
    };

    let correlation = envelope.correlation_id.clone();
    let tenant_fp = fingerprint(&envelope.tenant_id);
    let connector_fp = fingerprint(&envelope.connector_id);

    // Reconcile header + envelope protocol versions (directive H).
    if let Err(err) =
        reconcile_protocol_versions(header_version.as_deref(), Some(&envelope.protocol_version))
    {
        shared
            .sink
            .emit(&rejected(&err, Some(&envelope), &path, transport));
        return Ok(json_error(
            StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
            &err,
        ));
    }

    // Tenant binding (directive F).
    if let Err(err) = shared
        .config
        .tenant
        .enforce(&envelope.tenant_id, Some(&correlation))
    {
        shared
            .sink
            .emit(&rejected(&err, Some(&envelope), &path, transport));
        return Ok(json_error(
            StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
            &err,
        ));
    }

    // Connector/capability dispatch (directive G).
    if let Err(err) = shared.config.dispatch.enforce(
        &envelope.connector_id,
        &envelope.capability_id,
        class_for_operation(&envelope),
        Some(&correlation),
    ) {
        shared
            .sink
            .emit(&rejected(&err, Some(&envelope), &path, transport));
        return Ok(json_error(
            StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
            &err,
        ));
    }

    // Concurrency bound (directive T): acquire with a bounded wait.
    let permit = match tokio::time::timeout(
        std::time::Duration::from_millis(50),
        shared.config.concurrency.clone().acquire_owned(),
    )
    .await
    {
        Ok(Ok(permit)) => permit,
        _ => {
            let err = SidecarError::new(
                SidecarErrorKind::Overloaded,
                "sidecar concurrency limit reached",
                Some(correlation.clone()),
                Some(envelope.tenant_id.clone()),
                Some(envelope.capability_id.clone()),
            );
            shared
                .sink
                .emit(&rejected(&err, Some(&envelope), &path, transport));
            return Ok(json_error(
                StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
                &err,
            ));
        }
    };

    shared.sink.emit(&TelemetryEntry {
        event: TelemetryEvent::RequestAccepted,
        connector_fingerprint: Some(connector_fp.clone()),
        capability_id: Some(envelope.capability_id.clone()),
        class: Some(envelope.operation.as_str().to_string()),
        transport: Some(transport.to_string()),
        result_class: None,
        latency_ms: None,
        correlation_id: Some(correlation.clone()),
        tenant_fingerprint: Some(tenant_fp.clone()),
        detail: None,
    });

    let start = Instant::now();
    let result = dispatch(&shared, &envelope).await;
    let latency = start.elapsed().as_millis() as u64;
    drop(permit);

    match result {
        Ok(value) => {
            shared.sink.emit(&TelemetryEntry {
                event: TelemetryEvent::DispatchCompleted,
                connector_fingerprint: Some(connector_fp),
                capability_id: Some(envelope.capability_id.clone()),
                class: Some(envelope.operation.as_str().to_string()),
                transport: Some(transport.to_string()),
                result_class: Some("ALLOW".to_string()),
                latency_ms: Some(latency),
                correlation_id: Some(correlation),
                tenant_fingerprint: Some(tenant_fp),
                detail: None,
            });
            Ok(json_success(value))
        }
        Err(err) => {
            shared.sink.emit(&TelemetryEntry {
                event: telemetry_event_for(&err),
                connector_fingerprint: Some(connector_fp),
                capability_id: Some(envelope.capability_id.clone()),
                class: Some(envelope.operation.as_str().to_string()),
                transport: Some(transport.to_string()),
                result_class: Some(err.wire_code().as_str().to_string()),
                latency_ms: Some(latency),
                correlation_id: Some(correlation),
                tenant_fingerprint: Some(tenant_fp),
                detail: Some(err.message.clone()),
            });
            Ok(json_error(
                StatusCode::from_u16(err.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
                &err,
            ))
        }
    }
}

/// Execute the validated request against the provider (directive J).
async fn dispatch(
    shared: &Arc<ServerShared>,
    envelope: &RequestEnvelope,
) -> Result<serde_json::Value, SidecarError> {
    // Webhook ingress (directive P/Q): handled by the sidecar, never
    // forwarded as a command/workflow capability.
    if envelope.operation == RequestOperation::Webhook {
        return handle_webhook(shared, envelope).await;
    }

    // Owned poller boundary (directive R/S): the sidecar reads the
    // real JSONL source and persists the validated checkpoint.
    if envelope.operation == RequestOperation::Poll {
        let Some(poller) = &shared.config.poller else {
            return Err(SidecarError::new(
                SidecarErrorKind::Unavailable,
                "poller is not provisioned",
                Some(envelope.correlation_id.clone()),
                Some(envelope.tenant_id.clone()),
                Some(envelope.capability_id.clone()),
            ));
        };
        let mut poller = poller.clone();
        match poller.poll() {
            Ok(result) => {
                return Ok(serde_json::json!({
                    "capability_id": envelope.capability_id,
                    "events": result.events,
                    "next_cursor": result.next_cursor,
                    "rejected_records": result.rejected_records,
                }));
            }
            Err(err) => {
                return Err(err);
            }
        }
    }

    // Credential scope check (directive N): if the input carries a
    // credential reference, enforce scope before provider invocation.
    if let Some(reference) = credential_reference(&envelope.input)
        && let Err(err) = shared.config.credentials.enforce(
            &envelope.connector_id,
            reference,
            Some(&envelope.correlation_id),
        )
    {
        shared.sink.emit(&TelemetryEntry {
            event: TelemetryEvent::CredentialBrokerDenied,
            connector_fingerprint: Some(fingerprint(&envelope.connector_id)),
            capability_id: Some(envelope.capability_id.clone()),
            class: Some(envelope.operation.as_str().to_string()),
            transport: Some(envelope.transport.as_str().to_string()),
            result_class: Some(err.wire_code().as_str().to_string()),
            latency_ms: None,
            correlation_id: Some(envelope.correlation_id.clone()),
            tenant_fingerprint: Some(fingerprint(&envelope.tenant_id)),
            detail: Some("credential reference denied by scope".to_string()),
        });
        return Err(err);
    }

    // Provider body: canonical wire payload (M3 transport contract).
    let body = serde_json::json!({
        "context": {
            "request_id": envelope.request_id,
            "correlation_id": envelope.correlation_id,
            "origin_system": "nexus-sidecar",
            "external_actor_id": "sidecar:system",
            "external_actor_type": "SYSTEM",
            "tenant_id": envelope.tenant_id,
        },
        "capability_id": envelope.capability_id,
        "input": envelope.input,
        "idempotency_key": envelope.idempotency_key,
    });

    let path = endpoint_for(envelope.operation);
    match shared
        .provider
        .dispatch(
            path,
            body,
            Some(&envelope.correlation_id),
            &shared.config.limits,
        )
        .await
    {
        Ok(value) => Ok(value),
        Err(provider_err) => Err(provider_err.into_sidecar(
            Some(envelope.correlation_id.clone()),
            Some(envelope.capability_id.clone()),
        )),
    }
}

/// Webhook ingress handler (directive P/Q).
///
/// Signature verification is the sidecar's own real HMAC check;
/// unknown event types are preserved per contract and never mapped to
/// an executable capability id.
async fn handle_webhook(
    shared: &Arc<ServerShared>,
    envelope: &RequestEnvelope,
) -> Result<serde_json::Value, SidecarError> {
    let Some(ingress) = &shared.config.webhook else {
        return Err(SidecarError::new(
            SidecarErrorKind::WebhookRejected,
            "webhook ingress is not configured",
            Some(envelope.correlation_id.clone()),
            Some(envelope.tenant_id.clone()),
            Some(envelope.capability_id.clone()),
        ));
    };

    let signature = envelope.input.get("signature").and_then(|v| v.as_str());
    let key_fingerprint = envelope
        .input
        .get("key_fingerprint")
        .and_then(|v| v.as_str());
    let provider_event_id = envelope
        .input
        .get("provider_event_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Canonical signed bytes: the provider event fields ONLY, with
    // signature metadata excluded (the signature cannot sign itself).
    let mut canonical = envelope.input.clone();
    if let Some(obj) = canonical.as_object_mut() {
        obj.remove("signature");
        obj.remove("key_fingerprint");
    }
    let payload = serde_json::to_vec(&canonical).unwrap_or_default();

    // Lock the shared ingress so replay dedupe state persists across
    // requests (directive P.4: exact locked dedupe behavior).
    let mut guard = ingress.lock().map_err(|_| {
        SidecarError::new(
            SidecarErrorKind::WebhookRejected,
            "webhook ingress lock poisoned",
            Some(envelope.correlation_id.clone()),
            Some(envelope.tenant_id.clone()),
            Some(envelope.capability_id.clone()),
        )
    })?;
    let verdict = guard.verify(signature, key_fingerprint, provider_event_id, &payload);
    drop(guard);

    match verdict {
        WebhookVerdict::Accepted => {
            // Unknown event types are preserved (directive Q); the
            // event is NEVER dispatched as a command/workflow.
            let event_type = envelope
                .input
                .get("provider_event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("webhook.received");
            let event = serde_json::json!({
                "event_id": provider_event_id,
                "event_type": event_type,
                "version": "1",
                "correlation_id": envelope.correlation_id,
                "payload": envelope.input.get("raw_payload").cloned().unwrap_or(serde_json::Value::Null),
            });
            Ok(serde_json::json!({
                "event": event,
                "verification": "VALID",
                "executable": false,
            }))
        }
        WebhookVerdict::InvalidSignature => Err(SidecarError::new(
            SidecarErrorKind::WebhookRejected,
            "webhook signature invalid",
            Some(envelope.correlation_id.clone()),
            Some(envelope.tenant_id.clone()),
            Some(envelope.capability_id.clone()),
        )),
        WebhookVerdict::MissingSignature => Err(SidecarError::new(
            SidecarErrorKind::WebhookRejected,
            "webhook signature missing",
            Some(envelope.correlation_id.clone()),
            Some(envelope.tenant_id.clone()),
            Some(envelope.capability_id.clone()),
        )),
        WebhookVerdict::Replay => Err(SidecarError::new(
            SidecarErrorKind::WebhookRejected,
            "webhook replay detected",
            Some(envelope.correlation_id.clone()),
            Some(envelope.tenant_id.clone()),
            Some(envelope.capability_id.clone()),
        )),
        WebhookVerdict::BindingMismatch => Err(SidecarError::new(
            SidecarErrorKind::WebhookRejected,
            "webhook tenant/connector binding mismatch",
            Some(envelope.correlation_id.clone()),
            Some(envelope.tenant_id.clone()),
            Some(envelope.capability_id.clone()),
        )),
    }
}

/// Extract a credential reference from canonical input (directive N).
fn credential_reference(input: &serde_json::Value) -> Option<&str> {
    input.get("credential_reference").and_then(|v| v.as_str())
}

/// Endpoint mapping for an operation (canonical surface).
fn endpoint_for(op: RequestOperation) -> &'static str {
    match op {
        RequestOperation::Discover => "/v1/discover",
        RequestOperation::Query => "/v1/query",
        RequestOperation::Command => "/v1/command",
        RequestOperation::Workflow => "/v1/workflow",
        RequestOperation::Health => "/v1/health",
        RequestOperation::Changefeed => "/v1/changefeed",
        RequestOperation::Poll => "/v1/poll",
        RequestOperation::Webhook => "/v1/webhook/normalize",
    }
}

/// Map an operation to the canonical capability class it exercises.
fn class_for_operation(envelope: &RequestEnvelope) -> nexus_domain::vocabulary::CapabilityClass {
    use nexus_domain::vocabulary::CapabilityClass;
    match envelope.operation {
        RequestOperation::Query | RequestOperation::Health | RequestOperation::Changefeed => {
            CapabilityClass::Query
        }
        RequestOperation::Command => CapabilityClass::Command,
        RequestOperation::Workflow => CapabilityClass::Workflow,
        // Boundary operations (discover/poll/webhook) validate against
        // the table's declared class only when the table knows them;
        // they carry no execution class.
        RequestOperation::Discover | RequestOperation::Poll | RequestOperation::Webhook => {
            CapabilityClass::Query
        }
    }
}

/// Method hardening (directive I): POST for the canonical surface,
/// GET only for the health probe.
fn method_ok(method: &Method, path: &str) -> bool {
    if path == "/v1/fixture/healthz" {
        return *method == Method::GET;
    }
    *method == Method::POST
}

/// Path hardening (directive I): exact canonical routes only; no
/// debug/admin paths; no path traversal interpretation.
fn route_ok(path: &str) -> bool {
    matches!(
        path,
        "/v1/discover"
            | "/v1/query"
            | "/v1/command"
            | "/v1/workflow"
            | "/v1/health"
            | "/v1/changefeed"
            | "/v1/poll"
            | "/v1/webhook/normalize"
            | "/v1/fixture/healthz"
    )
}

/// Canonical transport for a path (telemetry).
fn transport_for_path(path: &str) -> &'static str {
    if path == "/v1/webhook/normalize" {
        SidecarTransport::Webhook.as_str()
    } else {
        SidecarTransport::Rest.as_str()
    }
}

/// Read a request body with the bounded size + read timeout
/// (directive D/U). Truncated, malformed, and oversized bodies fail
/// closed.
async fn read_body(body: Incoming, limits: Limits) -> Result<Vec<u8>, SidecarError> {
    let limited = http_body_util::Limited::new(body, limits.max_request_bytes as usize);
    let collected = tokio::time::timeout(limits.read_timeout, limited.collect()).await;
    match collected {
        Err(_) => Err(SidecarError::new(
            SidecarErrorKind::Timeout,
            "request body read timed out",
            None,
            None,
            None,
        )),
        Ok(Err(e)) => Err(SidecarError::new(
            SidecarErrorKind::PayloadTooLarge,
            format!("request body rejected: {e}"),
            None,
            None,
            None,
        )),
        Ok(Ok(frame)) => Ok(frame.to_bytes().to_vec()),
    }
}

/// Build a JSON success response.
fn json_success(value: serde_json::Value) -> Response<Full<Bytes>> {
    let body = value.to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .header("x-nexus-protocol-version", PROTOCOL_VERSION)
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Build a canonical JSON error envelope (directive X).
fn json_error(status: StatusCode, err: &SidecarError) -> Response<Full<Bytes>> {
    let value = serde_json::to_value(err.to_sdk_error()).unwrap_or_else(
        |_| serde_json::json!({ "code": "INTERNAL", "message": "error serialization failed" }),
    );
    let body = value.to_string();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("x-nexus-protocol-version", PROTOCOL_VERSION)
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// A redacted rejection telemetry entry.
fn rejected(
    err: &SidecarError,
    envelope: Option<&RequestEnvelope>,
    _path: &str,
    transport: &str,
) -> TelemetryEntry {
    TelemetryEntry {
        event: telemetry_event_for(err),
        connector_fingerprint: envelope.map(|e| fingerprint(&e.connector_id)),
        capability_id: envelope.map(|e| e.capability_id.clone()),
        class: envelope.map(|e| e.operation.as_str().to_string()),
        transport: Some(transport.to_string()),
        result_class: Some(err.wire_code().as_str().to_string()),
        latency_ms: None,
        correlation_id: envelope.map(|e| e.correlation_id.clone()),
        tenant_fingerprint: envelope.map(|e| fingerprint(&e.tenant_id)),
        detail: Some(err.message.clone()),
    }
}

/// Telemetry event class for a typed failure.
fn telemetry_event_for(err: &SidecarError) -> TelemetryEvent {
    match err.kind {
        SidecarErrorKind::Timeout => TelemetryEvent::ProviderTimeout,
        SidecarErrorKind::ProviderError => TelemetryEvent::ProviderMalformedResponse,
        SidecarErrorKind::CredentialDenied => TelemetryEvent::CredentialBrokerDenied,
        SidecarErrorKind::WebhookRejected => TelemetryEvent::WebhookRejected,
        SidecarErrorKind::PollerCorrupt => TelemetryEvent::PollerRejected,
        _ => TelemetryEvent::RequestRejected,
    }
}
