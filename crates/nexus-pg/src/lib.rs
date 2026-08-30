//! PostgreSQL and pgvector adapters (EP-004 M3, RX-005 AUD-007/AUD-008).
//!
//! Concrete implementations of the provider-neutral ports in `nexus-data`
//! and `nexus-events`:
//!
//! - `PgUnitOfWork` — real PostgreSQL transaction with fail-closed drop.
//! - `PgMemoryRepository` — memory records behind the `MemoryRepository`
//!   port, tenant-isolated by RLS at the database boundary.
//! - `PgWorldGraphRepository` — adjacency + recursive-walk fallback
//!   doctrine (INV-015), never a dedicated graph database.
//! - `PgVectorRepository` — pgvector HNSW index as a retrieval aid, never
//!   the source of truth (SPEC-002 behavior 2).
//! - `PgRepositorySet` — tenant-scoped composition root binding all three
//!   repositories to one transaction.
//! - `PgOutboxRepository` — transactional outbox (SPEC-023 behavior 1),
//!   atomic with the domain write through the shared unit of work.
//! - `PgInboxRepository` — idempotent consumer inbox (SPEC-023 behavior
//!   4), deduplicating by (consumer, event_id).
//!
//! Tenant isolation is enforced twice: every SQL statement carries the
//! tenant id AND the `003_tenant_isolation_rls.sql` migration enables row
//! level security with a policy keyed on the `app.tenant_id` session
//! setting. The adapters set that claim before every operation, so even a
//! statement that forgot its tenant filter would be denied by the database
//! (fail closed).
//!
//! This crate is infrastructure: it may import application ports
//! (`nexus-data`, `nexus-domain`, `nexus-events`) and drivers, never the
//! reverse. The dependency-direction tests in the behavior crates forbid
//! these drivers from leaking upward.

#![forbid(unsafe_code)]

mod inbox;
mod memory;
mod outbox;
mod repository_set;
mod unit_of_work;
mod vector;
mod world_graph;

pub use inbox::PgInboxRepository;
pub use memory::PgMemoryRepository;
pub use outbox::PgOutboxRepository;
pub use repository_set::PgRepositorySet;
pub use unit_of_work::PgUnitOfWork;
pub use vector::PgVectorRepository;
pub use world_graph::PgWorldGraphRepository;
