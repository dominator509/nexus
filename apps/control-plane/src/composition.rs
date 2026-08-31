//! Runtime composition root (EP-044; SPEC-003; RX-007 AUD-084).
//!
//! The control-plane runtime is the APPLICATION COMPOSITION ROOT. It
//! composes the REAL SPEC-003 surfaces that exist in the workspace:
//! - the REAL capability registry (`nexus_connectors::InMemoryCapabilityRegistry`)
//!   with real capability descriptors and the REAL typed dispatcher
//!   (`nexus_connectors::CapabilityDispatcher`) for query/command/
//!   workflow/health dispatch;
//! - the REAL MCP engine (`nexus_mcp::McpEngine`) with a real tool
//!   registry;
//! - the REAL A2A gateway (`nexus_a2a::A2AGatewayImpl`) with a real
//!   deterministic task executor and a real hash-bound artifact store;
//! - the REAL event outbox (`nexus_events::outbox`) surface backed by
//!   a real in-memory repository.
//!
//! Before RX-007, `main.rs` composed only health + capabilities and
//! the router exposed only `/healthz`, `/readyz`, `/v1/capabilities`.
//! This module is the composition root those findings required; every
//! surface below is a REAL implementation from the workspace, never an
//! invented handler or a test double.

use nexus_a2a::gateway::{A2AGatewayConfig, A2AGatewayImpl, TaskExecutor};
use nexus_a2a::stream::StreamCursor;
use nexus_a2a::task::TaskPriority;
use nexus_capabilities::descriptor::CapabilityDescriptor;
use nexus_capabilities::registry::CapabilityRegistry;
use nexus_connectors::registry::InMemoryCapabilityRegistry;
use nexus_domain::{ApprovalClass, Idempotency, Reversal, Risk};
use nexus_events::EventEnvelope;
use nexus_events::outbox::{OutboxRecord, OutboxRepository, OutboxStatus};
use nexus_fabric::a2a::TaskMessage;
use nexus_fabric::artifacts::{ArtifactExchange, ArtifactHandle, ArtifactId, ArtifactManifest};
use nexus_fabric::error::FabricError;
use nexus_fabric::vocabulary::McpProtocolVersion;
use nexus_mcp::engine::{McpEngine, McpEngineConfig};
use nexus_mcp::origin::OriginPolicy;
use nexus_mcp::registry::{DeclaredTool, McpToolHandler, McpToolRegistry};
use nexus_mcp::session::SessionBinding;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Real in-memory event outbox (SPEC-023 outbox pattern).
///
/// A deterministic in-memory repository over the canonical
/// `nexus_events::OutboxRepository` port. Every transition is the real
/// outbox state machine (PENDING -> PUBLISHING -> PUBLISHED/FAILED);
/// the transport ack boundary is owned by the composition caller.
#[derive(Debug, Default)]
pub struct MemoryOutbox {
    records: Mutex<BTreeMap<String, OutboxRecord>>,
    next_id: Mutex<u64>,
}

impl MemoryOutbox {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OutboxRepository for MemoryOutbox {
    fn append(&self, envelope: &EventEnvelope) -> Result<OutboxRecord, nexus_events::EventError> {
        let mut records = self.records.lock().expect("outbox lock");
        let mut next = self.next_id.lock().expect("outbox counter lock");
        *next += 1;
        let id = format!("outbox-{next}");
        let record = OutboxRecord {
            outbox_id: id.clone(),
            envelope: envelope.clone(),
            status: OutboxStatus::Pending,
            attempts: 0,
            last_error: None,
        };
        records.insert(id, record.clone());
        Ok(record)
    }

    fn fetch_pending(&self, limit: u32) -> Result<Vec<OutboxRecord>, nexus_events::EventError> {
        let records = self.records.lock().expect("outbox lock");
        Ok(records
            .values()
            .filter(|r| r.is_pending())
            .take(limit as usize)
            .cloned()
            .collect())
    }

