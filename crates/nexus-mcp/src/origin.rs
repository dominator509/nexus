//! MCP Origin validation (SPEC-003 required behavior 2).
//!
//! A request whose Origin is not in the allowlist fails closed BEFORE
//! any session or tenant work.

/// Origin policy failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginPolicyError {
    MissingOrigin,
    NotAllowed(String),
}

impl std::fmt::Display for OriginPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOrigin => f.write_str("request has no Origin header"),
            Self::NotAllowed(origin) => write!(f, "origin not allowed: {origin}"),
        }
    }
}

/// Deterministic origin allowlist policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginPolicy {
    allowed: Vec<String>,
}

impl OriginPolicy {
    pub fn new(allowed: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut allowed: Vec<String> = allowed.into_iter().map(Into::into).collect();
        allowed.sort();
        allowed.dedup();
        Self { allowed }
    }

    pub fn allowed(&self) -> &[String] {
        &self.allowed
    }

    /// Validate an Origin header value (exact match against the
    /// allowlist; no suffix/prefix wildcarding).
    pub fn validate(&self, origin: Option<&str>) -> Result<(), OriginPolicyError> {
        let Some(origin) = origin else {
            return Err(OriginPolicyError::MissingOrigin);
        };
        if self.allowed.iter().any(|a| a == origin) {
            Ok(())
        } else {
            Err(OriginPolicyError::NotAllowed(origin.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_mcp_origin_allowlist_exact_match() {
        let policy = OriginPolicy::new(["https://app.nexus.local"]);
        assert!(policy.validate(Some("https://app.nexus.local")).is_ok());
        assert!(policy.validate(Some("https://evil.example.com")).is_err());
        assert!(policy.validate(None).is_err());
    }

    #[test]
    fn ep012_unit_mcp_origin_no_wildcard_bypass() {
        let policy = OriginPolicy::new(["https://app.nexus.local"]);
        // Prefix tricks must not pass exact matching.
        assert!(
            policy
                .validate(Some("https://app.nexus.local.evil.com"))
                .is_err()
        );
        assert!(policy.validate(Some("https://app.nexus.local/")).is_err());
    }

    #[test]
    fn ep012_unit_mcp_origin_dedup_and_order() {
        let policy = OriginPolicy::new(["b.example", "a.example", "b.example"]);
        assert_eq!(
            policy.allowed(),
            &["a.example".to_string(), "b.example".to_string()]
        );
    }
}
