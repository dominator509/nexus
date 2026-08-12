//! Session (EP-003 acceptance obligation 1).
//!
//! Sessions are independently scoped from people, devices, and businesses.
//! A session is a bounded, revocable period of authenticated interaction
//! tied to a principal and (optionally) a device.

use std::fmt;

use nexus_domain::{CorrelationId, DeviceId, NexusId, PrincipalType, TenantId};
use serde::{Deserialize, Serialize};

/// Session lifecycle state (ADR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionState {
    Active,
    Expired,
    Revoked,
}

impl SessionState {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Expired => "EXPIRED",
            Self::Revoked => "REVOKED",
        }
    }
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned by session operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// The session has already terminated (expired or revoked).
    AlreadyTerminated,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyTerminated => f.write_str("session already terminated"),
        }
    }
}

impl std::error::Error for SessionError {}

/// A bounded period of authenticated interaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Session identifier.
    pub session_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Principal that owns the session.
    pub principal_id: NexusId,
    /// Actor class of the owning principal.
    pub principal_type: PrincipalType,
    /// Device bound to the session, when applicable.
    pub device_id: Option<DeviceId>,
    /// Session state.
    pub state: SessionState,
    /// Unix timestamp in seconds when the session was created.
    pub created_at_unix_s: i64,
    /// Unix timestamp in seconds when the session expires.
    pub expires_at_unix_s: i64,
    /// Correlation of the event that created the session.
    pub created_by_correlation: CorrelationId,
}

impl Session {
    /// Construct an active session.
    pub fn new(
        session_id: NexusId,
        tenant_id: TenantId,
        principal_id: NexusId,
        principal_type: PrincipalType,
        device_id: Option<DeviceId>,
        created_at_unix_s: i64,
        expires_at_unix_s: i64,
        created_by_correlation: CorrelationId,
    ) -> Self {
        Self {
            session_id,
            tenant_id,
            principal_id,
            principal_type,
            device_id,
            state: SessionState::Active,
            created_at_unix_s,
            expires_at_unix_s,
            created_by_correlation,
        }
    }

    /// Whether the session is currently valid at the given time.
    pub fn is_valid_at(&self, now_unix_s: i64) -> bool {
        self.state == SessionState::Active
            && self.created_at_unix_s <= now_unix_s
            && now_unix_s < self.expires_at_unix_s
    }

    /// Mark the session as expired.
    ///
    /// Idempotent: calling twice returns the same state. A revoked session
    /// stays revoked; revocation is terminal and dominates expiry.
    pub fn expire(&mut self) {
        if self.state == SessionState::Active {
            self.state = SessionState::Expired;
        }
    }

    /// Revoke the session.
    ///
    /// Returns an error if the session already terminated.
    pub fn revoke(&mut self) -> Result<(), SessionError> {
        if self.state != SessionState::Active {
            return Err(SessionError::AlreadyTerminated);
        }
        self.state = SessionState::Revoked;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::NexusId;

    const SID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6130";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";
    const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";

    fn session(created: i64, expires: i64) -> Session {
        Session::new(
            NexusId::new(SID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            NexusId::new(PID).unwrap(),
            PrincipalType::Human,
            None,
            created,
            expires,
            CorrelationId::new(CORR).unwrap(),
        )
    }

    #[test]
    fn ep003_unit_session_validity_window() {
        let s = session(1000, 2000);
        assert!(s.is_valid_at(1000));
        assert!(s.is_valid_at(1999));
        assert!(!s.is_valid_at(2000)); // expiry is exclusive
        assert!(!s.is_valid_at(999)); // before creation
    }

    #[test]
    fn ep003_unit_session_expire_is_idempotent() {
        let mut s = session(1000, 2000);
        s.expire();
        s.expire(); // idempotent
        assert!(!s.is_valid_at(1500));
        assert_eq!(s.state, SessionState::Expired);
    }

    #[test]
    fn ep003_unit_session_revoke_guards_terminated() {
        let mut s = session(1000, 2000);
        s.revoke().unwrap();
        assert_eq!(s.state, SessionState::Revoked);
        assert!(!s.is_valid_at(1500));
        // Revoking twice fails; expiring an already-revoked session is safe.
        assert_eq!(s.revoke(), Err(SessionError::AlreadyTerminated));
        s.expire();
        assert_eq!(s.state, SessionState::Revoked);
    }

    #[test]
    fn ep003_unit_session_serde_roundtrip() {
        let s = session(1000, 2000);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"state\":\"ACTIVE\""));
        let back: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}
