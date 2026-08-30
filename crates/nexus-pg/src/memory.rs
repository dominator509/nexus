//! PostgreSQL memory repository (EP-004 M3, RX-005 AUD-007).
//!
//! Concrete `MemoryRepository` on real PostgreSQL. Every operation:
//! 1. Sets the `app.tenant_id` session claim (transaction-local) so the
//!    RLS policies from migration 003 enforce isolation at the database
//!    boundary - a statement that forgot its tenant filter is denied.
//! 2. Also carries `tenant_id` in every WHERE/INSERT explicitly.
//!
//! Timestamps round-trip as RFC 3339 strings (the canonical wire form of
//! `MemoryRecord`) via `::timestamptz` casts and `to_char(... AT TIME ZONE
//! 'UTC')` on read. `embedding_ref` round-trips as canonical JSON in the
//! TEXT column (model/dimensions/version are structured in the model).

use std::str::FromStr;

use nexus_data::{
    DataError, DataErrorCode, EmbeddingRef, MemoryCandidate, MemoryProposal, MemoryQuery,
    MemoryRecord, MemoryRepository, MemoryStatus, RetentionPolicy, RetentionUnit, Sensitivity,
};
use nexus_domain::{MemoryType, NexusId, TenantId};
use postgres::Client;
use uuid::Uuid;

use crate::unit_of_work::PgUnitOfWork;

/// PostgreSQL implementation of the memory repository port.
pub struct PgMemoryRepository<'a> {
    uow: &'a PgUnitOfWork,
}

impl<'a> PgMemoryRepository<'a> {
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

    fn map_row(row: &postgres::Row) -> Result<MemoryRecord, DataError> {
        let retention_text: String = row.get("retention");
        let retention = parse_retention(&retention_text)?;
        let embedding_ref: Option<String> = row.get("embedding_ref");
        let embedding_ref = match embedding_ref {
            Some(s) => Some(serde_json::from_str::<EmbeddingRef>(&s).map_err(|e| {
                DataError::new(
                    DataErrorCode::Invariant,
                    format!("corrupt embedding_ref in store: {e}"),
                )
            })?),
            None => None,
        };
        let derived: Vec<Uuid> = row.get("derived_from");
        let supersedes: Option<Uuid> = row.get("supersedes");
        Ok(MemoryRecord {
            memory_id: NexusId::new(row.get::<_, Uuid>("memory_id").to_string()).map_err(|e| {
                DataError::new(DataErrorCode::Invariant, format!("corrupt memory_id: {e}"))
            })?,
            tenant_id: TenantId::new(row.get::<_, Uuid>("tenant_id").to_string()).map_err(|e| {
                DataError::new(DataErrorCode::Invariant, format!("corrupt tenant_id: {e}"))
            })?,
            namespace: row.get("namespace"),
            memory_type: MemoryType::from_str(&row.get::<_, String>("memory_type")).map_err(|e| {
                DataError::new(DataErrorCode::Invariant, format!("corrupt memory_type: {e}"))
            })?,
            content: row.get("content"),
            content_hash: row.get("content_hash"),
            source: row.get("source"),
            actor: row.get("actor"),
            created_at: row.get("created_at"),
            observed_at: row.get("observed_at"),
            confidence: row.get("confidence"),
            sensitivity: Sensitivity::from_str(&row.get::<_, String>("sensitivity")).map_err(
                |e| {
                    DataError::new(
                        DataErrorCode::Invariant,
                        format!("corrupt sensitivity: {e}"),
                    )
                },
            )?,
            purpose: row.get("purpose"),
            retention,
            status: MemoryStatus::from_str(&row.get::<_, String>("status")).map_err(|e| {
                DataError::new(DataErrorCode::Invariant, format!("corrupt status: {e}"))
            })?,
            derived_from: derived.into_iter().map(|u| NexusId::new(u.to_string())).collect::<Result<Vec<_>, _>>().map_err(|e| {
                DataError::new(DataErrorCode::Invariant, format!("corrupt derived_from: {e}"))
            })?,
            supersedes: supersedes.map(|u| NexusId::new(u.to_string())).transpose().map_err(|e| {
                DataError::new(DataErrorCode::Invariant, format!("corrupt supersedes: {e}"))
            })?,
            embedding_ref,
        })
    }
}

