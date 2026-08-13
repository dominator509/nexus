//! Session service contract (SPEC-005; EP-007).
//!
//! `SessionService` is the authentication-layer session contract. It
//! composes the EP-003 `Session` model (bounded, revocable authenticated
//! interaction) with an authentication strength, an issuance origin, and
//! refresh binding. Session records remain EP-003-owned; this node adds
//! the auth-layer issuance and audit contract.

use std::fmt;

use nexus_domain::{CorrelationId, DeviceId, NexusId, TenantId};
use serde::{Deserialize, Serialize};

use crate::vocabulary::AuthenticationStrength;

/// Error returned by session service operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionServiceError {
    /// The session is already terminated.
    AlreadyTerminated,
    /// The session does not satisfy the required strength.
    InsufficientStrength,
    /// A required field is absent or malformed.
    Malformed(String),
}

impl fmt::Display for SessionServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyTerminated => f.write_str("session already terminated"),
            Self::InsufficientStrength => {
                f.write_str("session authentication strength insufficient")
            }
            Self::Malformed(detail) => write!(f, "malformed session: {detail}"),
        }
    }
}

impl std::error::Error for SessionServiceError {}

/// Auth-layer session record.
///
/// The same `session_id` binds the EP-003 session row and this
/// auth-layer projection. `strength` is the strength at issuance; it is
/// immutable for the session lifetime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthSession {
    /// Session identifier (shared with the EP-003 session row).
    pub session_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Owning principal.
    pub principal_id: NexusId,
    /// Device bound to the session, when applicable.
    pub device_id: Option<DeviceId>,
    /// Authentication strength at issuance.
    pub strength: AuthenticationStrength,
    /// Unix seconds when the session was created.
    pub created_at_unix_s: i64,
    /// Unix seconds when the session expires.
    pub expires_at_unix_s: i64,
    /// Refresh token binding (opaque; rotation-only).
    pub refresh_handle: Option<String>,
    /// Correlation of the issuance event.
    pub correlation: CorrelationId,
}

impl AuthSession {
    /// Construct a validated auth session.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: NexusId,
        tenant_id: TenantId,
        principal_id: NexusId,
        device_id: Option<DeviceId>,
        strength: AuthenticationStrength,
        created_at_unix_s: i64,
        expires_at_unix_s: i64,
        refresh_handle: Option<String>,
        correlation: CorrelationId,
    ) -> Result<Self, SessionServiceError> {
        if expires_at_unix_s <= created_at_unix_s {
            return Err(SessionServiceError::Malformed(
                "expiry must be after creation".into(),
            ));
        }
        Ok(Self {
            session_id,
            tenant_id,
            principal_id,
            device_id,
            strength,
            created_at_unix_s,
            expires_at_unix_s,
            refresh_handle,
            correlation,
        })
    }

    /// Whether the session is valid at the given time.
    pub fn is_valid_at(&self, now_unix_s: i64) -> bool {
        self.created_at_unix_s <= now_unix_s && now_unix_s < self.expires_at_unix_s
    }

    /// Whether the session satisfies the required authentication strength.
    pub fn satisfies(&self, required: AuthenticationStrength) -> bool {
        self.strength >= required
    }
}

/// Session lifecycle transition record (audit-facing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionAuditRecord {
    /// Session identifier.
    pub session_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Owning principal.
    pub principal_id: NexusId,
    /// Lifecycle action taken: CREATED, REVOKED, EXPIRED.
    pub action: String,
    /// Unix seconds of the event.
    pub at_unix_s: i64,
    /// Correlation of the event.
    pub correlation: CorrelationId,
}

impl SessionAuditRecord {
    /// Construct a validated audit record.
    pub fn new(
        session_id: NexusId,
        tenant_id: TenantId,
        principal_id: NexusId,
        action: impl Into<String>,
        at_unix_s: i64,
        correlation: CorrelationId,
    ) -> Result<Self, SessionServiceError> {
        let action = action.into();
        match action.as_str() {
            "CREATED" | "REVOKED" | "EXPIRED" => {}
            other => {
                return Err(SessionServiceError::Malformed(format!(
                    "unknown session action {other}"
                )));
            }
        }
        Ok(Self {
            session_id,
            tenant_id,
            principal_id,
            action,
            at_unix_s,
            correlation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6130";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";
    const DID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105";
    const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";

    fn session() -> AuthSession {
        AuthSession::new(
            NexusId::new(SID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            Some(DeviceId::new(DID).unwrap()),
            AuthenticationStrength::MultiFactor,
            1000,
            2000,
            Some("refresh-handle".into()),
            CorrelationId::new(CORR).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn ep007_unit_auth_session_validity_window() {
        let s = session();
        assert!(s.is_valid_at(1000));
        assert!(s.is_valid_at(1999));
        assert!(!s.is_valid_at(2000));
        assert!(!s.is_valid_at(999));
    }

    #[test]
    fn ep007_unit_auth_session_strength_satisfaction() {
        let s = session();
        assert!(s.satisfies(AuthenticationStrength::SingleFactor));
        assert!(s.satisfies(AuthenticationStrength::MultiFactor));
        assert!(!s.satisfies(AuthenticationStrength::StepUp));
    }

    #[test]
    fn ep007_unit_auth_session_rejects_inverted_expiry() {
        let res = AuthSession::new(
            NexusId::new(SID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            None,
            AuthenticationStrength::SingleFactor,
            2000,
            1000,
            None,
            CorrelationId::new(CORR).unwrap(),
        );
        assert_eq!(
            res,
            Err(SessionServiceError::Malformed(
                "expiry must be after creation".into()
            ))
        );
    }

    #[test]
    fn ep007_unit_auth_session_serde_roundtrip() {
        let s = session();
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"MULTI_FACTOR\""));
        let back: AuthSession = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn ep007_unit_session_audit_record_accepts_canonical_actions() {
        for action in ["CREATED", "REVOKED", "EXPIRED"] {
            let rec = SessionAuditRecord::new(
                NexusId::new(SID).unwrap(),
                TenantId::new(TENANT).unwrap(),
                NexusId::new(PID).unwrap(),
                action,
                1500,
                CorrelationId::new(CORR).unwrap(),
            )
            .unwrap();
            assert_eq!(rec.action, action);
        }
    }

    #[test]
    fn ep007_unit_session_audit_record_rejects_unknown_action() {
        let res = SessionAuditRecord::new(
            NexusId::new(SID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            "TAMPERED",
            1500,
            CorrelationId::new(CORR).unwrap(),
        );
        assert!(matches!(res, Err(SessionServiceError::Malformed(_))));
    }
}