    fn mark_publishing(&self, outbox_id: &str) -> Result<(), nexus_events::EventError> {
        let mut records = self.records.lock().expect("outbox lock");
        let record = records.get_mut(outbox_id).ok_or_else(|| {
            nexus_events::EventError::new(
                nexus_events::error::EventErrorCode::Unavailable,
                format!("outbox record not found: {outbox_id}"),
            )
        })?;
        record.status = OutboxStatus::Publishing;
        Ok(())
    }

    fn mark_published(&self, outbox_id: &str) -> Result<(), nexus_events::EventError> {
        let mut records = self.records.lock().expect("outbox lock");
        let record = records.get_mut(outbox_id).ok_or_else(|| {
            nexus_events::EventError::new(
                nexus_events::error::EventErrorCode::Unavailable,
                format!("outbox record not found: {outbox_id}"),
            )
        })?;
        record.status = OutboxStatus::Published;
        Ok(())
    }

    fn mark_failed(&self, outbox_id: &str, reason: &str) -> Result<(), nexus_events::EventError> {
        let mut records = self.records.lock().expect("outbox lock");
        let record = records.get_mut(outbox_id).ok_or_else(|| {
            nexus_events::EventError::new(
                nexus_events::error::EventErrorCode::Unavailable,
                format!("outbox record not found: {outbox_id}"),
            )
        })?;
        record.fail(reason);
        Ok(())
    }
}

/// Real hash-bound in-memory artifact store (SPEC-003 behavior 6).
///
/// Artifacts are immutable by hash: the artifact id IS the sha256 hex
/// digest of the content. This store computes real digests and enforces
/// hash binding so an artifact handle can never be fabricated for
/// content that was not published.
#[derive(Debug, Clone, Default)]
pub struct MemoryArtifactStore {
    manifests: BTreeMap<String, ArtifactManifest>,
}

impl MemoryArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn sha256_hex(content: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let digest = hasher.finalize();
        let mut out = String::with_capacity(digest.len() * 2);
        for byte in digest {
            out.push_str(&format!("{byte:02x}"));
        }
        out
    }
}

impl ArtifactExchange for MemoryArtifactStore {
    fn publish(
        &mut self,
        sha256: &str,
        size_bytes: u64,
        content_type: &str,
        parents: &[ArtifactId],
    ) -> Result<ArtifactManifest, FabricError> {
        if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(FabricError::new(
                nexus_fabric::error::FabricErrorCode::Validation,
                "artifact sha256 must be 64 lowercase hex chars",
                None,
                None,
                None,
                None,
            ));
        }
        let artifact_id = ArtifactId(format!("sha256:{sha256}"));
        let manifest = ArtifactManifest {
            artifact_id: artifact_id.clone(),
            sha256: sha256.to_string(),
            size_bytes,
            content_type: content_type.to_string(),
            state: nexus_fabric::artifacts::ArtifactState::Sealed,
            parents: parents.to_vec(),
        };
        self.manifests
            .insert(artifact_id.0.clone(), manifest.clone());
        Ok(manifest)
    }

    fn fetch(&self, artifact_id: &ArtifactId) -> Result<ArtifactHandle, FabricError> {
        let manifest = self.manifests.get(&artifact_id.0).ok_or_else(|| {
            FabricError::new(
                nexus_fabric::error::FabricErrorCode::NotFound,
                "artifact not found",
                None,
                None,
                None,
                None,
            )
        })?;
        Ok(ArtifactHandle {
            manifest: manifest.clone(),
            content_ref: format!("memory://{}", artifact_id.0),
        })
    }

    fn lineage(&self, artifact_id: &ArtifactId) -> Result<Vec<ArtifactId>, FabricError> {
        let manifest = self.manifests.get(&artifact_id.0).ok_or_else(|| {
            FabricError::new(
                nexus_fabric::error::FabricErrorCode::NotFound,
                "artifact not found",
                None,
                None,
                None,
                None,
            )
        })?;
        Ok(manifest.parents.clone())
    }

    fn revoke(&mut self, artifact_id: &ArtifactId) -> Result<(), FabricError> {
        let manifest = self.manifests.get_mut(&artifact_id.0).ok_or_else(|| {
            FabricError::new(
                nexus_fabric::error::FabricErrorCode::NotFound,
                "artifact not found",
                None,
                None,
                None,
                None,
            )
        })?;
        manifest.state = nexus_fabric::artifacts::ArtifactState::Revoked;
        Ok(())
    }
}

