NODE-META-BEGIN
ID: EP-038
DEPS: EP-037
MAX_ATTEMPTS_PER_MILESTONE: 6
VERIFY: sh scripts/node-verify.sh EP-038
VERIFY_SENTINEL: node verify EP-038: ok
GREEN_TAG: green/EP-038
NODE-META-END

# 1. Purpose / Big Picture

Implement OpenTelemetry, GlitchTip, metrics, logs, traces, dashboards, alerts, SLOs, fleet health, and incident operations. This node is a bounded part of the final Nexus Life and Business OS. It must leave the repository green, preserve every lower-layer invariant, expose stable provider-neutral contracts, and create evidence that a lower-tier executor can independently verify.

# 2. Scope

- Implement the public interfaces in `.agent/node-contracts/EP-038.md`.
- Create only the exact files and directories authorized by `.agent/expected-files/EP-038.txt`.
- Implement real behavior, tests, telemetry, security, operations, and any owning live-fire proof.
- Preserve self-hosted-first selection and API fallback contracts.
- Keep optional providers disabled until certified.

# 3. Non-goals

- No work owned by a later node.
- No broad refactor, dependency replacement, vendor-specific domain model, or alternate architecture.
- No production deployment.
- No mocks, stubs, demonstration modes, or sample success in production paths.
- No claim that an adapter or hardware class is operational before real certification.
- No weakening of a spec, policy, security boundary, test, or GraphLock gate.

# 4. Context and Orientation

Nexus is logically one brain and physically a distributed control system. Domain and application code define intent; provider adapters implement replaceable infrastructure; OpenFGA and OPA provide authority inputs; the Action Gateway controls effects; PostgreSQL and NATS preserve durable truth and events; Temporal preserves long work; all clients and agents consume the same contracts. This node depends on `EP-037` and must not assume later components exist.

# 5. Files to Read First

- `AGENTS.md`
- `COMMANDS.md`
- `.agent/GRAPH.md`
- `.agent/LOOPS.md`
- `ARCHITECTURE.md`
- `SECURITY.md`
- `TESTING.md`
- `.agent/node-contracts/EP-038.md`
- `.agent/specs/SPEC-007-observability-incident-correlation-and-operations.md`

# 6. Expected Changed Files

The machine fence is `.agent/expected-files/EP-038.txt`. Directory entries authorize descendants. The scope audit rejects every other path.

- `.agent/execplans/EP-038-observability-and-operations.md`
- `.agent/state/LEDGER.md`
- `.agent/expected-files/EP-038.txt`
- `.agent/node-contracts/EP-038.md`
- `scripts/nodes/EP-038.sh`
- `crates/nexus-observability/`
- `infra/otel/`
- `infra/glitchtip/`
- `infra/observability/`
- `dashboards/`
- `alerts/`
- `tests/observability/`

# 7. Interfaces and Contracts

| Interface | Owning package or boundary | Contract |
| --- | --- | --- |
| `TelemetryContext` | `nexus-observability` | Defined by EP-038; provider-neutral and versioned |
| `RedactionPolicy` | `nexus-observability` | Defined by EP-038; provider-neutral and versioned |
| `MetricCatalog` | `nexus-observability` | Defined by EP-038; provider-neutral and versioned |
| `TracePolicy` | `nexus-observability` | Defined by EP-038; provider-neutral and versioned |
| `HealthAggregator` | `nexus-observability` | Defined by EP-038; provider-neutral and versioned |
| `IncidentSink` | `nexus-observability` | Defined by EP-038; provider-neutral and versioned |
| `FleetHealth` | `nexus-observability` | Defined by EP-038; provider-neutral and versioned |
| `SloEvaluator` | `nexus-observability` | Defined by EP-038; provider-neutral and versioned |

Acceptance obligations:

1. OpenTelemetry is the instrumentation standard
2. GlitchTip receives errors and incidents without secrets
3. Logs, metrics, traces, audit, and events correlate
4. Dashboards and alerts answer health, failures, agent work, cost, cache, and security

Every interface uses typed IDs, authenticated tenant and principal context, canonical errors, correlation, idempotency for retryable commands, and OpenTelemetry context. A provider implementation may add internal types but cannot alter the canonical contract.

# 8. Milestones


### M1: Contract, vocabulary, and package boundary

