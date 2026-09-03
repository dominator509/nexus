-- RX-005 AUD-008 migration 004: transactional outbox + idempotent inbox.
-- SPEC-023 behaviors 1 and 4. Additive only.
--
-- AUD-008 root cause: OutboxRepository / InboxRepository ports existed in
-- nexus-events but had no PostgreSQL implementations anywhere; the event
-- nervous system had no durable transactional outbox and no idempotent
-- consumer inbox. This migration creates the two ledger tables.
--
-- Both tables are platform-level ledgers (not tenant domain data), so RLS
-- from migration 003 deliberately does not apply: the publisher's
-- fetch_pending is cross-tenant by design.

CREATE TABLE IF NOT EXISTS outbox (
    outbox_id   TEXT PRIMARY KEY,
    envelope    JSONB NOT NULL,
    status      TEXT NOT NULL CHECK (status IN ('PENDING','PUBLISHING','PUBLISHED','FAILED')),
    attempts    INTEGER NOT NULL DEFAULT 0,
    last_error  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Publisher scan order: oldest pending first, bounded batch.
CREATE INDEX IF NOT EXISTS outbox_pending_idx
    ON outbox (status, created_at);

CREATE TABLE IF NOT EXISTS inbox (
    consumer    TEXT NOT NULL,
    event_id    TEXT NOT NULL,
    status      TEXT NOT NULL CHECK (status IN ('NEW','PROCESSING','DONE','FAILED')),
    attempts    INTEGER NOT NULL DEFAULT 0,
    last_error  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (consumer, event_id)
);

-- Dedup lookup + per-consumer retry scan.
CREATE INDEX IF NOT EXISTS inbox_consumer_pending_idx
    ON inbox (consumer, status, created_at);