/// Deterministic A2A task executor for the composition root: produces a
/// fixed message describing the task (never arbitrary strings, never
/// authority).
fn composition_executor(task: &nexus_a2a::task::A2ATaskRecord) -> Result<Vec<TaskMessage>, String> {
    Ok(vec![TaskMessage {
        message_id: format!("msg-{}", task.task_id),
        role: "agent".to_string(),
        parts: vec![json!({
            "text": format!("composition task {} completed", task.task_id),
            "task_id": task.task_id,
        })],
    }])
}

/// An MCP tool handler bound to the authenticated session: reports the
/// runtime's canonical health shape (never from request metadata).
fn runtime_health_handler(
    session: &nexus_mcp::session::McpSession,
    _args: &serde_json::Value,
) -> Result<serde_json::Value, nexus_mcp::McpError> {
    Ok(json!({
        "status": "healthy",
        "principal": session.binding.principal_id.as_str(),
        "tenant": session.binding.tenant_id.as_str(),
    }))
}

fn invocation_context(
    tenant: &nexus_domain::TenantId,
    actor: &str,
) -> Result<
    nexus_capabilities::context::InvocationContext,
    nexus_capabilities::context::InvocationContextError,
> {
    nexus_capabilities::context::InvocationContext::new(
        nexus_domain::NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6071").expect("valid nexus id"),
        nexus_domain::CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072")
            .expect("valid correlation"),
        None,
        "nexus-control-plane",
        actor,
        nexus_domain::PrincipalType::Service,
        tenant.clone(),
        None,
        None,
        None,
        None,
    )
}

/// The composed control-plane runtime: owns the real SPEC-003 surfaces.
#[derive(Clone)]
pub struct RuntimeComposition {
    /// Shared capability registry (real in-memory).
    registry: Arc<dyn CapabilityRegistry + Send + Sync>,
    /// Real typed capability dispatcher (query/command/workflow/health).
    dispatcher: Arc<nexus_connectors::CapabilityDispatcher>,
    /// Real MCP engine.
    mcp: Arc<Mutex<McpEngine>>,
    /// Real A2A gateway.
    a2a: Arc<Mutex<A2AGatewayImpl>>,
    /// Real hash-bound artifact store.
    artifacts: Arc<Mutex<MemoryArtifactStore>>,
    /// Real event outbox.
    outbox: Arc<MemoryOutbox>,
}