GOAL: Create the owned package or infrastructure roots and encode the public contracts for implement opentelemetry, glitchtip, metrics, logs, traces, dashboards, alerts, slos, fleet health, and incident operations.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-038-M1.txt`, `.agent/node-contracts/EP-038.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `.agent/execplans/EP-038-observability-and-operations.md`, `.agent/state/LEDGER.md`, `.agent/expected-files/EP-038.txt`, `.agent/node-contracts/EP-038.md`, `scripts/nodes/EP-038.sh`, `crates/nexus-observability/`, `alerts/`

CONTENT:

1. Read the accepted specs and node contract before creating code.
2. Create the owned workspace manifests and module roots in the exact language and layer assigned by ARCHITECTURE.md.
3. Define every public interface listed in the Interface Map with versioned serialization or transport contracts where applicable.
4. Create tests whose names begin `ep038_unit_` and prove construction, validation, serialization, vocabulary rejection, and dependency-direction constraints.
5. Update generated language bindings only through `schemas/` and `scripts/generate-contracts.sh` when the node owns cross-language contracts.
6. Do not create provider-specific behavior in domain or application ports.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-038.sh M1`

EXPECT:

- `EP-038 M1: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-038 MILESTONE_PASS "M1 EP-038 M1: ok"`

FALLBACK: Use local structured logs and Prometheus metrics when external collectors are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-038][M1] contract, vocabulary, and package boundary"`

### M2: Core behavior and deterministic invariants

GOAL: Implement the production behavior and deterministic invariants owned by EP-038.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-038-M2.txt`, `.agent/node-contracts/EP-038.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/otel/`, `tests/observability/`

CONTENT:

1. Implement all acceptance obligations in the node contract without test-mode branches.
2. Keep domain rules pure and move I/O behind ports; infrastructure adapters may import application ports, never the reverse.
3. Create tests whose names begin `ep038_unit_` and exercise real implementation, boundary values, concurrency or idempotency where applicable, and unauthorized states.
4. Return typed errors from SPEC-006 and preserve request, correlation, actor, tenant, and resource references.
5. Instrument public operations with the canonical telemetry context but never emit secrets, prompts, raw audio, raw video, or private content.
6. Document every ordinary implementation choice in the plan Decision Log before committing it.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-038.sh M2`

EXPECT:

- `EP-038 M2: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-038 MILESTONE_PASS "M2 EP-038 M2: ok"`

FALLBACK: Use local structured logs and Prometheus metrics when external collectors are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-038][M2] core behavior and deterministic invariants"`

### M3: Real dependency and transport integration

GOAL: Connect EP-038 to its real selected dependencies and prove contract behavior across the boundary.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-038-M3.txt`, `.agent/node-contracts/EP-038.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/glitchtip/`

CONTENT:

1. Use the selected open-source component or real local dependency from COMPONENT_REGISTRY.yaml; do not substitute an in-memory production engine.
2. Create migrations, container configuration, provider manifests, policies, fixtures, or generated clients required by the exact changed-file fence.
3. Create integration tests whose names begin `ep038_integration_` and use real ephemeral containers, controlled provider sandboxes, or owned test hardware as the specification requires.
4. Prove readiness, cancellation, timeout, idempotency, event emission, audit, and cleanup across the boundary.
5. If the component is optional, keep its advertised capability unavailable until provider or hardware certification evidence exists.
6. Record exact component version, digest, license, source, and replacement contract.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-038.sh M3`

EXPECT:

- `EP-038 M3: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-038 MILESTONE_PASS "M3 EP-038 M3: ok"`

FALLBACK: Use local structured logs and Prometheus metrics when external collectors are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-038][M3] real dependency and transport integration"`

### M4: Forced failures, abuse cases, and observability

GOAL: Prove EP-038 fails safely under dependency, policy, security, and resource faults.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-038-M4.txt`, `.agent/node-contracts/EP-038.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `infra/observability/`

CONTENT:

1. Create tests whose names begin `ep038_failure_` for unavailable dependency, timeout, malformed input, duplicate request, denied permission, cancelled work, and partial side effect where applicable.
2. Exercise the real failure mechanism: terminate a test container, revoke a sandbox token, corrupt a controlled message, exhaust a declared budget, or deny a policy decision. Do not mock the component being proven.
3. Prove rollback, compensation, quarantine, retry, or fail-closed behavior according to the owning spec.
4. Assert structured errors, redacted logs, metrics, traces, audit records, and incident correlation.
5. Run the security and license gates and correct the implementation rather than adding a broad allowlist.
6. Add an operations diagnostic and bounded recovery command for every new service or provider.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-038.sh M4`
2. `sh scripts/security-check.sh`
3. `sh scripts/license-gate.sh`

EXPECT:

- `EP-038 M4: ok`
- `security check: ok`
- `license gate: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-038 MILESTONE_PASS "M4 EP-038 M4: ok"`

FALLBACK: Use local structured logs and Prometheus metrics when external collectors are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-038][M4] forced failures, abuse cases, and observability"`

### M5: Live-fire, operations, and node closure

GOAL: Complete operational proof, documentation, and immutable node evidence for EP-038.

READ: Re-read this milestone, Section 3 Non-goals, `.agent/milestone-files/EP-038-M5.txt`, `.agent/node-contracts/EP-038.md`, the owning accepted specs, and `sh scripts/ledger.sh tail 15`.

CHANGE: `dashboards/`

CONTENT:

1. Run every live-fire proof owned by this node using real controlled dependencies and write machine-readable evidence under `.agent/state/evidence/`.
2. Update provider or hardware certification results only when the certification workflow produced signed evidence.
3. Complete health, readiness, backup, restore, upgrade, disable, and rollback instructions for the owned components.
4. Run the node script in verify mode, full repository verify, expected-file audit, adapter parity, and scope audit.
5. Fill Progress, Surprises and Discoveries, Decision Log, and Outcomes with actual commands, exit codes, sentinels, and evidence paths.
6. Append NODE_DONE and create `green/EP-038` only after all acceptance obligations pass.

All new public names must come from accepted vocabularies or be added by an ADR and schema update in the same milestone. Production code contains no placeholder, demonstration branch, sample success, or hidden fallback. Test-double code remains under TESTING.md's test zones.

RUN:

1. `sh scripts/nodes/EP-038.sh M5`
2. `sh scripts/node-verify.sh EP-038`
3. `sh scripts/scope-audit.sh EP-038`

EXPECT:

- `EP-038 M5: ok`
- `node verify EP-038: ok`
- `scope audit EP-038: ok`

EVIDENCE: `sh scripts/ledger.sh append <AGENT_ID> EP-038 MILESTONE_PASS "M5 EP-038 M5: ok"`

FALLBACK: Use local structured logs and Prometheus metrics when external collectors are unavailable. The fallback must satisfy the same public contract, tests, authorization, audit, and live-fire obligations; it may reduce optional breadth but never simulate success.

COMMIT: `git add -A && git commit -m "[EP-038][M5] live-fire, operations, and node closure"`


# 9. Validation and Acceptance

Run `sh scripts/node-verify.sh EP-038` and observe `node verify EP-038: ok`. Then walk every acceptance obligation above and cite the exact test or evidence path. Required provider and hardware certifications must be real; unavailable optional capabilities may remain disabled only when the release profile permits it.

Owned live-fire proofs:

- No standalone live-fire proof is owned by this node. Its behavior is exercised by downstream proofs and the node-specific real dependency tests.

# 10. Idempotence and Recovery

Resume cold by running the boot sequence, confirming the lease, reading Progress and ledger evidence, and rerunning the last checked milestone sentinel. All provisioning, migration, event consumption, provider writes, and workflow activities must be idempotent. Before a risky mutation, create the specified backup or snapshot. Rollback to the previous milestone commit under LOOPS.md; never cross a completed green tag.

# 11. Progress

- [x] M1: Contract, vocabulary, and package boundary
- [ ] M2: Core behavior and deterministic invariants
- [ ] M3: Real dependency and transport integration
- [ ] M4: Forced failures, abuse cases, and observability
- [ ] M5: Live-fire, operations, and node closure

## M1 progress (observed)

- Created `crates/nexus-observability/` - provider-neutral contract crate
  (no Prometheus/Grafana/OTel SDK/Datadog/Honeycomb/Sentry/Loki/Tempo/
  Jaeger/cloud SDK dependencies; cargo tree verified).
- All 8 public interfaces implemented and re-exported: TelemetryContext,
  RedactionPolicy, MetricCatalog (MetricRegistry), TracePolicy,
  HealthAggregator (CompositeHealthAggregator), IncidentSink
  (RecordingIncidentSink), FleetHealth, SloEvaluator
  (WindowedSloEvaluator). Port traits isolated in `src/port.rs`.
