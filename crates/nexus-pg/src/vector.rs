//! PostgreSQL pgvector repository (EP-004 M3, RX-005 AUD-007).
//!
//! Concrete `VectorRepository` on pgvector. The vector index is a
//! retrieval aid, never the source of truth (SPEC-002 behavior 2). Rows
//! carry tenant_id and the composite FK from migration 003 guarantees an
//! embedding can never reference a memory record of another tenant.

use nexus_data::{DataError, DataErrorCode, MemoryCandidate, VectorRepository};
use nexus_domain::{NexusId, TenantId};
use postgres::Client;
use uuid::Uuid;

use crate::unit_of_work::PgUnitOfWork;

/// PostgreSQL/pgvector implementation of the vector repository port.
pub struct PgVectorRepository<'a> {
    uow: &'a PgUnitOfWork,
}

impl<'a> PgVectorRepository<'a> {
    /// Bind the repository to a live unit of work.
    pub fn new(uow: &'a PgUnitOfWork) -> Self {
        Self { uow }
    }

    fn set_tenant(tx: &mut Client, tenant: &TenantId) -> Result<(), DataError> {
        tx.execute(
            "SELECT set_config('app.tenant_id', $1, true)",
            &[&tenant.as_str()],
        )
        .map_err(|e| {
            DataError::new(
                DataErrorCode::ExternalProvider,
                format!("postgres set tenant: {e}"),
            )
        })?;
        Ok(())
    }

    fn uuid(id: &str) -> Result<Uuid, DataError> {
        Uuid::parse_str(id)
            .map_err(|e| DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}")))
    }
}

