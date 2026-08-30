# RX-000 Gap Log - logged first, fixed one at a time, reported after each

## GAP-001 (RESOLVED) - Authoritative AUD-001...AUD-065 register was unavailable locally

**Status:** RESOLVED 2026-08-29
**Severity:** was BLOCKING

**Resolution:** Dominic provided the audit source: ChatGPT share
https://chatgpt.com/share/6a926876-0c84-83e8-a9da-4f3d53dd1ddc ("Audit Nexus Repository").
The full conversation was extracted (React Router single-flight stream decoded) and
all 90 findings imported verbatim into `register_data.py` / `AUDIT_FINDINGS.tsv`:

- AUD-001...006 from the master audit report
- AUD-007...012 compute-fabric continuation
- AUD-013...026 EP-037 storage/DR + communications + Sentinel continuation
- AUD-027...041 EP-030/031 Sentinel + client continuation
- AUD-042...065 setup/bootstrap + storage + observability + supply-chain + EP-040/041 continuation
- AUD-066...090 EP-042 update path / EP-043 / EP-044 continuation

Cumulative severities match the audit exactly: P0 0, P1 72, P2 18 (total 90).
Repair-node ownership: sec.12 of the remediation graph (AUD-066...090) + RX-node
ownership language (AUD-001...065). All rows OPEN; verifier green.

**Verifier:** `.agent/remediation/verify-remediation-register.sh` -> PASS
(90/90 registered, quarantine active: generation 2, release not allowed).

## GAP-002 (RX-005) - EP-004/EP-005/EP-006 persistence and retry truth (AUD-007, AUD-008, AUD-023)

**Status:** LOGGED 2026-08-30 — fix ONE at a time from top severity; report after each.

### GAP-002a (AUD-007, P1) - EP-004 closed without production PostgreSQL repository / UnitOfWork / pgvector adapters
- Only `MemoryRepository` / `WorldGraphRepository` / `VectorRepository` / `UnitOfWork` traits exist in `nexus-data`.
- No concrete PostgreSQL implementations anywhere; `integration_postgres.rs` drives raw SQL, bypassing production abstractions.
- Tenant isolation is application convention, not DB RLS (`memory_records`/`world_graph_edges` have no `ENABLE ROW LEVEL SECURITY`).
- `memory_embeddings` FK binds only `memory_id`; tenant_id is not part of a composite FK to `memory_records`.

### GAP-002b (AUD-008, P1) - EP-005 NATS checkpoint persistence is a no-op; outbox/inbox absent
- `NatsEventConsumer::checkpoint()` always returns `Ok(None)`; `save_checkpoint()` is a no-op.
- `poll()` names the durable consumer `{consumer}-{after_sequence}` — durable identity changes per poll, defeating durability.
- No `OutboxRepository` / `InboxRepository` PostgreSQL implementations; the restart proof polls once and manually changes sequence.

### GAP-002c (AUD-023, P2) - Temporal does not enforce permanent/transient retry classification
- `toTemporalRetry()` maps only backoff/maximumAttempts; never supplies `nonRetryableErrorTypes` or non-retryable `ApplicationFailure`.
- Permanent failures (VALIDATION/POLICY/AUTH) get up to five attempts.
