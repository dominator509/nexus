//! nexus-test-execution: EP-040 M2 deterministic test execution core
//! (SPEC-008; TESTING.md).
//!
//! This crate implements the deterministic behavior behind the M1 ports:
//! it runs real test commands as subprocesses, parses real output,
//! aggregates GateResult with real counts, maps required tests to
//! evidence, applies the flake policy, and records redacted current-run
//! evidence. No mocked component: the runner executes real programs and
//! the parser consumes real output.
//!
//! M2 core invariants (proven by tests):
//! - TEST EXISTS != TEST RAN (evidence only after a real run)
//! - TEST RAN != BEHAVIOR VERIFIED (passing parse is not certification)
//! - MOCK PASSED != PRODUCTION PATH VERIFIED
//! - ZERO TESTS COLLECTED != GREEN (vacuity guard)
//! - SKIPPED TEST != PASSED TEST
//! - IGNORED TEST != PASSED TEST
//! - FLAKE RETRIED GREEN != ROOT CAUSE FIXED
//! - RESOURCE CLEANUP ATTEMPTED != RESOURCE CLEAN
//! - BUILD PASSED != RUNTIME SAFE

pub mod evidence;
pub mod policy;
pub mod runner;

pub use evidence::{EvidenceRecord, FileEvidenceStore};
pub use policy::{ConsecutiveVerify, FlakePolicy};
pub use runner::{parse_output, run_tests, TestCommand};

use nexus_test_contract::error::TestingResult;
use nexus_test_contract::model::TestMatrix;
use nexus_test_contract::TestMatrixPort;

/// Deterministic matrix validator: every required test id must be
/// non-empty and unique within its layer, and the matrix must not be
/// vacuous when the zero-test guard is on.
#[derive(Debug, Default)]
pub struct DeterministicMatrixValidator;

impl DeterministicMatrixValidator {
    pub fn new() -> Self {
        Self
    }
}

impl TestMatrixPort for DeterministicMatrixValidator {
    fn validate(&self, matrix: &TestMatrix) -> TestingResult<()> {
        matrix.validate()
    }
}