impl VectorRepository for PgVectorRepository<'_> {
    fn upsert_vector(
        &mut self,
        tenant: TenantId,
        memory_id: NexusId,
        embedding: Vec<f32>,
    ) -> Result<(), DataError> {
        let mid = Self::uuid(memory_id.as_str())?;
        let tenant_uuid = Self::uuid(tenant.as_str())?;
        let vector_literal = format!(
            "[{}]",
            embedding
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            // Model/dimensions are fixed by the canonical embedding contract
            // (minilm, 384 dims; VERSIONS.lock.yaml). The FK guarantees the
            // parent memory record exists and belongs to this tenant.
            tx.execute(
                "INSERT INTO memory_embeddings (memory_id, tenant_id, model, dimensions, model_version, embedding)
                 VALUES ($1, $2, 'minilm', 384, 'v1', $3::text::vector)
                 ON CONFLICT (memory_id) DO UPDATE
                   SET embedding = EXCLUDED.embedding, model_version = EXCLUDED.model_version",
                &[&mid, &tenant_uuid, &vector_literal],
            )
            .map_err(|e| {
                DataError::new(
                    DataErrorCode::ExternalProvider,
                    format!("postgres upsert vector: {e}"),
                )
            })?;
            Ok(())
        })
    }

    fn nearest(
        &mut self,
        tenant: TenantId,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<MemoryCandidate>, DataError> {
        let tenant_uuid = Self::uuid(tenant.as_str())?;
        let vector_literal = format!(
            "[{}]",
            embedding
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            let rows = tx
                .query(
                    "SELECT e.memory_id, e.tenant_id, m.namespace, m.memory_type, m.content,
                            m.content_hash, m.source, m.actor,
                            to_char(m.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at,
                            to_char(m.observed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS observed_at,
                            m.confidence, m.sensitivity, m.purpose, m.retention, m.status,
                            m.derived_from, m.supersedes, m.embedding_ref,
                            1 - (e.embedding <=> $2::text::vector) AS cosine
                     FROM memory_embeddings e
                     JOIN memory_records m ON m.memory_id = e.memory_id AND m.tenant_id = e.tenant_id
                     WHERE e.tenant_id = $1
                     ORDER BY cosine DESC
                     LIMIT $3",
                    &[&tenant_uuid, &vector_literal, &(limit as i64)],
                )
                .map_err(|e| {
                    DataError::new(
                        DataErrorCode::ExternalProvider,
                        format!("postgres vector nearest: {e}"),
                    )
                })?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let memory_id: Uuid = row.get("memory_id");
                let tenant_id: Uuid = row.get("tenant_id");
                let retention_text: String = row.get("retention");
                let retention = parse_retention(&retention_text)?;
                let embedding_ref: Option<String> = row.get("embedding_ref");
                let embedding_ref = match embedding_ref {
                    Some(s) => Some(serde_json::from_str(&s).map_err(|e| {
                        DataError::new(
                            DataErrorCode::Invariant,
                            format!("corrupt embedding_ref in store: {e}"),
                        )
                    })?),
                    None => None,
                };
                let derived: Vec<Uuid> = row.get("derived_from");
                let supersedes: Option<Uuid> = row.get("supersedes");
                let record = nexus_data::MemoryRecord {
                    memory_id: NexusId::new(memory_id.to_string()).map_err(|e| {
                        DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}"))
                    })?,
                    tenant_id: TenantId::new(tenant_id.to_string()).map_err(|e| {
                        DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}"))
                    })?,
                    namespace: row.get("namespace"),
                    memory_type: row
                        .get::<_, String>("memory_type")
                        .parse()
                        .map_err(|e| {
                            DataError::new(
                                DataErrorCode::Invariant,
                                format!("corrupt memory_type: {e}"),
                            )
                        })?,
                    content: row.get("content"),
                    content_hash: row.get("content_hash"),
                    source: row.get("source"),
                    actor: row.get("actor"),
                    created_at: row.get("created_at"),
                    observed_at: row.get("observed_at"),
                    confidence: row.get("confidence"),
                    sensitivity: row
                        .get::<_, String>("sensitivity")
                        .parse()
                        .map_err(|e| {
                            DataError::new(
                                DataErrorCode::Invariant,
                                format!("corrupt sensitivity: {e}"),
                            )
                        })?,
                    purpose: row.get("purpose"),
                    retention,
                    status: row
                        .get::<_, String>("status")
                        .parse()
                        .map_err(|e| {
                            DataError::new(DataErrorCode::Invariant, format!("corrupt status: {e}"))
                        })?,
                    derived_from: derived
                        .into_iter()
                        .map(|u| NexusId::new(u.to_string()))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| {
                            DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}"))
                        })?,
                    supersedes: supersedes
                        .map(|u| NexusId::new(u.to_string()))
                        .transpose()
                        .map_err(|e| {
                            DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}"))
                        })?,
                    embedding_ref,
                };
                out.push(MemoryCandidate {
                    record,
                    score: row.get("cosine"),
                });
            }
            Ok(out)
        })
    }

    fn remove(&mut self, tenant: TenantId, memory_id: NexusId) -> Result<(), DataError> {
        let mid = Self::uuid(memory_id.as_str())?;
        let tenant_uuid = Self::uuid(tenant.as_str())?;
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            tx.execute(
                "DELETE FROM memory_embeddings WHERE memory_id = $1 AND tenant_id = $2",
                &[&mid, &tenant_uuid],
            )
            .map_err(|e| {
                DataError::new(
                    DataErrorCode::ExternalProvider,
                    format!("postgres vector remove: {e}"),
                )
            })?;
            Ok(())
        })
    }
}

fn parse_retention(text: &str) -> Result<nexus_data::RetentionPolicy, DataError> {
    if text == "INDEFINITE" {
        return Ok(nexus_data::RetentionPolicy::indefinite());
    }
    let mut parts = text.split_whitespace();
    let unit = parts
        .next()
        .ok_or_else(|| DataError::new(DataErrorCode::Invariant, "corrupt retention in store"))?;
    let value = parts
        .next()
        .ok_or_else(|| DataError::new(DataErrorCode::Invariant, "corrupt retention in store"))?;
    let unit = match unit {
        "Hours" => nexus_data::RetentionUnit::Hours,
        "Days" => nexus_data::RetentionUnit::Days,
        "Weeks" => nexus_data::RetentionUnit::Weeks,
        "Months" => nexus_data::RetentionUnit::Months,
        "Years" => nexus_data::RetentionUnit::Years,
        other => {
            return Err(DataError::new(
                DataErrorCode::Invariant,
                format!("corrupt retention unit: {other}"),
            ));
        }
    };
    let value = value
        .parse::<u32>()
        .map_err(|_| DataError::new(DataErrorCode::Invariant, "corrupt retention value"))?;
    Ok(nexus_data::RetentionPolicy::for_duration(unit, value))
}
