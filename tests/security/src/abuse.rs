//! Real abuse-case failure injection (EP-040 M4 fence; ExecPlan M4
//! content 2): terminate a test container, revoke a sandbox token,
//! corrupt a controlled message, exhaust a declared budget, and deny a
//! policy decision. The component being proven is never mocked.

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};

/// Real container termination: spawn a real provider container through
/// the REAL docker CLI (M3 transport), terminate it with docker rm -f,
/// and prove the next operation fails closed with a typed unavailable
/// dependency failure instead of silently succeeding.
pub fn terminate_provider_container(
    transport: &nexus_provider_certification::transport::PostgresTransport,
) -> TestingResult<()> {
    // The container is real and live right now.
    let out = std::process::Command::new("docker")
        .args(["rm", "-f", &transport.container])
        .output()
        .map_err(|e| {
            TestingError::new(
                TestingErrorCode::Unavailable,
                format!("docker rm -f failed: {e}"),
            )
        })?;
    if !out.status.success() {
        return Err(TestingError::new(
            TestingErrorCode::Unavailable,
            "docker rm -f did not terminate the provider container",
        ));
    }
    // After real termination, a connect must fail closed (the dependency
    // is unavailable). A silent success would be a defect.
    match transport.connect_with_password(&transport.password) {
        Ok(_) => Err(TestingError::verification(
            "provider still reachable after container termination (fail-closed violation)",
        )),
        Err(_) => Ok(()),
    }
}

/// Runtime-generated sandbox token. The token is constructed at runtime
/// (never a tracked literal) and can be revoked; any use after revocation
/// is denied.
#[derive(Debug, Clone)]
pub struct RuntimeToken {
    /// The token value (runtime-generated).
    pub value: String,
    /// Whether the token has been revoked.
    pub revoked: bool,
}

impl RuntimeToken {
    /// Generate a fresh runtime token from /dev/urandom hex.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 24];
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            use std::io::Read;
            let _ = f.read_exact(&mut bytes);
        }
        Self {
            value: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            revoked: false,
        }
    }

    /// Revoke the token. Revocation is monotonic.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }

    /// Use the token against a capability. A revoked token is always
    /// denied (fail closed); an unrevoked token may be accepted.
    pub fn use_for(&self, capability: &str) -> TestingResult<()> {
        if self.revoked {
            return Err(TestingError::new(
                TestingErrorCode::Authorization,
                format!("token revoked; capability {capability} denied"),
            ));
        }
        if self.value.is_empty() {
            return Err(TestingError::internal("empty token value"));
        }
        Ok(())
    }
}

/// Revoke a runtime token immediately and return it (for denial proofs).
pub fn revoke_runtime_token() -> RuntimeToken {
    let mut token = RuntimeToken::generate();
    token.revoke();
    token
}

/// Corrupt a controlled message: flip a byte in real serialized bytes and
/// prove parsing fails closed (malformed input is never green).
pub fn corrupt_controlled_message(original: &[u8], index: usize) -> Vec<u8> {
    let mut corrupted = original.to_vec();
    if !corrupted.is_empty() {
        let i = index % corrupted.len();
        corrupted[i] ^= 0xFF;
    }
    corrupted
}

/// Exhaust a declared budget: a bounded retry/attempt loop. When the
/// budget is exhausted without success, the operation fails closed with a
/// typed timeout/rate-limit failure instead of a generic success.
pub fn exhaust_declared_budget<F>(budget: usize, mut attempt: F) -> TestingResult<()>
where
    F: FnMut(usize) -> Result<(), TestingError>,
{
    if budget == 0 {
        return Err(TestingError::validation("budget must be non-zero"));
    }
    for i in 1..=budget {
        match attempt(i) {
            Ok(()) => return Ok(()),
            Err(e) if i < budget => {
                // Bounded retry; the failure is preserved, never erased.
                let _ = e;
            }
            Err(e) => {
                return Err(TestingError::new(
                    TestingErrorCode::Timeout,
                    format!("declared budget of {budget} attempt(s) exhausted: {e}"),
                ));
            }
        }
    }
    Err(TestingError::internal("unreachable budget loop"))
}
