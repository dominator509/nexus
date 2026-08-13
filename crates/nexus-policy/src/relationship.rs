//! Relationship authorization (SPEC-005; OpenFGA tuple shape).
//!
//! `RelationshipAuthorizer` is the provider-neutral port for
//! relationship checks. A provider implementation (OpenFGA in M3)
//! answers whether a principal holds a relation on an object within a
//! tenant. This crate defines the canonical tuple shape and the
//! decision envelope; it does not contain provider behavior.

use std::fmt;

use nexus_domain::TenantId;
use nexus_identity::Principal;
use serde::{Deserialize, Serialize};

use crate::error::{PolicyError, PolicyErrorCode};

/// A relationship tuple (SPEC-005): principal + relation + object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipTuple {
    /// Tenant boundary; tuples are always tenant-scoped.
    pub tenant_id: TenantId,
    /// Principal (or a role/group reference carried as a principal).
    pub principal: Principal,
    /// Relation name, e.g. `owner`, `member`, `reader`.
    pub relation: String,
    /// Object type, e.g. `household`, `task`, `memory`.
    pub object_type: String,
    /// Object identifier (canonical Nexus UUIDv7 string).
    pub object_id: String,
}

impl RelationshipTuple {
    /// Construct a relationship tuple; rejects empty relation/object
    /// names and non-canonical object identifiers.
    pub fn new(
        tenant_id: TenantId,
        principal: Principal,
        relation: impl Into<String>,
        object_type: impl Into<String>,
        object_id: impl Into<String>,
    ) -> Result<Self, RelationshipError> {
        let relation = relation.into();
        let object_type = object_type.into();
        let object_id = object_id.into();
        if relation.trim().is_empty() {
            return Err(RelationshipError::EmptyRelation);
        }
        if object_type.trim().is_empty() {
            return Err(RelationshipError::EmptyObjectType);
        }
        // Object identifiers are canonical Nexus UUIDv7 strings; reuse
        // the domain IdError surface via a NexusId parse for the shape.
        nexus_domain::NexusId::new(&object_id).map_err(|_| RelationshipError::InvalidObjectId)?;
        Ok(Self {
            tenant_id,
            principal,
            relation,
            object_type,
            object_id,
        })
    }
}

/// Relationship-specific construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipError {
    /// Relation name was empty/whitespace.
    EmptyRelation,
    /// Object type was empty/whitespace.
    EmptyObjectType,
    /// Object id is not a canonical Nexus UUIDv7.
    InvalidObjectId,
}

impl fmt::Display for RelationshipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyRelation => "relation must not be empty",
            Self::EmptyObjectType => "object type must not be empty",
            Self::InvalidObjectId => "object id is not a canonical Nexus UUIDv7",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for RelationshipError {}

/// A relationship decision (SPEC-005). Fail closed: only an explicit
/// `Allowed` grants; any error surfaces as a denied policy failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationshipDecision {
    /// The tuple holds; the principal has the relation on the object.
    Allowed,
    /// The tuple does not hold (or the provider could not prove it).
    Denied { reason: String },
}

impl RelationshipDecision {
    /// Whether the decision is an explicit allow.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// Provider-neutral relationship authorization port (SPEC-005).
pub trait RelationshipAuthorizer {
    /// Check whether the tuple holds. Fail closed: any error is a
    /// denial, never a grant.
    fn check(&self, tuple: &RelationshipTuple) -> Result<RelationshipDecision, PolicyError>;
}

/// Helper for provider implementations: map a provider failure to the
/// canonical policy error surface without leaking provider internals.
pub fn provider_failure(provider: &str, detail: &str) -> PolicyError {
    PolicyError::new(
        PolicyErrorCode::ExternalProvider,
        format!("relationship provider {provider} unavailable: {detail}"),
        None,
    )
}
