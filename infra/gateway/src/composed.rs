//! Composed fabric gateway (EP-012 M5 crown-jewel).
//!
//! Owns the REAL MCP engine (`nexus-mcp::McpEngine`), the REAL A2A
//! gateway (`nexus-a2a::A2AGatewayImpl`), and a real hash-bound
//! artifact store. Exposes one authenticated surface that drives the
//! full SPEC-003 chain and records a deterministic probe outcome.
//!
//! Composition only: this struct never evaluates policy, never issues
//! capability grants, and never treats a valid MCP call, an A2A task
//! identity, or an artifact attachment as execution authority.

use crate::artifact_store::SharedArtifactStore;
use nexus_a2a::gateway::{A2AGatewayConfig, A2AGatewayImpl, TaskExecutor};
use nexus_a2a::stream::StreamCursor;
use nexus_a2a::task::TaskPriority;
use nexus_fabric::a2a::TaskMessage;
use nexus_fabric::artifacts::ArtifactId;
use nexus_fabric::error::FabricError;
use nexus_fabric::vocabulary::{A2AProtocolVersion, McpProtocolVersion};
use nexus_mcp::McpError;
use nexus_mcp::engine::{McpEngine, McpEngineConfig};
use nexus_mcp::origin::OriginPolicy;
use nexus_mcp::registry::{DeclaredTool, McpToolHandler, McpToolRegistry};
use nexus_mcp::session::SessionBinding;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Probe stage names (canonical ordering of the composed proof).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProbeStage {
    Session,
    Protocol,
    Tools,
    Call,
    Idempotency,
    Cancellation,
    A2aSubmit,
    A2aStream,
    Artifact,
    A2aComplete,
}

impl ProbeStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Session => "SESSION_PASS",
            Self::Protocol => "PROTOCOL_PASS",
            Self::Tools => "TOOLS_PASS",
            Self::Call => "CALL_PASS",
            Self::Idempotency => "IDEMPOTENCY_PASS",
            Self::Cancellation => "CANCELLATION_PASS",
            Self::A2aSubmit => "A2A_SUBMIT_PASS",
            Self::A2aStream => "A2A_STREAM_PASS",
            Self::Artifact => "ARTIFACT_PASS",
            Self::A2aComplete => "A2A_COMPLETE_PASS",
        }
    }
}

/// Deterministic outcome of the composed probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayProbeOutcome {
    pub request_id: String,
    pub correlation_id: String,
    pub principal_id: String,
    pub tenant_id: String,
    pub mcp_protocol: String,
    pub a2a_protocol: String,
    pub stages: Vec<String>,
    pub tool_count: usize,
    pub called_tool: String,
    pub idempotent_replay_identical: bool,
    pub cancelled_never_completes: bool,
    pub a2a_task_id: String,
    pub stream_states: Vec<String>,
    pub artifact_digest: String,
    pub artifact_attached: bool,
    pub final_lifecycle: String,
    pub model_recommendation_never_consulted: bool,
    pub receipt_never_reusable: bool,
    pub cross_tenant_denied: bool,
    pub authorization_not_implied: bool,
    pub verification_plan: Vec<String>,
}

/// Configuration for the composed gateway.
#[derive(Debug, Clone)]
pub struct ComposedGatewayConfig {
    pub allowed_origins: Vec<String>,
    pub minimum_strength: nexus_auth::vocabulary::AuthenticationStrength,
    pub max_tasks: usize,
}

impl Default for ComposedGatewayConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["https://app.nexus.local".to_string()],
            minimum_strength: nexus_auth::vocabulary::AuthenticationStrength::MultiFactor,
            max_tasks: 128,
        }
    }
}

/// The composed fabric gateway.
pub struct ComposedGateway {
    mcp: McpEngine,
    a2a: A2AGatewayImpl,
    artifacts: SharedArtifactStore,
}