fn parse_retention(text: &str) -> Result<RetentionPolicy, DataError> {
    if text == "INDEFINITE" {
        return Ok(RetentionPolicy::indefinite());
    }
    let mut parts = text.split_whitespace();
    let unit = parts.next().ok_or_else(|| {
        DataError::new(DataErrorCode::Invariant, "corrupt retention in store")
    })?;
    let value = parts.next().ok_or_else(|| {
        DataError::new(DataErrorCode::Invariant, "corrupt retention in store")
    })?;
    let unit = match unit {
        "Hours" => RetentionUnit::Hours,
        "Days" => RetentionUnit::Days,
        "Weeks" => RetentionUnit::Weeks,
        "Months" => RetentionUnit::Months,
        "Years" => RetentionUnit::Years,
        other => {
            return Err(DataError::new(
                DataErrorCode::Invariant,
                format!("corrupt retention unit: {other}"),
            ))
        }
    };
    let value = value.parse::<u32>().map_err(|_| {
        DataError::new(DataErrorCode::Invariant, "corrupt retention value in store")
    })?;
    Ok(RetentionPolicy::for_duration(unit, value))
}

/// Parse a canonical ID string (from `NexusId::as_str` or
/// `TenantId::as_str`) into a UUID.
fn uuid_param(id: &str) -> Result<Uuid, DataError> {
    Uuid::parse_str(id)
        .map_err(|e| DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}")))
}

