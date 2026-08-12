-- EP-004 M3 migration 002: pgvector index (SPEC-002 behavior 2).
-- Additive only. Requires the pgvector extension (COMPONENT_REGISTRY.yaml,
-- VERSIONS.lock.yaml pgvector 0.8.6). The vector index is a retrieval aid,
-- never the source of truth (INV-004, SPEC-002 behavior 2).

CREATE EXTENSION IF NOT EXISTS vector;

-- Versioned embeddings per row: model and dimensions are recorded so a
-- model upgrade can re-embed without losing provenance.
CREATE TABLE IF NOT EXISTS memory_embeddings (
    memory_id     UUID PRIMARY KEY REFERENCES memory_records (memory_id) ON DELETE CASCADE,
    tenant_id     UUID NOT NULL,
    model         TEXT NOT NULL,
    dimensions    INTEGER NOT NULL,
    model_version TEXT NOT NULL,
    embedding     vector(384) NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS memory_embeddings_tenant_idx
    ON memory_embeddings (tenant_id, model);

-- Tenant-scoped nearest-neighbor search support.
CREATE INDEX IF NOT EXISTS memory_embeddings_vector_idx
    ON memory_embeddings USING hnsw (embedding vector_cosine_ops);
