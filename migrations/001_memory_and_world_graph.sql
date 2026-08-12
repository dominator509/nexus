-- EP-004 M3 migration 001: canonical memory and world graph state.
-- Additive only: creates new tables/indexes; never alters or drops prior
-- schema. PostgreSQL 18.4 (COMPONENT_REGISTRY.yaml, VERSIONS.lock.yaml).
-- INV-004: PostgreSQL is the durable truth; vector indexes are projections.

-- Memory records (SPEC-002 behavior 4, schema memory-record.schema.json).
CREATE TABLE IF NOT EXISTS memory_records (
    memory_id      UUID PRIMARY KEY,
    tenant_id      UUID NOT NULL,
    namespace      TEXT NOT NULL,
    memory_type    TEXT NOT NULL,
    content        JSONB NOT NULL,
    content_hash   TEXT NOT NULL CHECK (content_hash ~ '^[a-f0-9]{64}$'),
    source         TEXT NOT NULL,
    actor          TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL,
    observed_at    TIMESTAMPTZ NOT NULL,
    confidence     DOUBLE PRECISION NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    sensitivity    TEXT NOT NULL,
    purpose        TEXT NOT NULL,
    retention      TEXT NOT NULL,
    status         TEXT NOT NULL,
    derived_from   UUID[] NOT NULL DEFAULT '{}',
    supersedes     UUID,
    embedding_ref  TEXT,
    -- Multi-tenant isolation: every query must carry tenant_id.
    CONSTRAINT memory_records_tenant_status CHECK (status IN
        ('PROPOSED', 'ACTIVE', 'SUPERSEDED', 'REJECTED', 'DELETED')),
    CONSTRAINT memory_records_supersedes_self CHECK (supersedes IS NULL OR supersedes <> memory_id)
);

-- Tenant-scoped lookups.
CREATE INDEX IF NOT EXISTS memory_records_tenant_status_idx
    ON memory_records (tenant_id, status);
CREATE INDEX IF NOT EXISTS memory_records_tenant_namespace_idx
    ON memory_records (tenant_id, namespace);
CREATE INDEX IF NOT EXISTS memory_records_tenant_observed_idx
    ON memory_records (tenant_id, observed_at DESC);
CREATE INDEX IF NOT EXISTS memory_records_tenant_type_idx
    ON memory_records (tenant_id, memory_type);
-- Full-text retrieval aid (SPEC-002 behavior 6).
CREATE INDEX IF NOT EXISTS memory_records_content_fts_idx
    ON memory_records USING GIN (to_tsvector('simple', content::text));

-- World graph adjacency (SPEC-002 behavior 7; EP-004 fallback doctrine:
-- PostgreSQL recursive queries and adjacency tables only, INV-015).
CREATE TABLE IF NOT EXISTS world_graph_edges (
    tenant_id   UUID NOT NULL,
    from_node   UUID NOT NULL,
    to_node     UUID NOT NULL,
    edge_type   TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, from_node, to_node, edge_type)
);

CREATE INDEX IF NOT EXISTS world_graph_edges_tenant_to_idx
    ON world_graph_edges (tenant_id, to_node);
