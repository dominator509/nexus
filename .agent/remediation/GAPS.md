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

**NATS consumer portion: RESOLVED 2026-08-30 (VERIFIED_FIXED, commit pending)**
- `checkpoint()`/`save_checkpoint()` now persist to a real JetStream KV bucket
  (`nexus_checkpoints`, keyed by consumer name) - durable, survives restart;
  `checkpoint()` reads the stored checkpoint, `save_checkpoint()` writes it.
- `poll()` now creates an EPHEMERAL per-call pull consumer positioned by the
  application-owned `after_sequence` (DeliverAll for 0). The pre-fix durable
  consumer per sequence (`{consumer}-{after_sequence}`) accumulated unbounded
  server-side state and defeated durability; a single stable durable consumer
  would track its own position and ignore the checkpoint, so it is avoided by
  design (at-least-once + inbox dedup, matching SPEC-023 behavior 4).
- Proof (live-fire `nats:2.14.3`): integration + failure suites 19/19; new
  tests prove checkpoint round-trip equality, overwrite advance, persistence
  across a fresh connection, None for unsaved consumers, and resume-after-
  checkpoint skipping processed events.

**Still LOGGED - GAP-002b2 (AUD-008 remainder): Outbox/Inbox PostgreSQL implementations absent**
- `OutboxRepository` / `InboxRepository` ports exist in `nexus-events`
  (SPEC-023 behaviors 1 & 4); no PostgreSQL implementations anywhere.
- The restart proof now exercises the real checkpoint (no manual sequence
  fiddling), but transactional outbox/inbox persistence in `nexus-pg` is a
  distinct piece of work - scope decision with Dominic before implementation.

**GAP-002b2: RESOLVED 2026-08-30 (VERIFIED_FIXED, commit pending)**
- Port corrected: `OutboxRepository::append` dropped its unimplementable
  `&mut dyn UnitOfWork` parameter (the trait exposes no statement
  execution; no caller existed). Atomicity is expressed by binding the
  repository to the same `PgUnitOfWork` as the domain repositories.
- Migration 004: `outbox` + `inbox` tables (idempotent, status CHECK
  constraints, scan indexes). Platform-level ledgers - deliberately no
  tenant RLS (publisher scan is cross-tenant by design).
- `PgOutboxRepository`: append (PENDING), fetch_pending (PENDING+FAILED,
  oldest first, in-flight PUBLISHING excluded), mark_publishing/
  mark_published/mark_failed (idempotent per row, Conflict on missing,
  attempts incremented on failure - bounded retry).
- `PgInboxRepository`: record_delivery deduplicates via ON CONFLICT DO
  NOTHING (first sighting true, replay false), mark_done/mark_failed,
  fetch_new (NEW+FAILED per consumer).
- `PgUnitOfWork::with_tx` generalized over the closure error type
  (`E: From<DataError>`); `From<DataError> for EventError` added to
  nexus-events preserving the SPEC-006 code ladder and correlation.
- Proof (live-fire `pgvector/pgvector:pg18`): atomicity both ways (domain
  write + append commit together, roll back together), publisher lifecycle
  (pending -> publishing excluded -> published; failed retried with
  attempts), inbox dedup + lifecycle + consumer isolation, migration
  idempotency covers the new tables. nexus-pg 14/14; data+events+pg
  52/52; workspace check clean (0 errors, 0 warnings).

### GAP-002c (AUD-023, P2) - Temporal does not enforce permanent/transient retry classification
- `toTemporalRetry()` maps only backoff/maximumAttempts; never supplies `nonRetryableErrorTypes` or non-retryable `ApplicationFailure`.
- Permanent failures (VALIDATION/POLICY/AUTH) get up to five attempts.