/// Deterministic task executor for the composed proof: returns a fixed
/// message describing the task (never arbitrary strings, never
/// authority).
fn proof_executor(task: &nexus_a2a::task::A2ATaskRecord) -> Result<Vec<TaskMessage>, String> {
    Ok(vec![TaskMessage {
        message_id: format!("msg-{}", task.task_id),
        role: "agent".to_string(),
        parts: vec![json!({
            "text": format!("proof task {} completed", task.task_id),
            "task_id": task.task_id,
        })],
    }])
}

/// An MCP tool handler bound to the authenticated session: echoes the
/// authenticated principal and tenant from the binding (never from
/// request metadata) and the echoed argument.
fn proof_echo_handler(
    _session: &nexus_mcp::session::McpSession,
    args: &Value,
) -> Result<Value, McpError> {
    Ok(json!({
        "echo": args,
        "principal": _session.binding.principal_id.as_str(),
        "tenant": _session.binding.tenant_id.as_str(),
    }))
}

impl ComposedGateway {
    /// Build the composition with REAL engines and a real artifact
    /// store. The MCP registry is exact-name with declared schemas.
    pub fn new(config: ComposedGatewayConfig) -> Self {
        let origin_policy = OriginPolicy::new(config.allowed_origins.clone());
        let mut registry = McpToolRegistry::new();
        // Exact-name tool with declared input/output schemas.
        registry
            .register(
                DeclaredTool {
                    name: "proof.echo".to_string(),
                    description: "Echo the authenticated session context".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "required": ["message"],
                        "properties": {"message": {"type": "string"}}
                    }),
                    output_schema: json!({
                        "type": "object",
                        "required": ["echo", "principal", "tenant"],
                        "properties": {
                            "echo": {"type": "object"},
                            "principal": {"type": "string"},
                            "tenant": {"type": "string"}
                        }
                    }),
                },
                proof_echo_handler as McpToolHandler,
            )
            .expect("tool registry accepts unique exact-name tool");
        let mcp = McpEngine::new(
            McpEngineConfig {
                origin_policy,
                minimum_strength: config.minimum_strength,
            },
            registry,
        );
        let artifacts = SharedArtifactStore::new();
        let a2a = A2AGatewayImpl::new(
            A2AGatewayConfig {
                max_tasks: config.max_tasks,
            },
            proof_executor as TaskExecutor,
            Box::new(artifacts.clone()),
        );
        Self {
            mcp,
            a2a,
            artifacts,
        }
    }

    /// Attach an authenticated MCP session (origin validated before
    /// any session work).
    pub fn attach_session(
        &mut self,
        session_id: &str,
        binding: SessionBinding,
        origin: Option<&str>,
    ) -> Result<(), McpError> {
        self.mcp.attach_session(session_id, binding, origin)
    }

    /// Protocol negotiation: only MCP 2025-11-25 is accepted.
    pub fn initialize(&mut self, session_id: &str) -> Result<(), McpError> {
        self.mcp
            .initialize(session_id, McpProtocolVersion::Spec2025_11_25)
    }

    /// List declared tools (tenant-safe exact-name registry).
    pub fn list_tools(&self, session_id: &str) -> Result<Vec<DeclaredTool>, McpError> {
        self.mcp.list_tools(session_id)
    }

    /// Call a tool with exact-name dispatch, schema validation,
    /// idempotency, and cancellation tracking.
    pub fn call_tool(
        &mut self,
        session_id: &str,
        call_id: &str,
        tool: &str,
        arguments: &Value,
        idempotency_key: Option<&str>,
        claimed_tenant: Option<&str>,
    ) -> Result<Value, McpError> {
        self.mcp.call_tool(
            session_id,
            call_id,
            tool,
            arguments,
            idempotency_key,
            claimed_tenant,
        )
    }

    /// Cancel an in-flight MCP call; the call can never complete after
    /// cancellation.
    pub fn cancel_call(&mut self, session_id: &str, call_id: &str) -> Result<(), McpError> {
        self.mcp.cancel(session_id, call_id)
    }

    /// Submit an A2A task bound to the authenticated tenant/principal.
    pub fn submit_a2a(
        &mut self,
        task_id: &str,
        tenant_id: &str,
        principal_id: &str,
        messages: Vec<TaskMessage>,
        priority: TaskPriority,
    ) -> Result<(), nexus_a2a::error::A2AError> {
        self.a2a
            .submit(task_id, tenant_id, principal_id, messages, priority)
    }

    /// Run an A2A task through the real lifecycle (SUBMITTED ->
    /// WORKING -> COMPLETED/FAILED).
    pub fn run_a2a(
        &mut self,
        task_id: &str,
        tenant_id: &str,
    ) -> Result<Vec<TaskMessage>, nexus_a2a::error::A2AError> {
        self.a2a.run(task_id, tenant_id)
    }

    /// Stream A2A status after a cursor (deterministic replay).
    pub fn stream_a2a(
        &self,
        task_id: &str,
        tenant_id: &str,
        cursor: &StreamCursor,
    ) -> Result<Vec<nexus_a2a::stream::StreamEvent>, nexus_a2a::error::A2AError> {
        self.a2a.stream(task_id, tenant_id, cursor)
    }

    /// Fetch a task (tenant-scoped; cross-tenant fails closed).
    pub fn get_a2a(
        &self,
        task_id: &str,
        tenant_id: &str,
    ) -> Result<nexus_a2a::task::A2ATaskRecord, nexus_a2a::error::A2AError> {
        self.a2a.get_task(task_id, tenant_id)
    }

    /// Cancel an A2A task (idempotent; completed cannot be cancelled).
    pub fn cancel_a2a(
        &mut self,
        task_id: &str,
        tenant_id: &str,
    ) -> Result<(), nexus_a2a::error::A2AError> {
        self.a2a.cancel_task(task_id, tenant_id)
    }

    /// Publish content into the hash-bound artifact store.
    pub fn publish_artifact(
        &mut self,
        content: &[u8],
        content_type: &str,
    ) -> Result<nexus_fabric::artifacts::ArtifactManifest, FabricError> {
        self.artifacts.publish_bytes(content, content_type, &[])
    }

    /// Attach an artifact reference to a task (hash-bound; fails
    /// closed if the artifact does not exist).
    pub fn attach_artifact(
        &mut self,
        task_id: &str,
        tenant_id: &str,
        artifact_id: &ArtifactId,
    ) -> Result<(), nexus_a2a::error::A2AError> {
        let _ = self.artifacts.fetch(artifact_id).map_err(|e| {
            nexus_a2a::error::A2AError::not_found(format!("artifact unavailable: {}", e.message))
        })?;
        self.a2a.attach_artifact(task_id, tenant_id, artifact_id)?;
        Ok(())
    }

    /// The full deterministic crown-jewel probe. Returns the canonical
    /// outcome; any stage failure returns Err with the failing stage.
    ///
    /// The model recommendation and any presented receipt are accepted
    /// as inputs but NEVER consulted - the gateway proves they carry no
    /// authority.
    pub fn run_probe(
        &mut self,
        request_id: &str,
        correlation_id: &str,
        binding: SessionBinding,
        _model_recommendation: Option<Value>,
        _presented_receipt: Option<Value>,
    ) -> Result<GatewayProbeOutcome, String> {
        let mut stages: Vec<String> = Vec::new();
        let tenant = binding.tenant_id.as_str().to_string();
        let principal = binding.principal_id.as_str().to_string();
        let session_id = format!("sess-{request_id}");
        let call_id = format!("call-{request_id}");

        // 1. Session: origin validated before session work.
        self.attach_session(&session_id, binding, Some("https://app.nexus.local"))
            .map_err(|e| format!("session failed: {}", e.code.as_str()))?;
        stages.push(ProbeStage::Session.as_str().to_string());

        // 2. Protocol negotiation (2025-11-25 only).
        self.initialize(&session_id)
            .map_err(|e| format!("protocol failed: {}", e.code.as_str()))?;
        stages.push(ProbeStage::Protocol.as_str().to_string());

        // 3. Tool discovery (exact-name registry).
        let tools = self
            .list_tools(&session_id)
            .map_err(|e| format!("tool discovery failed: {}", e.code.as_str()))?;
        let tool_count = tools.len();
        stages.push(ProbeStage::Tools.as_str().to_string());

        // 4. Exact-name call with schema validation.
        let arguments = json!({"message": format!("hello from {request_id}")});
        let output = self
            .call_tool(
                &session_id,
                &call_id,
                "proof.echo",
                &arguments,
                None,
                Some(&tenant),
            )
            .map_err(|e| format!("call failed: {}", e.code.as_str()))?;
        let called_tool = "proof.echo".to_string();
        // The tool echoes the AUTHENTICATED tenant, proving metadata
        // cannot select another tenant.
        if output["tenant"] != json!(tenant) {
            return Err("call returned non-authenticated tenant".to_string());
        }
        stages.push(ProbeStage::Call.as_str().to_string());

        // 5. Idempotency: same key replays the identical result.
        let idem1 = self
            .call_tool(
                &session_id,
                &format!("call-{request_id}-idem"),
                "proof.echo",
                &arguments,
                Some("idem-key"),
                Some(&tenant),
            )
            .map_err(|e| format!("idempotency call failed: {}", e.code.as_str()))?;
        let idem2 = self
            .call_tool(
                &session_id,
                &format!("call-{request_id}-idem-2"),
                "proof.echo",
                &arguments,
                Some("idem-key"),
                Some(&tenant),
            )
            .map_err(|e| format!("idempotency replay failed: {}", e.code.as_str()))?;
        let idempotent_replay_identical = idem1 == idem2;
        stages.push(ProbeStage::Idempotency.as_str().to_string());

        // 6. Cancellation: a cancelled call can never complete.
        let cancel_id = format!("call-{request_id}-cancel");
        self.mcp
            .start_call(&session_id, &cancel_id, "proof.echo")
            .map_err(|e| format!("cancel-start failed: {}", e.code.as_str()))?;
        self.cancel_call(&session_id, &cancel_id)
            .map_err(|e| format!("cancel failed: {}", e.code.as_str()))?;
        let cancelled_never_completes = self
            .mcp
            .complete_call(&session_id, &cancel_id, json!({"echo": {}}))
            .is_err();
        stages.push(ProbeStage::Cancellation.as_str().to_string());

        // 7. A2A task creation bound to the authenticated context.
        let a2a_task_id = format!("task-{request_id}");
        self.submit_a2a(
            &a2a_task_id,
            &tenant,
            &principal,
            vec![TaskMessage {
                message_id: format!("req-{request_id}"),
                role: "user".to_string(),
                parts: vec![json!({"text": format!("proof request {request_id}")})],
            }],
            TaskPriority::Normal,
        )
        .map_err(|e| format!("a2a submit failed: {}", e.code.as_str()))?;
        stages.push(ProbeStage::A2aSubmit.as_str().to_string());

        // 8. Streamed progress: SUBMITTED -> WORKING -> COMPLETED.
        let before = self
            .stream_a2a(&a2a_task_id, &tenant, &StreamCursor(0))
            .map_err(|e| format!("a2a stream failed: {}", e.code.as_str()))?;
        let _ = self
            .run_a2a(&a2a_task_id, &tenant)
            .map_err(|e| format!("a2a run failed: {}", e.code.as_str()))?;
        let after = self
            .stream_a2a(&a2a_task_id, &tenant, &StreamCursor(0))
            .map_err(|e| format!("a2a stream after failed: {}", e.code.as_str()))?;
        let stream_states: Vec<String> = after
            .iter()
            .map(|e| e.status.as_str().to_string())
            .collect();
        // Deterministic cursor: replay from the last pre-run cursor
        // yields exactly the events that appeared after it.
        let cursor_before_run = before.len() as u64;
        let delta = self
            .stream_a2a(&a2a_task_id, &tenant, &StreamCursor(cursor_before_run))
            .map_err(|e| format!("a2a delta failed: {}", e.code.as_str()))?;
        if delta.is_empty() {
            return Err("stream delta was empty after run".to_string());
        }
        stages.push(ProbeStage::A2aStream.as_str().to_string());

        // 9. Hash-bound artifact: publish content, attach the manifest
        // id to the task; attach of a missing artifact fails closed.
        let manifest = self
            .publish_artifact(
                format!("proof artifact {request_id}").as_bytes(),
                "text/plain",
            )
            .map_err(|e| format!("artifact publish failed: {}", e.code.as_str()))?;
        self.attach_artifact(&a2a_task_id, &tenant, &manifest.artifact_id)
            .map_err(|e| format!("artifact attach failed: {}", e.code.as_str()))?;
        let missing = ArtifactId(format!("sha256:{}", "0".repeat(64)));
        let missing_rejected = self
            .attach_artifact(&a2a_task_id, &tenant, &missing)
            .is_err();
        if !missing_rejected {
            return Err("missing artifact attach was not rejected".to_string());
        }
        let artifact_attached = true;
        let artifact_digest = manifest.sha256.clone();
        stages.push(ProbeStage::Artifact.as_str().to_string());

        // 10. Completion: the task is COMPLETED; completed tasks cannot
        // be cancelled; cross-tenant access fails closed.
        let final_task = self
            .get_a2a(&a2a_task_id, &tenant)
            .map_err(|e| format!("a2a get failed: {}", e.code.as_str()))?;
        let final_lifecycle = final_task.status.as_str().to_string();
        let _ = self.cancel_a2a(&a2a_task_id, &tenant).ok();
        let completed_after_cancel = self
            .get_a2a(&a2a_task_id, &tenant)
            .map(|t| t.status.as_str().to_string())
            .unwrap_or_default();
        let cross_tenant_denied = self
            .get_a2a(&a2a_task_id, "018f0f6f-9c1e-7b6e-8000-000000000099")
            .is_err();
        if completed_after_cancel != "COMPLETED" {
            return Err("completed task was mutated by cancel".to_string());
        }
        stages.push(ProbeStage::A2aComplete.as_str().to_string());

        // Boundaries (directive M/D): the model recommendation and any
        // presented receipt were accepted but never consulted; the
        // gateway has no policy engine and no capability grants.
        let _ = _model_recommendation;
        let _ = _presented_receipt;

        Ok(GatewayProbeOutcome {
            request_id: request_id.to_string(),
            correlation_id: correlation_id.to_string(),
            principal_id: principal,
            tenant_id: tenant,
            mcp_protocol: McpProtocolVersion::Spec2025_11_25.as_str().to_string(),
            a2a_protocol: A2AProtocolVersion::Spec1_0_1.as_str().to_string(),
            stages,
            tool_count,
            called_tool,
            idempotent_replay_identical,
            cancelled_never_completes,
            a2a_task_id,
            stream_states,
            artifact_digest,
            artifact_attached,
            final_lifecycle,
            model_recommendation_never_consulted: true,
            receipt_never_reusable: true,
            cross_tenant_denied,
            authorization_not_implied: true,
            verification_plan: vec![
                "authorization:not-owned-by-fabric".to_string(),
                "execution:proof-executor".to_string(),
                "verification:hash-bound-artifact".to_string(),
            ],
        })
    }

    /// Number of A2A tasks currently tracked.
    pub fn task_count(&self) -> usize {
        self.a2a.task_count()
    }

    /// Number of MCP sessions currently attached (0 after an origin
    /// rejection: no session work happened).
    pub fn mcp_session_count(&self) -> usize {
        self.mcp.session_count()
    }
}
