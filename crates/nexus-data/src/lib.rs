//! Nexus data and memory contracts (EP-004 M1).
//!
//! This crate owns the provider-neutral contracts for durable data access:
//!
//! - `error`: typed data errors (SPEC-006 codes, correlation preserved).
//! - `memory`: `MemoryRecord`, `MemoryQuery`, `MemoryCandidate`,
//!   `RetentionPolicy`, `Sensitivity`, `MemoryStatus`, `MemoryProposal`,
//!   `EmbeddingRef` - the canonical memory wire model (SPEC-002).
//! - `unit_of_work`: `UnitOfWork` transaction boundary.
//! - `repository_set`: `RepositorySet` composition root.
//! - `ports`: `MemoryRepository`, `WorldGraphRepository`,
//!   `PostgresWorldGraphRepository`, and `VectorRepository` ports.
//!
//! This crate imports `nexus-domain` (typed IDs, vocabulary) and serde only.
//! No infrastructure crate may be imported here; the dependency-direction
//! tests enforce it. Providers (PostgreSQL, pgvector) implement these ports
//! in `nexus-memory` and downstream infrastructure crates (EP-004 M2+).

#![forbid(unsafe_code)]

pub mod error;
pub mod memory;
pub mod ports;
pub mod repository_set;
pub mod unit_of_work;

pub use error::{DataError, DataErrorCode};
pub use memory::{
    EmbeddingRef, MemoryCandidate, MemoryProposal, MemoryQuery, MemoryRecord, MemoryStatus,
    RetentionPolicy, RetentionUnit, Sensitivity,
};
pub use ports::{
    MemoryRepository, PostgresWorldGraphRepository, VectorRepository, WorldGraphRepository,
};
pub use repository_set::RepositorySet;
pub use unit_of_work::UnitOfWork;
