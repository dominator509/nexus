//! Query capability port (SPEC-003 canonical term `Query`).
//!
//! A query is a read-only invocation: it observes state and never
//! mutates it. Query capabilities are distinct from command, workflow,
//! and stream classes by construction; there is no generic execute
//! string anywhere in the contract.

use serde::{Deserialize, Serialize};

use crate::context::InvocationContext;
use crate::error::CapabilityError;

/// Typed query request. The payload is a canonical JSON value whose
/// schema is advertised by the capability descriptor's `input_schema`;
/// free-form provider payloads are normalized at the infrastructure
/// boundary and never become domain contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryRequest {
    /// Capability key to invoke.
    pub capability_id: String,
    /// Invocation context.
    pub context: InvocationContext,
    /// Canonical input payload.
    pub input: serde_json::Value,
}

/// Typed query result. The payload's schema is advertised by the
/// capability descriptor's `output_schema`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryResult {
    /// Capability key that produced the result.
    pub capability_id: String,
    /// Canonical output payload.
    pub output: serde_json::Value,
}

/// Provider-neutral read-only capability port (SPEC-003).
pub trait QueryCapability {
    /// Execute a read-only query.
    fn query(&self, request: QueryRequest) -> Result<QueryResult, CapabilityError>;
}
