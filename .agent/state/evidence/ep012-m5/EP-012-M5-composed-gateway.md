# EP-012 M5 - Composed Fabric Gateway Live-Fire

Real engines: nexus-mcp McpEngine + nexus-a2a A2AGatewayImpl + hash-bound artifact store.

- request_id: `0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01`
- correlation_id: `corr-0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01`
- principal_id: `018f0f6f-9c1e-7b6e-8000-00000000000a`
- tenant_id: `018f0f6f-9c1e-7b6e-8000-000000000001`
- mcp_protocol: `2025-11-25`
- a2a_protocol: `1.0.1`

## Canonical ordering

```
SESSION_PASS
PROTOCOL_PASS
TOOLS_PASS
CALL_PASS
IDEMPOTENCY_PASS
CANCELLATION_PASS
A2A_SUBMIT_PASS
A2A_STREAM_PASS
ARTIFACT_PASS
A2A_COMPLETE_PASS
```

- tool_count: 1
- called_tool: `proof.echo`
- idempotent_replay_identical: true
- cancelled_never_completes: true
- a2a_task_id: `task-0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01`
- stream_states: SUBMITTED, WORKING, COMPLETED
- artifact_digest: `864b494162b63d8e8d4824c5e26ca465e38d6bda203611f1e9ae6e0e1cfc532f`
- artifact_attached: true
- final_lifecycle: `COMPLETED`
- cross_tenant_denied: true

## Authority boundaries

- model_recommendation_never_consulted: true
- receipt_never_reusable: true
- authorization_not_implied: true

MCP acceptance != execution authorization (EP-008 owns authorization).
A2A task identity/tenant scope != capability grant.
Artifact integrity (hash binding) != execution authority.
Protocol acceptance != execution permission.

## Verification plan

- authorization:not-owned-by-fabric
- execution:proof-executor
- verification:hash-bound-artifact

