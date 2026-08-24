//! nexus-security-core: EP-040 M4 security test behavior (SPEC-008;
//! SECURITY.md; node contract).
//!
//! This crate owns real security test behavior: forbidden-secret-literal
//! scanning, redaction of evidence, authorization failure, insecure-config
//! rejection, scanner-unavailable capability blocking, stale/malformed
//! scan-output rejection, mock-only certification distinction, and real
//! failure-injection abuse cases (terminate a container, revoke a token,
//! corrupt a controlled message, exhaust a declared budget, deny a policy
//! decision). No component being proven is mocked.
//!
//! Permanent invariants proven by tests:
//! - SECURITY SCAN RAN != SECURITY HARDENED
//! - SCANNER OUTPUT EXISTS != AUDIT CERTIFIED
//! - MALFORMED OUTPUT != GREEN
//! - UNKNOWN CRITERION != SAFE
//! - MISSING SCAN TARGET != GREEN
//! - MOCK SECURITY SCAN != PRODUCTION SECURITY CERTIFIED
//! - TOKEN REVOKED != TOKEN ACCEPTED
//! - BUDGET EXHAUSTED != OPERATION GREEN

pub mod abuse;
pub mod evidence;
pub mod policy;
pub mod scanner;

pub use abuse::{
    corrupt_controlled_message, exhaust_declared_budget, revoke_runtime_token,
    terminate_provider_container, RuntimeToken,
};
pub use evidence::{SecurityEvidence, SecurityEvidenceStore};
pub use policy::{AuthDecision, InsecureConfig, SecurityPolicy};
pub use scanner::{ScanFinding, ScanOutcome, ScanTarget, SecurityScanner};
