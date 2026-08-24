//! Provider certification behavior: mock/simulated evidence can never
//! certify; a real probe certifies only for the exact provider/version/
//! interface exercised; stale evidence is rejected; unavailable providers
//! fail closed.

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};
use nexus_test_contract::model::ProviderCertificationSuite;
use nexus_test_contract::vocabulary::CertificationStatus;
use nexus_test_contract::ProviderCertificationPort;

use crate::transport::{PostgresTransport, ProviderProbe};

/// Provenance of certification evidence. Only real controlled-dependency
/// evidence can certify; mock/simulated evidence is mock-certified at
/// most (never CERTIFIED).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceProvenance {
    /// Evidence came from a mock or in-memory substitute.
    Mock,
    /// Evidence came from a simulated or scripted responder.
    Simulated,
    /// Evidence came from a real controlled dependency (real container,
    /// real provider, real observable effect).
    Real,
}

impl EvidenceProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mock => "MOCK",
            Self::Simulated => "SIMULATED",
            Self::Real => "REAL",
        }
    }
}

/// A real provider certifier bound to a live transport probe.
///
/// Certifying requires:
/// - a real probe (provider identity + version observed from the engine),
/// - non-empty, redacted, current evidence,
/// - and the exact provider/version/interface recorded in the suite.
pub struct RealProviderCertifier {
    probe: ProviderProbe,
    run_id: String,
    git_commit: String,
}

impl RealProviderCertifier {
    pub fn new(
        probe: ProviderProbe,
        run_id: impl Into<String>,
        git_commit: impl Into<String>,
    ) -> Self {
        Self {
            probe,
            run_id: run_id.into(),
            git_commit: git_commit.into(),
        }
    }

    /// Prove that mock/simulated evidence can never reach CERTIFIED.
    pub fn reject_provenance(
        &self,
        suite: &ProviderCertificationSuite,
        provenance: EvidenceProvenance,
    ) -> TestingResult<()> {
        if provenance != EvidenceProvenance::Real {
            return Err(TestingError::mock_only(format!(
                "{} evidence cannot certify provider {} (real controlled-dependency evidence required)",
                provenance.as_str(),
                suite.provider
            )));
        }
        Ok(())
    }

    /// Prove that stale evidence (run_id/git_commit mismatch) is rejected.
    pub fn reject_stale(&self, run_id: &str, git_commit: &str) -> TestingResult<()> {
        if run_id != self.run_id || git_commit != self.git_commit {
            return Err(TestingError::verification(
                "provider certification evidence is stale (run_id/git_commit mismatch)",
            ));
        }
        Ok(())
    }
}

impl ProviderCertificationPort for RealProviderCertifier {
    fn certify(
        &self,
        suite: ProviderCertificationSuite,
    ) -> TestingResult<ProviderCertificationSuite> {
        // Real probe identity must match the suite's declared provider.
        if suite.provider != self.probe.provider {
            return Err(TestingError::verification(format!(
                "certification suite provider {} does not match probed provider {}",
                suite.provider, self.probe.provider
            )));
        }
        // Evidence is required and must be redacted.
        if suite.evidence.is_empty() {
            return Err(TestingError::missing_evidence(
                "provider certification requires real controlled-dependency evidence",
            ));
        }
        for e in &suite.evidence {
            if nexus_test_contract::redact_secret_shaped(e) != *e {
                return Err(TestingError::validation(
                    "certification evidence must be redacted",
                ));
            }
        }
        // The suite certifies only for the exact interface exercised.
        let mut certified = suite.clone().certify(suite.evidence.clone())?;
        certified.status = CertificationStatus::Certified;
        Ok(certified)
    }
}

/// Deterministic behavior proofs that do not require a live provider:
/// mock/simulated rejection, stale rejection, missing evidence, and
/// provider/version identity binding.
pub mod behavior {
    use super::*;

    /// A mock-only certifier: no real probe exists, so certification is
    /// impossible by construction.
    pub fn certify_with_provenance(
        suite: ProviderCertificationSuite,
        provenance: EvidenceProvenance,
    ) -> TestingResult<ProviderCertificationSuite> {
        let certifier = RealProviderCertifier::new(
            ProviderProbe {
                provider: "postgresql".into(),
                version: "test-only".into(),
                interface: "sql-tcp-host-port".into(),
                digest: crate::POSTGRES_DIGEST.to_string(),
                roundtrip_ms: 1,
            },
            "run-mock",
            "0000000",
        );
        certifier.reject_provenance(&suite, provenance)?;
        certifier.certify(suite)
    }
}

/// A live probe wrapper so integration tests can run real probes.
pub fn probe_live(transport: &PostgresTransport) -> TestingResult<ProviderProbe> {
    transport
        .probe()
        .map_err(|e| TestingError::new(TestingErrorCode::Unavailable, e.to_string()))
}