- Redaction-first model: `RedactedEnvelope` is the only egress form;
  `OBSERVED RAW EVENT != EXPORTABLE TELEMETRY`; secret-shaped values are
  hashed (`sha256:`-prefixed to avoid re-classification as artifact
  keys), payload/prompt/token fields denied by default, unclassified
  values fail closed to `[REDACTED]`; `contains_secret_shaped` rejects
  embedded keys in metadata fields.
- Health ladder: CONFIGURED != REACHABLE != RESPONDING != READY; stale
  observations compose to Unknown/Degraded, never healthy.
- FleetHealth: staleness visible; one healthy node never makes an
  unknown fleet healthy; unsafe-to-claim when critical nodes unknown.
- SLO: total=0 -> NoData; below min_evidence -> InsufficientEvidence;
  never green without data.
- IncidentSink: dedupe by key; severity escalation never hidden;
  redacted bodies; id->dedupe_key index fixes get/ack/resolve.
- Traces: TRACE ID PRESENT != TRACE EXPORTED != TRACE SAFE; denied
  attribute keys force Denied (fail-closed); redaction before export.
- Deny-unknown vocabularies: Severity, HealthState, MetricKind,
  IncidentState, SloState, RedactionAction, TelemetrySignal,
  StabilityLevel, CardinalityPolicy; serde rejects unknown wire values.
- `alerts/` M1-owned contract/config: README.md, catalog.yaml (6 rules),
  redaction-policy.yaml (fail-closed), slo-catalog.yaml (3 SLOs).
- Gate: `scripts/ep038-m1-tests.sh` (17 sentinel anti-masking guards,
  vacuity guards, dependency-direction, alerts validation, clippy/fmt);
  node `scripts/nodes/EP-038.sh M1` rewired from artifact-check masking
  to the real gate.
- Test counts: 27 unit tests green (27 passed; 0 failed; 0 ignored);
  5 real contract defects found and fixed during the run (embedded-key
  context rejection, hash prefix, trace denied-key semantics, sink
  id->dedupe index, error size).
- Certification boundary: nexus-observability + 8 interfaces + alerts/ =
  CONTRACT CERTIFIED; Prometheus/Grafana/OTel collector/GlitchTip/
  Loki/Tempo/Jaeger/incident delivery = NOT ASSERTED.
- M1 closure: EP-037 M4 fresh-provider proof refreshed 2026-08-23
  (canonical self-provisioned gate 21/21 on a truly fresh SeaweedFS
  4.43 runtime; M5 gate green incl. M4 regression; workspace battery
  green on approved scope with live battery providers; foreign LF-*
  evidence churn from gate reruns reverted per EP-037 closure
  precedent).

## M2 progress (observed)

- Created `infra/otel/` - crate `nexus-otel` (workspace member), the
  OpenTelemetry provider layer consuming the M1 contracts
  (`nexus-observability` + `nexus-domain` + `serde_json` only; no
  vendor telemetry SDK - dependency direction enforced).
- `src/otlp.rs` - hand-rolled OTLP/JSON serialization for traces,
  metrics, and logs. Wire-format facts verified against the
  authoritative `opentelemetry-proto` sources before coding:
  trace_id = 32 lowercase base16 chars, span_id = 16 base16 chars,
  camelCase field names (resourceSpans/scopeSpans/traceId/spanId/
  startTimeUnixNano/severityNumber/...), fixed64 timestamps as
  decimal strings (proto3 JSON mapping), SpanKind INTERNAL=1,
  StatusCode UNSET=0/ERROR=2, SeverityNumber DEBUG=5 INFO=9 WARN=13
  ERROR=17 FATAL=21, Sum CUMULATIVE=2 monotonic for counters.
- `src/prometheus.rs` - Prometheus text exposition format 0.0.4
  writer (node-contract fallback): HELP/TYPE lines, label/docstring
  escaping (backslash, double-quote, newline), sorted labels,
  trailing LF, Go strconv value formatting (NaN/+Inf/-Inf).
- `src/structured.rs` - bounded JSON-lines structured-log fallback.
- `src/export.rs` - export boundary: the ONLY entry points to the
  serializers; accepts `RedactedEnvelope` only and re-verifies
  `assert_exportable()` before any byte is produced. No API accepts
  raw observed events.