impl MemoryRepository for PgMemoryRepository<'_> {
    fn propose(&mut self, tenant: TenantId, proposal: MemoryProposal) -> Result<(), DataError> {
        let record = &proposal.record;
        record.validate()?;
        if record.tenant_id != tenant {
            return Err(DataError::new(
                DataErrorCode::Authorization,
                "proposal tenant does not match repository tenant",
            ));
        }
        let memory_id = uuid_param(record.memory_id.as_str())?;
        let tenant_uuid = uuid_param(tenant.as_str())?;
        let derived: Vec<Uuid> = record
            .derived_from
            .iter()
            .map(|id| uuid_param(id.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let supersedes: Option<Uuid> = record
            .supersedes
            .as_ref()
            .map(|id| uuid_param(id.as_str()))
            .transpose()?;
        let embedding_ref = record
            .embedding_ref
            .as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()
            .map_err(|e| DataError::new(DataErrorCode::Validation, format!("json: {e}")))?;
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            tx.execute(
                "INSERT INTO memory_records (
                    memory_id, tenant_id, namespace, memory_type, content, content_hash,
                    source, actor, created_at, observed_at, confidence, sensitivity,
                    purpose, retention, status, derived_from, supersedes, embedding_ref
                 ) VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7, $8, $9::text::timestamptz,
                    $10::text::timestamptz, $11, $12, $13, $14, $15, $16::uuid[], $17::uuid,
                    $18)",
                &[
                    &memory_id,
                    &tenant_uuid,
                    &record.namespace,
                    &record.memory_type.as_str(),
                    &record.content,
                    &record.content_hash,
                    &record.source,
                    &record.actor,
                    &record.created_at,
                    &record.observed_at,
                    &record.confidence,
                    &record.sensitivity.as_str(),
                    &record.purpose,
                    &record.retention.to_string(),
                    &MemoryStatus::Proposed.as_str(),
                    &derived,
                    &supersedes,
                    &embedding_ref,
                ],
            )
            .map_err(|e| {
                DataError::new(
                    DataErrorCode::ExternalProvider,
                    format!("postgres propose: {e}"),
                )
            })?;
            Ok(())
        })
    }

    fn activate(&mut self, tenant: TenantId, memory_id: NexusId) -> Result<(), DataError> {
        let mid = uuid_param(memory_id.as_str())?;
        let tenant_uuid = uuid_param(tenant.as_str())?;
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            let n = tx
                .execute(
                    "UPDATE memory_records SET status = 'ACTIVE'
                     WHERE memory_id = $1 AND tenant_id = $2 AND status = 'PROPOSED'",
                    &[&mid, &tenant_uuid],
                )
                .map_err(|e| {
                    DataError::new(
                        DataErrorCode::ExternalProvider,
                        format!("postgres activate: {e}"),
                    )
                })?;
            if n == 0 {
                return Err(DataError::new(
                    DataErrorCode::Conflict,
                    "memory record not found or not PROPOSED",
                ));
            }
            Ok(())
        })
    }

    fn get(&mut self, tenant: TenantId, memory_id: NexusId) -> Result<MemoryRecord, DataError> {
        let mid = uuid_param(memory_id.as_str())?;
        let tenant_uuid = uuid_param(tenant.as_str())?;
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            let row = tx
                .query_opt(
                    "SELECT memory_id, tenant_id, namespace, memory_type, content,
                            content_hash, source, actor,
                            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at,
                            to_char(observed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS observed_at,
                            confidence, sensitivity, purpose, retention, status,
                            derived_from, supersedes, embedding_ref
                     FROM memory_records
                     WHERE memory_id = $1 AND tenant_id = $2",
                    &[&mid, &tenant_uuid],
                )
                .map_err(|e| {
                    DataError::new(
                        DataErrorCode::ExternalProvider,
                        format!("postgres get: {e}"),
                    )
                })?;
            match row {
                Some(r) => Self::map_row(&r),
                None => Err(DataError::new(DataErrorCode::Conflict, "memory record not found")),
            }
        })
    }

    fn query(
        &mut self,
        tenant: TenantId,
        query: &MemoryQuery,
    ) -> Result<Vec<MemoryCandidate>, DataError> {
        let tenant_uuid = uuid_param(tenant.as_str())?;
        let mut sql = String::from(
            "SELECT memory_id, tenant_id, namespace, memory_type, content,
                    content_hash, source, actor,
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at,
                    to_char(observed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS observed_at,
                    confidence, sensitivity, purpose, retention, status,
                    derived_from, supersedes, embedding_ref
             FROM memory_records WHERE tenant_id = $1",
        );
        let mut params: Vec<Box<dyn postgres::types::ToSql + Sync>> =
            vec![Box::new(tenant_uuid)];
        let mut n = 2i32;
        if let Some(ns) = &query.namespace {
            sql.push_str(&format!(" AND namespace = ${n}"));
            params.push(Box::new(ns.clone()));
            n += 1;
        }
        if let Some(mt) = &query.memory_type {
            sql.push_str(&format!(" AND memory_type = ${n}"));
            params.push(Box::new(mt.as_str().to_string()));
            n += 1;
        }
        if let Some(sens) = &query.max_sensitivity {
            // Filter to sensitivities at or below the ceiling. The ranking
            // ladder is the canonical declaration order of `Sensitivity`.
            let ceiling = sensitivity_rank(*sens);
            sql.push_str(&format!(
                " AND sensitivity IN ('PUBLIC','HOUSEHOLD','PERSONAL','SENSITIVE','BUSINESS_CONFIDENTIAL','SECURITY','SECRET')"
            ));
            sql.push_str(&format!(" AND sensitivity_rank(sensitivity) <= ${n}"));
            params.push(Box::new(ceiling));
            n += 1;
        }
        if let Some(st) = &query.status {
            sql.push_str(&format!(" AND status = ${n}"));
            params.push(Box::new(st.as_str().to_string()));
            n += 1;
        }
        if let Some(text) = &query.text {
            sql.push_str(&format!(
                " AND to_tsvector('simple', content::text) @@ plainto_tsquery('simple', ${n})"
            ));
            params.push(Box::new(text.clone()));
            n += 1;
        }
        if let Some(after) = &query.observed_after {
            sql.push_str(&format!(" AND observed_at >= ${n}::text::timestamptz"));
            params.push(Box::new(after.clone()));
            n += 1;
        }
        sql.push_str(&format!(" ORDER BY observed_at DESC LIMIT ${n}"));
        params.push(Box::new(query.limit as i64));
        let refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            let rows = tx.query(&sql, &refs).map_err(|e| {
                DataError::new(
                    DataErrorCode::ExternalProvider,
                    format!("postgres query: {e}"),
                )
            })?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let record = Self::map_row(&row)?;
                // Provider-defined score: confidence in [0,1] (deterministic
                // blend; the memory behavior layer applies recency/diversity
                // policy on top of the candidate set).
                out.push(MemoryCandidate {
                    record,
                    score: row.get::<_, f64>("confidence"),
                });
            }
            Ok(out)
        })
    }

    fn delete(&mut self, tenant: TenantId, memory_id: NexusId) -> Result<(), DataError> {
        let mid = uuid_param(memory_id.as_str())?;
        let tenant_uuid = uuid_param(tenant.as_str())?;
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            let n = tx
                .execute(
                    "UPDATE memory_records SET status = 'DELETED'
                     WHERE memory_id = $1 AND tenant_id = $2 AND status <> 'DELETED'",
                    &[&mid, &tenant_uuid],
                )
                .map_err(|e| {
                    DataError::new(
                        DataErrorCode::ExternalProvider,
                        format!("postgres delete: {e}"),
                    )
                })?;
            if n == 0 {
                return Err(DataError::new(
                    DataErrorCode::Conflict,
                    "memory record not found or already deleted",
                ));
            }
            Ok(())
        })
    }

    fn supersede(
        &mut self,
        tenant: TenantId,
        old_id: NexusId,
        new_record: MemoryRecord,
    ) -> Result<(), DataError> {
        new_record.validate()?;
        if new_record.tenant_id != tenant {
            return Err(DataError::new(
                DataErrorCode::Authorization,
                "new record tenant does not match repository tenant",
            ));
        }
        let old_uuid = uuid_param(old_id.as_str())?;
        let new_uuid = uuid_param(new_record.memory_id.as_str())?;
        let tenant_uuid = uuid_param(tenant.as_str())?;
        let derived: Vec<Uuid> = new_record
            .derived_from
            .iter()
            .map(|id| uuid_param(id.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let supersedes: Option<Uuid> = new_record
            .supersedes
            .as_ref()
            .map(|id| uuid_param(id.as_str()))
            .transpose()?;
        let embedding_ref = new_record
            .embedding_ref
            .as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()
            .map_err(|e| DataError::new(DataErrorCode::Validation, format!("json: {e}")))?;
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            let n = tx
                .execute(
                    "UPDATE memory_records SET status = 'SUPERSEDED'
                     WHERE memory_id = $1 AND tenant_id = $2 AND status = 'ACTIVE'",
                    &[&old_uuid, &tenant_uuid],
                )
                .map_err(|e| {
                    DataError::new(
                        DataErrorCode::ExternalProvider,
                        format!("postgres supersede (old): {e}"),
                    )
                })?;
            if n == 0 {
                return Err(DataError::new(
                    DataErrorCode::Conflict,
                    "old record not found or not ACTIVE",
                ));
            }
            tx.execute(
                "INSERT INTO memory_records (
                    memory_id, tenant_id, namespace, memory_type, content, content_hash,
                    source, actor, created_at, observed_at, confidence, sensitivity,
                    purpose, retention, status, derived_from, supersedes, embedding_ref
                 ) VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7, $8, $9::text::timestamptz,
                    $10::text::timestamptz, $11, $12, $13, $14, 'ACTIVE', $15::uuid[], $16::uuid,
                    $17)",
                &[
                    &new_uuid,
                    &tenant_uuid,
                    &new_record.namespace,
                    &new_record.memory_type.as_str(),
                    &new_record.content,
                    &new_record.content_hash,
                    &new_record.source,
                    &new_record.actor,
                    &new_record.created_at,
                    &new_record.observed_at,
                    &new_record.confidence,
                    &new_record.sensitivity.as_str(),
                    &new_record.purpose,
                    &new_record.retention.to_string(),
                    &derived,
                    &supersedes,
                    &embedding_ref,
                ],
            )
            .map_err(|e| {
                DataError::new(
                    DataErrorCode::ExternalProvider,
                    format!("postgres supersede (new): {e}"),
                )
            })?;
            Ok(())
        })
    }
}

/// Canonical sensitivity rank (declaration order: PUBLIC lowest, SECRET
/// highest). Used by the SQL sensitivity_rank function created in migration
/// 004's function (see `query`).
fn sensitivity_rank(s: Sensitivity) -> i32 {
    match s {
        Sensitivity::Public => 0,
        Sensitivity::Household => 1,
        Sensitivity::Personal => 2,
        Sensitivity::Sensitive => 3,
        Sensitivity::BusinessConfidential => 4,
        Sensitivity::Security => 5,
        Sensitivity::Secret => 6,
    }
}
