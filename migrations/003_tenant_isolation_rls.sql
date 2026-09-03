-- RX-005 migration 003: tenant isolation at the database boundary and
-- tenant-bound embedding FK (AUD-007).
-- Additive only: enables RLS and adds policies/constraints; never alters or
-- drops prior schema. PostgreSQL 18.4 (COMPONENT_REGISTRY.yaml,
-- VERSIONS.lock.yaml).
--
-- AUD-007 root cause: tenant isolation was application convention only (a
-- WHERE tenant_id in every query) and memory_embeddings.tenant_id bound only
-- memory_id, so an embedding row could reference a memory record of a
-- different tenant. This migration makes isolation a database-enforced
-- property:
--   1. ROW LEVEL SECURITY on every EP-004 table, with a policy that filters
--      on the session tenant claim (set by the PostgreSQL adapters via
--      SELECT set_config('app.tenant_id', $1, true)).
--   2. A composite FK (tenant_id, memory_id) -> memory_records(tenant_id,
--      memory_id) so an embedding can never cross tenant boundaries.
--
-- PostgreSQL lacks IF NOT EXISTS for policies and constraints, so each
-- addition is guarded by an idempotent DO block (the migration must survive
-- being applied twice - see ep004_integration_migrations_are_idempotent).

-- 1. Session tenant claim. Adapters set this per connection before any
--    statement; the RLS policies below read it. When unset (NULL) or
--    expired (the '' placeholder a transaction-local SET leaves behind),
--    NULLIF turns it into NULL and the policy denies everything (fail
--    closed). An unguarded `current_setting(...)::uuid` would raise
--    E22P02 on '' instead of denying rows - failing loud, not closed.
ALTER TABLE memory_records ENABLE ROW LEVEL SECURITY;
ALTER TABLE memory_records FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'memory_records'
          AND policyname = 'memory_records_tenant_isolation'
    ) THEN
        EXECUTE 'CREATE POLICY memory_records_tenant_isolation ON memory_records
            USING (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid)';
    END IF;
END
$$;

ALTER TABLE world_graph_edges ENABLE ROW LEVEL SECURITY;
ALTER TABLE world_graph_edges FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'world_graph_edges'
          AND policyname = 'world_graph_edges_tenant_isolation'
    ) THEN
        EXECUTE 'CREATE POLICY world_graph_edges_tenant_isolation ON world_graph_edges
            USING (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid)';
    END IF;
END
$$;

ALTER TABLE memory_embeddings ENABLE ROW LEVEL SECURITY;
ALTER TABLE memory_embeddings FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'memory_embeddings'
          AND policyname = 'memory_embeddings_tenant_isolation'
    ) THEN
        EXECUTE 'CREATE POLICY memory_embeddings_tenant_isolation ON memory_embeddings
            USING (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid)';
    END IF;
END
$$;

-- 2. Tenant-bound composite FK. The FK guarantees an embedding row's
--    tenant_id matches its parent memory record's tenant_id. A unique
--    constraint on the referenced columns is required by PostgreSQL even
--    though memory_id alone is already unique.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'memory_records_tenant_memory_unique'
    ) THEN
        ALTER TABLE memory_records
            ADD CONSTRAINT memory_records_tenant_memory_unique
            UNIQUE (tenant_id, memory_id);
    END IF;
END
$$;

ALTER TABLE memory_embeddings
    DROP CONSTRAINT IF EXISTS memory_embeddings_memory_id_fkey;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'memory_embeddings_tenant_memory_fkey'
    ) THEN
        ALTER TABLE memory_embeddings
            ADD CONSTRAINT memory_embeddings_tenant_memory_fkey
            FOREIGN KEY (tenant_id, memory_id)
            REFERENCES memory_records (tenant_id, memory_id)
            ON DELETE CASCADE;
    END IF;
END
$$;

-- 3. Canonical sensitivity rank helper used by the memory repository's
--    max_sensitivity filter (declaration order: PUBLIC lowest, SECRET
--    highest; matches `Sensitivity` in nexus-data).
DO $block$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_proc WHERE proname = 'sensitivity_rank'
    ) THEN
        EXECUTE $func$CREATE FUNCTION sensitivity_rank(s TEXT) RETURNS INTEGER AS $body$
            SELECT CASE s
                WHEN 'PUBLIC' THEN 0
                WHEN 'HOUSEHOLD' THEN 1
                WHEN 'PERSONAL' THEN 2
                WHEN 'SENSITIVE' THEN 3
                WHEN 'BUSINESS_CONFIDENTIAL' THEN 4
                WHEN 'SECURITY' THEN 5
                WHEN 'SECRET' THEN 6
                ELSE 99
            END
        $body$ LANGUAGE SQL IMMUTABLE$func$;
    END IF;
END
$block$;
