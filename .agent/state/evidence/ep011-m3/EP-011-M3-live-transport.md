# EP-011 M3 Evidence - Real Connector Transport and Cross-Language Parity

Date: 2026-08-14
Node: EP-011 (connector SDKs and sidecar runtime)
Milestone: M3 (real dependency and transport integration)
Agent: hermes-nexus-main

## Real process transport

The proof harness is `tests/connectors/fixture_sidecar.py`, a REAL HTTP
server process bound to 127.0.0.1 with an ephemeral port, implementing
the canonical sidecar REST protocol (SPEC-022 Tier 1 REST). Every M3
proof crosses a real process boundary:

  Nexus-side test client -> real HTTP -> fixture sidecar process
    -> Python SDK implementation -> fixture provider

No direct function calls are used for the transport proofs; no
in-process mocks substitute for the sidecar.

## Canonical wire corpus

19 golden fixtures generated from the authoritative Rust types
(`cargo run -p nexus-connector-sdk --example generate_golden`) are
committed under `tests/connectors/golden/`:

invocation_context, capability_descriptor, connector_manifest,
query_request, query_result, command_request, command_result,
workflow_request, workflow_result, health_report, change_cursor,
change_batch, webhook_event, raw_webhook, normalized_webhook,
sidecar_request, sidecar_response, credential_reference,
error_envelope

All three bindings deserialize and serialize against the SAME corpus;
comparisons are semantic structures, never raw JSON strings.

## Proof summary (directive T)

- Real process transport: PASS (Rust reqwest + TS fetch + Python urllib
  over real HTTP to the fixture sidecar process)
- Rust SDK parity: PASS (10 golden + 9 transport tests)
- TypeScript SDK parity: PASS (7 golden + 7 transport vitest tests)
- Python SDK parity: PASS (58 integration/failure pytest tests)
- Canonical wire parity: PASS (same golden corpus in all three
  languages; snake_case field names; Option fields as JSON null)
- Query dispatch: PASS (provider executes once; canonical result)
- Command/idempotency transport: PASS (replay identity; key bound to
  capability; cross-capability key reuse = CONFLICT, provider not
  re-executed)
- Workflow transport dispatch: PASS (RUNNING handle returned;
  durable Temporal execution NOT claimed - EP-006 owns Temporal)
- Health transport: PASS (observation only; no authorization material)
- Webhook normalization: PASS (two provider shapes normalize to the
  same canonical event; bad signature fail-closed; unknown event type
  preserved as typed metadata)
- Legacy polling: PASS (real JSONL source; unchanged poll = no
  fabricated change; real mutation observed; restart resumes from
  persisted checkpoint)
- Sidecar lifecycle: PASS (start/readiness/query/health/controlled
  shutdown; typed UNAVAILABLE when not listening; protocol-version
  mismatch fails closed)
- Credential-broker boundary: PASS (reference-only on the wire;
  fingerprint only crosses transport; broker-unavailable fails closed;
  no secrets in manifests/descriptors/discovery/errors/telemetry)
- Typed transport failures: PASS (10-case matrix; no generic success)
- Cross-language error parity: PASS (NOT_FOUND, VALIDATION,
  CONFLICT, UNAVAILABLE mapped to identical canonical codes)
- Protocol-version rejection: PASS (426 + VALIDATION envelope for
  unsupported version; payload never silently reinterpreted)
- Zero-orphan teardown: PASS (clean shutdown exit 0; port released;
  `scripts/ep011-orphan-audit.sh`: ok)

External SaaS/provider certification: NOT ASSERTED BY EP-011 M3.

## Observed sentinels

- `EP-011 M3: ok`
- cargo test -p nexus-connector-sdk ep011_unit: 19 passed
- cargo test -p nexus-connector-sdk ep011_integration: 19 passed
  (10 golden_parity + 9 transport_live)
- pnpm --filter @nexus/connector-sdk test:unit: 26 passed
- EP-011 vacuity: ok (58 M3 tests passed)
- EP-011 orphan audit: ok

## Correlation IDs and fingerprints (evidence only)

- Correlation: 018f0f6f-9c1e-7b6e-8000-000000000002
- Tenant A: 018f0f6f-9c1e-7b6e-8000-000000000003 (fingerprinted in
  telemetry)
- Connector fingerprint: sha256("fixture-connector")[:16]
- Broker credential fingerprint: sha256("fixture-secret-value")[:16]
  (the raw value never appears in any evidence, response, log, or
  telemetry)

## Canonical authorization ordering observed

discover -> class check -> typed dispatch (query/command/workflow/
health/changefeed) -> idempotency check (commands) -> credential
resolution inside sandbox -> canonical result or typed error envelope
(code, message, correlation_id, actor, tenant, resource).
