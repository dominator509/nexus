//! MCP session binding (SPEC-003: authentication before tenant
//! resolution; tenant never selected by untrusted metadata).

use crate::error::McpError;
use nexus_auth::vocabulary::AuthenticationStrength;
use nexus_domain::{NexusId, PrincipalType, TenantId};
use serde::{Deserialize, Serialize};

/// Session lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpSessionState {
    Initialized,
    Active,
    Closed,
}

impl McpSessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Initialized => "INITIALIZED",
            Self::Active => "ACTIVE",
            Self::Closed => "CLOSED",
        }
    }
}

impl std::str::FromStr for McpSessionState {
    type Err = crate::error::McpError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "INITIALIZED" => Ok(Self::Initialized),
            "ACTIVE" => Ok(Self::Active),
            "CLOSED" => Ok(Self::Closed),
            other => Err(McpError::validation(format!(
                "unknown McpSessionState: {other}"
            ))),
        }
    }
}

/// Authenticated binding for an MCP session.
///
/// The tenant and principal come from the AUTHENTICATED identity, never
/// from request metadata (SPEC-003 required behavior 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBinding {
    pub principal_id: NexusId,
    pub principal_type: PrincipalType,
    pub tenant_id: TenantId,
    pub authentication_strength: AuthenticationStrength,
}

/// A tenant-safe MCP session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpSession {
    pub session_id: String,
    pub binding: SessionBinding,
    /// Origin validated at attach time.
    pub origin: String,
    pub state: McpSessionState,
}

impl McpSession {
    pub fn new(
        session_id: impl Into<String>,
        binding: SessionBinding,
        origin: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            binding,
            origin: origin.into(),
            state: McpSessionState::Initialized,
        }
    }

    /// A session is active only after initialization.
    pub fn activate(&mut self) -> Result<(), McpError> {
        if self.state != McpSessionState::Initialized {
            return Err(McpError::conflict(format!(
                "session {} is not initializable from state {}",
                self.session_id,
                self.state.as_str()
            )));
        }
        self.state = McpSessionState::Active;
        Ok(())
    }

    pub fn close(&mut self) {
        self.state = McpSessionState::Closed;
    }

    /// Tenant check against untrusted metadata: any request body that
    /// names a different tenant fails closed.
    pub fn enforce_tenant(&self, claimed_tenant: Option<&str>) -> Result<(), McpError> {
        match claimed_tenant {
            Some(claimed) if claimed != self.binding.tenant_id.as_str() => {
                Err(McpError::authorization(format!(
                    "tenant mismatch: session tenant {} != claimed {claimed}",
                    self.binding.tenant_id.as_str()
                )))
            }
            _ => Ok(()),
        }
    }

    /// Minimum authentication strength gate (SPEC-003 security).
    pub fn enforce_strength(&self, minimum: AuthenticationStrength) -> Result<(), McpError> {
        if (self.binding.authentication_strength as u8) < (minimum as u8) {
            return Err(McpError::authorization(format!(
                "authentication strength {:?} below required {:?}",
                self.binding.authentication_strength, minimum
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> SessionBinding {
        SessionBinding {
            principal_id: "018f0f6f-9c1e-7b6e-8000-00000000000a".parse().unwrap(),
            principal_type: PrincipalType::Human,
            tenant_id: "018f0f6f-9c1e-7b6e-8000-000000000003".parse().unwrap(),
            authentication_strength: AuthenticationStrength::MultiFactor,
        }
    }

    #[test]
    fn ep012_unit_mcp_session_activation_lifecycle() {
        let mut s = McpSession::new("s1", binding(), "https://app.nexus.local");
        assert_eq!(s.state, McpSessionState::Initialized);
        s.activate().unwrap();
        assert_eq!(s.state, McpSessionState::Active);
        // Double activation fails closed.
        assert!(s.activate().is_err());
        s.close();
        assert_eq!(s.state, McpSessionState::Closed);
    }

    #[test]
    fn ep012_unit_mcp_session_rejects_claimed_tenant_mismatch() {
        let mut s = McpSession::new("s1", binding(), "https://app.nexus.local");
        s.activate().unwrap();
        assert!(
            s.enforce_tenant(Some("018f0f6f-9c1e-7b6e-8000-000000000099"))
                .is_err()
        );
        assert!(
            s.enforce_tenant(Some("018f0f6f-9c1e-7b6e-8000-000000000003"))
                .is_ok()
        );
        assert!(s.enforce_tenant(None).is_ok());
    }

    #[test]
    fn ep012_unit_mcp_session_strength_gate() {
        let mut s = McpSession::new("s1", binding(), "https://app.nexus.local");
        s.activate().unwrap();
        assert!(
            s.enforce_strength(AuthenticationStrength::MultiFactor)
                .is_ok()
        );
        assert!(s.enforce_strength(AuthenticationStrength::StepUp).is_err());
    }
}