- `tests/observability/` - crate `nexus-observability-tests`
  (workspace member) with 24 `ep038_unit_*` proofs: OTLP wire shape
  exactness (camelCase, base16 ids, fixed64 strings, severity
  mapping), redaction canaries absent from OTLP/JSON/Prometheus/
  structured output, export boundary rejects non-exportable
  envelopes, Prometheus escaping + value formatting + name
  validation, deterministic output + sorted resource attributes,
  structured-log shape, histogram/distribution truthfully
  UnsupportedSignal (bucket layout owned by a later milestone).
- Real defects found and fixed by the proofs: (1) resource
  attributes were not sorted -> now sorted by key for deterministic
  wire output; (2) test expectation corrected - the `payload` FIELD
  NAME may appear with a sha256: fingerprint value; the raw secret
  never does (Hash redaction action keeps the key, redacts the
  value).
- Gate `scripts/ep038-m2-tests.sh` (non-vacuous): material presence,
  24 anti-masking sentinels, vacuity guards, dependency-direction
  proof, authoritative wire-field presence check, clippy/fmt, M1
  regression. Node `scripts/nodes/EP-038.sh M2` rewired from
  artifact-check masking to the real gate.
- Test counts: 24 M2 provider proofs green (0 failed/ignored); M1 27
  regression green; clippy -D warnings clean; fmt clean.
- Certification boundary: OTLP/JSON serialization for traces/metrics/
  logs INTERNAL PROVIDER CERTIFIED for exact exercised wire shapes;
  Prometheus text 0.0.4 writer FORMAT CERTIFIED for exact exercised
  grammar; structured-log fallback CERTIFIED for exact exercised JSON
  shape. NOT ASSERTED: a Prometheus server, an OTel collector, OTLP
  network transport, Grafana, GlitchTip, Loki, Tempo, Jaeger,
  incident delivery, production monitoring deployment (M3+ own
  them).

## M3 progress (observed)

- Fence (`.agent/milestone-files/EP-038-M3.txt`): M3 owns `infra/glitchtip/`
  real dependency + transport integration; node `scripts/nodes/EP-038.sh M3`
  must emit `EP-038 M3: ok`; commit theme `real dependency and transport
  integration`. MANIFEST requires `scripts/probes/glitchtip.sh` (upgraded
  from placeholder to the full ladder by M3).
- Created `infra/glitchtip/` - crate `nexus-glitchtip` (workspace member):
  `dsn.rs` (DSN parse, secret-safe diagnostics, 32-hex public key never
  rendered), `envelope.rs` (exact Sentry envelope grammar
  `Headers {"\n" Item} ["\n"]`), `event.rs` (bounded event builder),
  `transport.rs` (hand-rolled std::net POST; SPEC-006 mapping
  refused->Unavailable, timeout->Timeout, 401/403->Authorization,
  404->NotFound, 429->RateLimit, 5xx->ExternalProvider, malformed->
  ExternalProvider; fresh TcpStream per request), `incident.rs`
  (RedactedEnvelope-only boundary; dedupe key -> Sentry fingerprint;
  per-delivery event-id nonce), `sink.rs` (M1 IncidentSink impl with
  dedupe/escalation semantics), `diag.rs` (probe ladder
  CONFIGURED != REACHABLE != RESPONDING != ACCEPTED != VERIFIED).
- Integration proofs in `tests/glitchtip/` (crate `nexus-glitchtip-tests`,
  workspace member): 4 real-provider tests (`ep038_integration_*`) +
  dedicated stopped-provider binary (`ep038_m3_stopped.rs`) executed by the
  gate as a separate cargo invocation after the real fixture is stopped.
- Gate `scripts/ep038-m3-tests.sh` (non-vacuous): provisions REAL
  postgres:18.4 + redis:7-alpine (network alias `redis`, required by baked
  django cache config) + glitchtip:6.1.8 (`SERVER_ROLE=all_in_one`,
  `GLITCHTIP_EMBED_WORKER=true`), runtime-generated credentials, Django
  shell provisioning (users.User.create_user, org, OrganizationUser
  role=3, Project, ProjectKey.public_key.hex, APIToken scopes=1153 =
  project:read(0)|event:read(7)|org:read(10)), mode-600 env/token files,
  unit 40/40, integration 4/4, stopped phase 1/1 (refused -> Unavailable),
  exact pass-count vacuity guards, orphan guard (owned containers/network/
  volume/temp files), trap teardown on every exit path. Node M3 rewired
  from artifact-check masking to the real gate.
