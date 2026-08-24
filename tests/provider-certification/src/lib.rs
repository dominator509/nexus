//! nexus-provider-certification: EP-040 M3 real provider certification
//! transport (SPEC-008; TESTING.md; COMPONENT_REGISTRY.yaml).
//!
//! M3 proves the ProviderCertificationSuite port against a REAL controlled
//! dependency: an ephemeral digest-pinned `postgres:18.4` container. The
//! container is started by the real `docker` CLI, readiness is proven by
//! connecting through the PUBLISHED HOST PORT (never an in-memory
//! substitute, never a mock), and every proof executes real SQL against
//! the real engine.
//!
//! Permanent M3 invariants (proven by tests):
//! - REAL PROVIDER PASS -> CERTIFIED for exact provider/version/interface
//! - MOCK/SIMULATED EVIDENCE -> MockOnlyCertification, never Certified
//! - PROVIDER UNAVAILABLE -> Unavailable (never a silent skip)
//! - PROVIDER AUTH FAILURE -> Authentication failure, not generic success
//! - STALE PROVIDER EVIDENCE -> rejected
//! - READINESS != GREEN (probe must observe the real engine)
//! - CONTAINER STARTED != CLEAN (drop must remove the resource)

pub mod certifier;
pub mod transport;

pub use certifier::{EvidenceProvenance, RealProviderCertifier};
pub use transport::{PostgresTransport, ProviderProbe, TransportError};

/// Digest-pinned PostgreSQL image per COMPONENT_REGISTRY.yaml.
pub const POSTGRES_IMAGE: &str = "postgres:18.4";
pub const POSTGRES_DIGEST: &str =
    "sha256:a02db8cac496f15b094798a38254f14d6e00741f709360e5e00bb6668ea31636";