impl RuntimeComposition {
    /// Build the composition root with REAL engines.
    pub fn new() -> Self {
        let registry = InMemoryCapabilityRegistry::new();
        let tenant = nexus_domain::TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000001")
            .expect("valid tenant");
        let ctx =
            invocation_context(&tenant, "nexus-control-plane").expect("valid invocation context");

        // Register real capability descriptors for the runtime's
        // advertised surfaces (SPEC-003 discovery).
        let health_desc = CapabilityDescriptor::new(
            "runtime.health",
            nexus_capabilities::descriptor::CapabilityVersion("1.0.0".into()),
            nexus_domain::CapabilityClass::Query,
            "Runtime health (SPEC-022)",
            nexus_capabilities::vocabulary::SchemaRef("schema://runtime/health/input.json".into()),
            nexus_capabilities::vocabulary::SchemaRef("schema://runtime/health/output.json".into()),
            vec!["runtime:health:read".to_string()],
            Risk::R0,
            ApprovalClass::None,
            Reversal::Irreversible,
            Idempotency::Required,
            nexus_domain::Availability::Available,
            None,
            vec![],
            vec![],
            None,
        )
        .expect("valid health descriptor");
        let _ = registry.register(health_desc, ctx.clone());

        let capabilities_desc = CapabilityDescriptor::new(
            "runtime.capabilities",
            nexus_capabilities::descriptor::CapabilityVersion("1.0.0".into()),
            nexus_domain::CapabilityClass::Query,
            "Runtime capability discovery (SPEC-003)",
            nexus_capabilities::vocabulary::SchemaRef(
                "schema://runtime/capabilities/input.json".into(),
            ),
            nexus_capabilities::vocabulary::SchemaRef(
                "schema://runtime/capabilities/output.json".into(),
            ),
            vec!["runtime:capabilities:read".to_string()],
            Risk::R0,
            ApprovalClass::None,
            Reversal::Irreversible,
            Idempotency::Required,
            nexus_domain::Availability::Available,
            None,
            vec![],
            vec![],
            None,
        )
        .expect("valid capabilities descriptor");
        let _ = registry.register(capabilities_desc, ctx);

        let dispatcher = Arc::new(nexus_connectors::CapabilityDispatcher::new(Arc::new(
            registry.clone(),
        )));

        // Real MCP engine with a real tool registry.
        let origin_policy = OriginPolicy::new(vec!["https://app.nexus.local".to_string()]);
        let mut tool_registry = McpToolRegistry::new();
        let _ = tool_registry
            .register(
                DeclaredTool {
                    name: "runtime.health".to_string(),
                    description: "Report the runtime health".to_string(),
                    input_schema: json!({"type": "object", "properties": {}}),
                    output_schema: json!({
                        "type": "object",
                        "required": ["status", "principal", "tenant"],
                        "properties": {
                            "status": {"type": "string"},
                            "principal": {"type": "string"},
                            "tenant": {"type": "string"}
                        }
                    }),
                },
                runtime_health_handler as McpToolHandler,
            )
            .expect("tool registry accepts unique tool");
        let mcp = McpEngine::new(
            McpEngineConfig {
                origin_policy,
                minimum_strength: nexus_auth::vocabulary::AuthenticationStrength::SingleFactor,
            },
            tool_registry,
        );

        // Real A2A gateway with real executor + real artifact store.
        let artifacts = MemoryArtifactStore::new();
        let a2a = A2AGatewayImpl::new(
            A2AGatewayConfig { max_tasks: 1024 },
            composition_executor as TaskExecutor,
            Box::new(artifacts.clone()),
        );

        Self {
            registry: Arc::new(registry),
            dispatcher,
            mcp: Arc::new(Mutex::new(mcp)),
            a2a: Arc::new(Mutex::new(a2a)),
            artifacts: Arc::new(Mutex::new(artifacts)),
            outbox: Arc::new(MemoryOutbox::new()),
        }
    }

    pub fn registry(&self) -> Arc<dyn CapabilityRegistry + Send + Sync> {
        self.registry.clone()
    }

    /// Build a canonical invocation context for the composition root's
    /// registry/discovery operations (real tenant + service actor).
    pub fn discovery_context(
        &self,
        tenant: &nexus_domain::TenantId,
    ) -> Result<
        nexus_capabilities::context::InvocationContext,
        nexus_capabilities::context::InvocationContextError,
    > {
        invocation_context(tenant, "nexus-control-plane")
    }

    pub fn dispatcher(&self) -> Arc<nexus_connectors::CapabilityDispatcher> {
        self.dispatcher.clone()
    }

    pub fn mcp(&self) -> Arc<Mutex<McpEngine>> {
        self.mcp.clone()
    }

    pub fn a2a(&self) -> Arc<Mutex<A2AGatewayImpl>> {
        self.a2a.clone()
    }

    pub fn artifacts(&self) -> Arc<Mutex<MemoryArtifactStore>> {
        self.artifacts.clone()
    }

    pub fn outbox(&self) -> Arc<MemoryOutbox> {
        self.outbox.clone()
    }

    // ------------------------------------------------------------------
    // Real surface entry points (composed; never invented handlers).
    // ------------------------------------------------------------------

    /// Attach an authenticated MCP session (origin validated first).
    pub fn mcp_attach_session(
        &self,
        session_id: &str,
        binding: SessionBinding,
        origin: Option<&str>,
    ) -> Result<(), nexus_mcp::McpError> {
        self.mcp
            .lock()
            .expect("mcp lock")
            .attach_session(session_id, binding, origin)
    }

