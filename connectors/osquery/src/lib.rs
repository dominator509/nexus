//! EP-031 osquery connector (M5).
//!
//! Endpoint profile security sensor (SPEC-013 behavior 3: Endpoint
//! adds Wazuh or osquery). Nexus is the SELF-HOSTED COLLECTOR:
//! `HttpOsqueryEndpoint` implements the DOCUMENTED osquery TLS remote
//! API server surface (osquery.readthedocs.io/en/stable/deployment/
//! remote) - POST /enroll, POST /distributed_read, POST
//! /distributed_write - so a real osqueryd node can enroll and report
//! observed telemetry. Free-form osquery JSON is normalized at this
//! infrastructure boundary and never becomes a domain contract.

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::{OsqueryEndpointTelemetryProvider, OsqueryQueryResult};
pub use observability::{SentinelAuditEntry, SentinelObservability};
pub use transport::{
    DistributedQuery, HttpOsqueryEndpoint, OsqueryEnrollRequest, OsqueryTransport,
};
