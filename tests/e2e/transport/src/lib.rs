//! nexus-e2e-transport: EP-040 M3 end-to-end transport journey
//! (SPEC-008; TESTING.md).
//!
//! This crate composes the M1 contract suite, the M2 execution core, and
//! the M3 real provider transport into ONE real end-to-end proof:
//!
//! real repo state -> real container -> real SQL -> real evidence ->
//! real verification -> real cleanup.
//!
//! No component is mocked. The M2 runner executes real commands and
//! parses real output; the M3 transport probes a real digest-pinned
//! PostgreSQL container; evidence is written through the M2 evidence
//! store and bound to the current run.
//!
//! M3 e2e invariants (proven by tests):
//! - BUILD PASSED != RUNTIME SAFE (evidence only after a real run)
//! - PARSED OUTPUT PASSED != TARGET BEHAVIOR VERIFIED
//! - MOCK PASSED != PRODUCTION PATH VERIFIED
//! - CONTAINER STARTED != RESOURCE CLEAN (drop verified)
//! - STALE/EMPTY EVIDENCE != GREEN

pub mod journey;

pub use journey::{E2eJourney, E2eJourneyResult};