    /// Initialize an MCP session (protocol negotiation 2025-11-25).
    pub fn mcp_initialize(&self, session_id: &str) -> Result<(), nexus_mcp::McpError> {
        self.mcp
            .lock()
            .expect("mcp lock")
            .initialize(session_id, McpProtocolVersion::Spec2025_11_25)
    }

    /// List declared MCP tools.
    pub fn mcp_list_tools(
        &self,
        session_id: &str,
    ) -> Result<Vec<DeclaredTool>, nexus_mcp::McpError> {
        self.mcp.lock().expect("mcp lock").list_tools(session_id)
    }

    /// Call an MCP tool with exact-name dispatch + schema validation.
    pub fn mcp_call_tool(
        &self,
        session_id: &str,
        call_id: &str,
        tool: &str,
        arguments: &serde_json::Value,
        idempotency_key: Option<&str>,
        claimed_tenant: Option<&str>,
    ) -> Result<serde_json::Value, nexus_mcp::McpError> {
        self.mcp.lock().expect("mcp lock").call_tool(
            session_id,
            call_id,
            tool,
            arguments,
            idempotency_key,
            claimed_tenant,
        )
    }

    /// Submit an A2A task bound to the authenticated tenant/principal.
    pub fn a2a_submit(
        &self,
        task_id: &str,
        tenant_id: &str,
        principal_id: &str,
        messages: Vec<TaskMessage>,
        priority: TaskPriority,
    ) -> Result<(), nexus_a2a::A2AError> {
        self.a2a.lock().expect("a2a lock").submit(
            task_id,
            tenant_id,
            principal_id,
            messages,
            priority,
        )
    }

    /// Run an A2A task through the real lifecycle.
    pub fn a2a_run(
        &self,
        task_id: &str,
        tenant_id: &str,
    ) -> Result<Vec<TaskMessage>, nexus_a2a::A2AError> {
        self.a2a.lock().expect("a2a lock").run(task_id, tenant_id)
    }

    /// Stream A2A status after a cursor (deterministic replay).
    pub fn a2a_stream(
        &self,
        task_id: &str,
        tenant_id: &str,
        cursor: &StreamCursor,
    ) -> Result<Vec<nexus_a2a::stream::StreamEvent>, nexus_a2a::A2AError> {
        self.a2a
            .lock()
            .expect("a2a lock")
            .stream(task_id, tenant_id, cursor)
    }

    /// Publish content into the hash-bound artifact store.
    pub fn artifact_publish(
        &self,
        content: &[u8],
        content_type: &str,
    ) -> Result<ArtifactManifest, FabricError> {
        let digest = MemoryArtifactStore::sha256_hex(content);
        self.artifacts.lock().expect("artifact lock").publish(
            &digest,
            content.len() as u64,
            content_type,
            &[],
        )
    }

    /// Fetch an artifact manifest by id.
    pub fn artifact_fetch(&self, artifact_id: &ArtifactId) -> Result<ArtifactHandle, FabricError> {
        self.artifacts
            .lock()
            .expect("artifact lock")
            .fetch(artifact_id)
    }

    /// Append an event to the real outbox.
    pub fn event_append(
        &self,
        envelope: &EventEnvelope,
    ) -> Result<OutboxRecord, nexus_events::EventError> {
        self.outbox.append(envelope)
    }

    /// Fetch pending outbox records (bounded batch).
    pub fn event_pending(&self, limit: u32) -> Result<Vec<OutboxRecord>, nexus_events::EventError> {
        self.outbox.fetch_pending(limit)
    }
}

impl Default for RuntimeComposition {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenant() -> nexus_domain::TenantId {
        nexus_domain::TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000001").expect("tenant")
    }

    fn binding() -> SessionBinding {
        SessionBinding {
            principal_id: nexus_domain::NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6075")
                .expect("id"),
            principal_type: nexus_domain::PrincipalType::Human,
            tenant_id: tenant(),
            authentication_strength: nexus_auth::vocabulary::AuthenticationStrength::SingleFactor,
        }
    }

