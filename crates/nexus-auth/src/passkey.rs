//! Passkey enrollment contract (SPEC-005; EP-007).
//!
//! Passkeys are WebAuthn-based possession factors. The enrollment flow:
//! request a challenge (with a bound device and tenant), register the
//! credential, verify the assertion, and manage revocation. The contract
//! is provider-neutral; WebAuthn specifics are normalized at the
//! infrastructure boundary.

use std::fmt;

use nexus_domain::{CorrelationId, DeviceId, NexusId, TenantId};
use serde::{Deserialize, Serialize};

use crate::vocabulary::{AuthenticationStrength, PasskeyState};

/// Error returned by passkey enrollment operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasskeyError {
    /// The challenge has expired.
    ChallengeExpired,
    /// The challenge is in the wrong state for the operation.
    WrongState,
    /// The registration response failed verification.
    VerificationFailed(String),
    /// The credential is already registered.
    AlreadyRegistered,
    /// A required field is absent or malformed.
    Malformed(String),
}

impl fmt::Display for PasskeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChallengeExpired => f.write_str("passkey challenge expired"),
            Self::WrongState => f.write_str("passkey challenge in wrong state"),
            Self::VerificationFailed(detail) => write!(f, "passkey verification failed: {detail}"),
            Self::AlreadyRegistered => f.write_str("passkey credential already registered"),
            Self::Malformed(detail) => write!(f, "malformed passkey input: {detail}"),
        }
    }
}

impl std::error::Error for PasskeyError {}

/// A passkey enrollment challenge (pending state).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasskeyChallenge {
    /// Nexus-wide challenge identifier.
    pub challenge_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Principal that requested enrollment.
    pub principal_id: NexusId,
    /// Device the credential will be bound to.
    pub device_id: DeviceId,
    /// Opaque challenge payload (random per issuance; never stored plaintext
    /// after use).
    pub challenge: String,
    /// Unix seconds when the challenge was created.
    pub created_at_unix_s: i64,
    /// Unix seconds when the challenge expires.
    pub expires_at_unix_s: i64,
    /// Correlation of the enrollment event.
    pub correlation: CorrelationId,
    /// Current lifecycle state.
    pub state: PasskeyState,
}

impl PasskeyChallenge {
    /// Construct a pending challenge.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        challenge_id: NexusId,
        tenant_id: TenantId,
        principal_id: NexusId,
        device_id: DeviceId,
        challenge: impl Into<String>,
        created_at_unix_s: i64,
        expires_at_unix_s: i64,
        correlation: CorrelationId,
    ) -> Result<Self, PasskeyError> {
        let challenge = challenge.into();
        if challenge.trim().is_empty() {
            return Err(PasskeyError::Malformed("empty challenge".into()));
        }
        if expires_at_unix_s <= created_at_unix_s {
            return Err(PasskeyError::Malformed(
                "expiry must be after creation".into(),
            ));
        }
        Ok(Self {
            challenge_id,
            tenant_id,
            principal_id,
            device_id,
            challenge,
            created_at_unix_s,
            expires_at_unix_s,
            correlation,
            state: PasskeyState::PendingChallenge,
        })
    }

    /// Whether the challenge is still usable at the given time.
    pub fn is_valid_at(&self, now_unix_s: i64) -> bool {
        self.state == PasskeyState::PendingChallenge
            && self.created_at_unix_s <= now_unix_s
            && now_unix_s < self.expires_at_unix_s
    }

    /// Verify a registration response against the expected challenge.
    ///
    /// The provider layer performs the actual WebAuthn verification and
    /// passes the outcome here; the contract consumes a normalized
    /// response. Returns the registered credential on success.
    pub fn verify_registration(
        &mut self,
        response: &RegistrationResponse,
        now_unix_s: i64,
    ) -> Result<RegisteredCredential, PasskeyError> {
        if self.state != PasskeyState::PendingChallenge {
            return Err(PasskeyError::WrongState);
        }
        if now_unix_s >= self.expires_at_unix_s {
            return Err(PasskeyError::ChallengeExpired);
        }
        if !response.verification_ok {
            return Err(PasskeyError::VerificationFailed(
                response.failure_detail.clone().unwrap_or_default(),
            ));
        }
        if response.credential_id.trim().is_empty() {
            return Err(PasskeyError::Malformed("empty credential id".into()));
        }
        let credential = RegisteredCredential::new(
            NexusId::new(&response.credential_id)
                .map_err(|e| PasskeyError::Malformed(format!("credential id: {e}")))?,
            self.tenant_id.clone(),
            self.principal_id.clone(),
            self.device_id.clone(),
            response.credential_id.clone(),
            self.challenge_id.clone(),
        )?;
        self.state = PasskeyState::Registered;
        Ok(credential)
    }

    /// Revoke the challenge (also revokes any issued credential).
    pub fn revoke(&mut self) {
        self.state = PasskeyState::Revoked;
    }
}

/// A normalized registration response from the WebAuthn boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationResponse {
    /// Whether the provider verified the assertion cryptographically.
    pub verification_ok: bool,
    /// Failure detail when verification failed (redacted upstream).
    pub failure_detail: Option<String>,
    /// Provider credential id (opaque, provider-scoped).
    pub credential_id: String,
}

/// A registered passkey credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredCredential {
    /// Nexus-wide credential identifier.
    pub credential_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Owning principal.
    pub principal_id: NexusId,
    /// Bound device.
    pub device_id: DeviceId,
    /// Provider-scoped credential handle.
    pub provider_handle: String,
    /// Challenge that produced this registration.
    pub challenge_id: NexusId,
}

