# EP-010 M5 composed capability subsystem proof

- Node: `EP-010`
- Milestone: `M5`
- Correlation ID: `018f0f6f-9c1e-7b6e-8000-000000000002`
- Tenant: `018f0f6f-9c1e-7b6e-8000-000000000003`
- Composed subsystem: `nexus-capabilities + nexus-connectors + canonical schemas`
- Validator: `jsonschema 0.49.9 (draft 2020-12)`
- Capability descriptor schema: `v1`
- Connector manifest schema: `v1`
- Schema evolution: current-version parity `PASS`, future-version migration `NOT ASSERTED`

## Canonical authorization ordering (EP-010 boundary)

`REGISTER_DISCOVER -> UNAVAILABLE_NOT_ADVERTISED -> QUERY_DISPATCH -> COMMAND_IDEMPOTENT -> WORKFLOW_DISPATCH -> HEALTH -> CHANGEFEED -> CLASS_MISMATCH_DENIED -> CROSS_TENANT_DENIED -> PROVIDER_ERROR_FAIL_CLOSED -> IDEMPOTENCY_CONFLICT -> SCHEMA_VALIDATION -> SCHEMA_REJECTION`

## Stage results (observed)

- **REGISTER_DISCOVER**: `PASS` - discovered=["test.command", "test.query", "test.stream", "test.workflow"] re-register_idempotent=true len_stable=true
- **UNAVAILABLE_NOT_ADVERTISED**: `PASS` - unavailable capability omitted from discovery
- **QUERY_DISPATCH**: `PASS` - output={"request_id":"018f0f6f-9c1e-7b6e-8000-000000000001","state":"on"}
- **COMMAND_IDEMPOTENT**: `PASS` - first={"applied":true,"request_id":"018f0f6f-9c1e-7b6e-8000-000000000001"} second={"applied":true,"request_id":"018f0f6f-9c1e-7b6e-8000-000000000001"} records=1
- **WORKFLOW_DISPATCH**: `PASS` - workflow_id=wf-livefire-1
- **HEALTH**: `PASS` - state=HEALTHY
- **CHANGEFEED**: `PASS` - events=1 next_cursor=cursor-livefire-2
- **CLASS_MISMATCH_DENIED**: `PASS` - code=VALIDATION
- **CROSS_TENANT_DENIED**: `PASS` - code=NOT_FOUND
- **PROVIDER_ERROR_FAIL_CLOSED**: `PASS` - query_code=UNAVAILABLE command_code=UNAVAILABLE cached_success=false
- **IDEMPOTENCY_CONFLICT**: `PASS` - code=CONFLICT
- **SCHEMA_VALIDATION**: `PASS` - descriptor_errors=[] manifest_errors=[]
- **SCHEMA_REJECTION**: `PASS` - unknown_class=true missing_required=true duplicate_events=true duplicate_secrets=true duplicate_origins=true

## Authority boundaries

- descriptor_is_metadata_only: `True`
- ep005_event_transport_authority: `EP-005 owns event transport substrate`
- ep006_workflow_authority: `EP-006 owns durable workflow execution`
- ep008_authorization_authority: `EP-008 owns authorization to invoke`
- external_connector_certification: `NOT OWNED BY EP-010`
- health_is_observation_only: `True`
- tier_is_metadata_only: `True`

## Certification boundaries (directive T)

- capability_contract_certification: `PASS`
- deterministic_registry_dispatcher_certification: `PASS`
- canonical_schema_parity: `PASS`
- forced_failure_behavior: `PASS`
- composed_ep010_subsystem_proof: `PASS`
- external_connector_provider_certification: `NOT OWNED BY EP-010`

Evidence is derived from the observed probe output of
`crates/nexus-connectors/examples/livefire_probe.rs` and the real
jsonschema 0.49.9 validator. No credentials, bearer tokens, private
data, or raw provider payloads are persisted.