- `scripts/probes/glitchtip.sh` ladder upgraded: CONFIGURED (DSN present
  and shaped) -> REACHABLE (TCP) -> RESPONDING (HTTP status) ->
  AUTHENTICATED (envelope POST with X-Sentry-Auth accepted) -> READY
  (token readback returns real issues). Never prints the DSN key/token.
- Test counts: M3 unit 40/40 green; M3 integration 4/4 green; stopped
  phase 1/1 green; M1 regression 27 green; M2 regression 24 green; clippy
  -D warnings clean; fmt clean; security/license/dependency-audit/reality/
  scope-audit gates green. Expected-files full-list gate is M5-owned (the
  list intentionally includes `infra/observability/`, `dashboards/` which
  do not exist until M4/M5; same as M1/M2).
- Real defects found and fixed by the proofs: (1) GlitchTip 6.1.8
  authenticates envelope ingestion from the `X-Sentry-Auth` header, NOT
  the envelope-body `dsn` (the probe originally omitted `sentry_key` and
  would 403 against a healthy provider); (2) event_id was derived from
  incident_id alone, so an escalated redelivery of the same incident got a
  duplicate event_id and the provider dropped it -> per-delivery nonce
  added; (3) redis must answer the baked-in hostname `redis` (network
  alias on the owned container); (4) postgres:18+ refuses a volume mount
  at `/var/lib/postgresql/data` (versioned PGDATA) -> mount the parent;
  (5) readback is asynchronous: HTTP 200 acceptance precedes worker
  processing by seconds -> integration tests poll readback against a
  monotonic 30s deadline with recorded last observation.
- Certification boundary: GlitchTip adapter INTEGRATION CERTIFIED for the
  exact real fixture path exercised (real 6.1.8, real postgres 18.4, real
  redis 7-alpine, real envelope POST + worker + readback); stopped-
  provider handling CERTIFIED for the exact stopped fixture phase. NOT
  ASSERTED: production GlitchTip deployment, GlitchTip SaaS/Sentry cloud,
  PagerDuty/Slack/email incident delivery, arbitrary Sentry-compatible
  providers, production monitoring deployment, real fleet incident
  operations beyond the exercised fixture.

# 12. Surprises & Discoveries

- 2026-08-23 (M3, real GlitchTip 6.1.8): envelope HTTP 200 != processed.
  The provider accepts the envelope immediately; the embedded worker
  creates the issue asynchronously (seconds). Readback immediately after
  POST returns zero issues; deadline-based readback polling is required.
- 2026-08-23 (M3, real GlitchTip 6.1.8): `event_auth`/`auth_from_request`
  authenticates via `X-Sentry-Auth` header or `?sentry_key=` query param;
  the envelope-body `dsn` header is IGNORED for authentication. The DSN
  public key must be placed in the header.
- 2026-08-23 (M3, real GlitchTip 6.1.8): duplicate `event_id` events are
  dropped by the provider (an escalated redelivery with a stale
  event_id never lands). Event ids must be unique per delivery.
- 2026-08-23 (M3): GlitchTip's django cache config bakes in hostname
  `redis`; a differently-named redis container breaks org/project
  creation (`Organization.save` -> `clear_metrics_cache` -> name
  resolution failure). Use `--network-alias redis`.
- 2026-08-23 (M3): postgres:18+ official image refuses data volumes at
  `/var/lib/postgresql/data` ("Error: in 18+, these Docker images are
  configured to store database data in a..."): mount the parent
  `/var/lib/postgresql` instead.
- 2026-08-23 (M3): APIToken scopes are a BitField; the working readback
  token carries scopes=1153 = bits project:read(0) | event:read(7) |
  org:read(10). ProjectKey.public_key is a UUID; the DSN key is its
  32-hex `.hex` form. Django user creation is `create_user(email,
  password)` (2-arg custom user model).

# 13. Decision Log

Append date, decision, evidence, alternatives, consequence, reversal, security, license, and compatibility impact.

# 14. Outcomes & Retrospective

At completion record changed files versus the machine fence, exact commands and observed sentinels, test and proof evidence, assumptions confirmed or changed, provider and hardware status, remaining risks, and the green tag.