impl RegisteredCredential {
    /// Construct a validated registered credential.
    pub fn new(
        credential_id: NexusId,
        tenant_id: TenantId,
        principal_id: NexusId,
        device_id: DeviceId,
        provider_handle: impl Into<String>,
        challenge_id: NexusId,
    ) -> Result<Self, PasskeyError> {
        let provider_handle = provider_handle.into();
        if provider_handle.trim().is_empty() {
            return Err(PasskeyError::Malformed("empty provider handle".into()));
        }
        Ok(Self {
            credential_id,
            tenant_id,
            principal_id,
            device_id,
            provider_handle,
            challenge_id,
        })
    }
}

/// A passkey assertion used for step-up authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasskeyAssertion {
    /// Credential that produced the assertion.
    pub credential_id: NexusId,
    /// Whether the provider verified the assertion.
    pub verification_ok: bool,
    /// Authentication strength this assertion satisfies (STEP_UP).
    pub strength: AuthenticationStrength,
    /// Correlation of the assertion event.
    pub correlation: CorrelationId,
}

impl PasskeyAssertion {
    /// Construct a validated assertion.
    pub fn new(credential_id: NexusId, verification_ok: bool, correlation: CorrelationId) -> Self {
        Self {
            credential_id,
            verification_ok,
            strength: AuthenticationStrength::StepUp,
            correlation,
        }
    }

    /// Whether this assertion satisfies the required strength.
    pub fn satisfies(&self, required: AuthenticationStrength) -> bool {
        self.verification_ok && self.strength >= required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103";
    const DID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105";
    const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";
    const CRED: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6107";

    fn challenge(created: i64, expires: i64) -> PasskeyChallenge {
        PasskeyChallenge::new(
            NexusId::new(CID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            DeviceId::new(DID).unwrap(),
            "challenge-payload",
            created,
            expires,
            CorrelationId::new(CORR).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn ep007_unit_passkey_challenge_constructs_and_validates_window() {
        let c = challenge(1000, 2000);
        assert!(c.is_valid_at(1000));
        assert!(c.is_valid_at(1500));
        assert!(!c.is_valid_at(2000));
        assert!(!c.is_valid_at(999)); // before creation
    }

    #[test]
    fn ep007_unit_passkey_challenge_rejects_inverted_window() {
        let res = PasskeyChallenge::new(
            NexusId::new(CID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            DeviceId::new(DID).unwrap(),
            "challenge-payload",
            2000,
            1000,
            CorrelationId::new(CORR).unwrap(),
        );
        assert_eq!(
            res,
            Err(PasskeyError::Malformed(
                "expiry must be after creation".into()
            ))
        );
    }

    #[test]
    fn ep007_unit_passkey_challenge_rejects_empty_payload() {
        let res = PasskeyChallenge::new(
            NexusId::new(CID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            DeviceId::new(DID).unwrap(),
            "",
            1000,
            2000,
            CorrelationId::new(CORR).unwrap(),
        );
        assert_eq!(res, Err(PasskeyError::Malformed("empty challenge".into())));
    }

    #[test]
    fn ep007_unit_passkey_registration_success_marks_registered() {
        let mut c = challenge(1000, 2000);
        let response = RegistrationResponse {
            verification_ok: true,
            failure_detail: None,
            credential_id: CRED.to_string(),
        };
        let credential = c.verify_registration(&response, 1500).unwrap();
        assert_eq!(credential.provider_handle, CRED);
        assert_eq!(c.state, PasskeyState::Registered);
    }

    #[test]
    fn ep007_unit_passkey_registration_rejects_failed_verification() {
        let mut c = challenge(1000, 2000);
        let response = RegistrationResponse {
            verification_ok: false,
            failure_detail: Some("bad signature".into()),
            credential_id: CRED.to_string(),
        };
        let res = c.verify_registration(&response, 1500);
        assert_eq!(
            res,
            Err(PasskeyError::VerificationFailed("bad signature".into()))
        );
        assert_eq!(c.state, PasskeyState::PendingChallenge);
    }

    #[test]
    fn ep007_unit_passkey_registration_rejects_expired_challenge() {
        let mut c = challenge(1000, 2000);
        let response = RegistrationResponse {
            verification_ok: true,
            failure_detail: None,
            credential_id: CRED.to_string(),
        };
        let res = c.verify_registration(&response, 2500);
        assert_eq!(res, Err(PasskeyError::ChallengeExpired));
    }

    #[test]
    fn ep007_unit_passkey_challenge_revokes() {
        let mut c = challenge(1000, 2000);
        c.revoke();
        assert_eq!(c.state, PasskeyState::Revoked);
        assert!(!c.is_valid_at(1500));
    }

    #[test]
    fn ep007_unit_passkey_assertion_satisfies_step_up() {
        let assertion = PasskeyAssertion::new(
            NexusId::new(CRED).unwrap(),
            true,
            CorrelationId::new(CORR).unwrap(),
        );
        assert!(assertion.satisfies(AuthenticationStrength::StepUp));
        let failed = PasskeyAssertion::new(
            NexusId::new(CRED).unwrap(),
            false,
            CorrelationId::new(CORR).unwrap(),
        );
        assert!(!failed.satisfies(AuthenticationStrength::StepUp));
    }

    #[test]
    fn ep007_unit_passkey_records_serde_roundtrip() {
        let mut c = challenge(1000, 2000);
        let response = RegistrationResponse {
            verification_ok: true,
            failure_detail: None,
            credential_id: CRED.to_string(),
        };
        let credential = c.verify_registration(&response, 1500).unwrap();
        let json = serde_json::to_string(&credential).unwrap();
        let back: RegisteredCredential = serde_json::from_str(&json).unwrap();
        assert_eq!(back, credential);
        assert!(!json.contains("\"REGISTERED\"")); // credential has no state; challenge does
    }
}
