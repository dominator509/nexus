//! Nexus capability and connector domain (EP-010).
//!
//! Owns the provider-neutral capability model: capability registry,
//! capability descriptors, connector manifests, and the distinct
//! query / command / workflow / health / change-feed ports
//! (SPEC-003, SPEC-022). This crate may import `nexus-domain`
//! (typed IDs and canonical vocabulary) and `nexus-identity`
//! (principals, devices, trust levels) plus serde only. No
//! infrastructure, database, network, or vendor crate may be
//! imported here; the dependency-direction tests enforce this
//! boundary.
//!
//! INV-003 + SPEC-003/SPEC-022: capabilities advertise stable
//! schemas, scopes, risk, idempotency, health, and availability;
//! read, proposal, command, and workflow classes remain distinct;
//! a generic execute string is impossible; unavailable provider
//! features are not advertised.

#![forbid(unsafe_code)]

pub mod changefeed;
pub mod command;
pub mod context;
pub mod descriptor;
pub mod error;
pub mod health;
pub mod manifest;
pub mod query;
pub mod registry;
pub mod vocabulary;
pub mod workflow;

pub use changefeed::{ChangeBatch, ChangeCursor, ChangeFeedCapability};
pub use command::{CommandCapability, CommandRequest, CommandResult};
pub use context::{InvocationContext, InvocationContextError};
pub use descriptor::{CapabilityDescriptor, CapabilityDescriptorError};
pub use error::{CapabilityError, CapabilityErrorCode};
pub use health::{HealthCapability, HealthReport};
pub use manifest::{ConnectorBinding, ConnectorManifest, ConnectorManifestError};
pub use query::{QueryCapability, QueryRequest, QueryResult};
pub use registry::CapabilityRegistry;
pub use vocabulary::{Certification, HealthState, SchemaRef};
pub use workflow::{
    WorkflowCapability, WorkflowHandle, WorkflowRequest, WorkflowResult, WorkflowStatus,
};

#[cfg(test)]
mod lib_tests;