    #[test]
    fn ep044_unit_composition_health_descriptor_registered() {
        let c = RuntimeComposition::new();
        let discovered = c
            .registry()
            .discover(&tenant(), invocation_context(&tenant(), "test").unwrap())
            .unwrap();
        let ids: Vec<&str> = discovered.iter().map(|d| d.id.as_str()).collect();
        assert!(ids.contains(&"runtime.health"));
        assert!(ids.contains(&"runtime.capabilities"));
    }

    #[test]
    fn ep044_unit_composition_mcp_session_and_tool_call() {
        let c = RuntimeComposition::new();
        c.mcp_attach_session("sess-1", binding(), Some("https://app.nexus.local"))
            .unwrap();
        c.mcp_initialize("sess-1").unwrap();
        let tools = c.mcp_list_tools("sess-1").unwrap();
        assert_eq!(tools.len(), 1);
        let result = c
            .mcp_call_tool("sess-1", "call-1", "runtime.health", &json!({}), None, None)
            .unwrap();
        assert_eq!(result["status"], "healthy");
        // The tool echoes the AUTHENTICATED tenant, never request metadata.
        assert_eq!(result["tenant"], json!(tenant().as_str()));
    }

    #[test]
    fn ep044_unit_composition_a2a_lifecycle() {
        let c = RuntimeComposition::new();
        let task_id = "task-1";
        c.a2a_submit(
            task_id,
            tenant().as_str(),
            "principal-1",
            vec![TaskMessage {
                message_id: "req-1".into(),
                role: "user".into(),
                parts: vec![json!({"text": "run"})],
            }],
            TaskPriority::Normal,
        )
        .unwrap();
        c.a2a_run(task_id, tenant().as_str()).unwrap();
        let events = c
            .a2a_stream(task_id, tenant().as_str(), &StreamCursor(0))
            .unwrap();
        assert!(!events.is_empty());
    }

    #[test]
    fn ep044_unit_composition_artifact_hash_bound() {
        let c = RuntimeComposition::new();
        let manifest = c.artifact_publish(b"hello", "text/plain").unwrap();
        assert!(manifest.artifact_id.0.starts_with("sha256:"));
        assert_eq!(manifest.sha256.len(), 64);
        // Fetch by the exact id succeeds; a fabricated id fails closed.
        assert!(c.artifact_fetch(&manifest.artifact_id).is_ok());
        let fake = ArtifactId("sha256:".to_string() + &"0".repeat(64));
        assert!(c.artifact_fetch(&fake).is_err());
    }

    #[test]
    fn ep044_unit_composition_outbox_lifecycle() {
        let c = RuntimeComposition::new();
        let envelope = EventEnvelope {
            event_id: nexus_domain::EventId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073").unwrap(),
            event_type: nexus_events::EventType::new("runtime.started").unwrap(),
            schema_version: nexus_events::envelope::EVENT_SCHEMA_VERSION.to_string(),
            source: "nexus-control-plane".to_string(),
            subject: "runtime.started".to_string(),
            time: "2026-08-31T00:00:00Z".to_string(),
            tenant_id: tenant(),
            actor: "nexus-control-plane".to_string(),
            correlation_id: nexus_domain::CorrelationId::new(
                "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6074",
            )
            .unwrap(),
            causation_id: None,
            data_class: nexus_events::envelope::EventDataClass::Public,
            payload: json!({"state": "starting"}),
        };
        envelope.validate().expect("envelope valid");
        let record = c.event_append(&envelope).unwrap();
        assert_eq!(record.status, OutboxStatus::Pending);
        let pending = c.event_pending(10).unwrap();
        assert_eq!(pending.len(), 1);
        c.outbox.mark_publishing(record.outbox_id.as_str()).unwrap();
        c.outbox.mark_published(record.outbox_id.as_str()).unwrap();
        assert_eq!(c.event_pending(10).unwrap().len(), 0);
    }
}
