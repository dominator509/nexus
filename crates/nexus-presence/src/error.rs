//! Typed presence errors (SPEC-006).
//!
//! Every boundary error carries a stable machine code and a safe human
//! explanation. Sensitive content is never included in error text.

use std::fmt;

/// Presence behavior error with a SPEC-006 machine code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceError {
    /// Evidence confidence outside 0.0..=1.0 or empty evidence set.
    Validation(String),
    /// Evidence is too stale to fuse (fail closed).
    StaleEvidence(String),
    /// A cross-tenant access attempt was refused without disclosure.
    NotFound,
    /// An unauthorized access attempt was refused without disclosure.
    Authorization(String),
}

impl PresenceError {
    /// Stable SPEC-006 machine code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::StaleEvidence(_) => "unavailable",
            Self::NotFound => "not_found",
            Self::Authorization(_) => "authorization",
        }
    }
}

impl fmt::Display for PresenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(msg) => write!(f, "validation: {msg}"),
            Self::StaleEvidence(msg) => write!(f, "unavailable: {msg}"),
            Self::NotFound => f.write_str("not_found: resource does not exist"),
            Self::Authorization(msg) => write!(f, "authorization: {msg}"),
        }
    }
}

impl std::error::Error for PresenceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep003_unit_presence_error_codes_are_stable() {
        assert_eq!(PresenceError::Validation("x".into()).code(), "validation");
        assert_eq!(
            PresenceError::StaleEvidence("x".into()).code(),
            "unavailable"
        );
        assert_eq!(PresenceError::NotFound.code(), "not_found");
        assert_eq!(
            PresenceError::Authorization("x".into()).code(),
            "authorization"
        );
    }

    #[test]
    fn ep003_unit_presence_error_never_leaks_details() {
        // The NotFound variant carries no tenant or resource information;
        // cross-tenant probes must not learn whether a resource exists.
        let e = PresenceError::NotFound;
        assert_eq!(e.to_string(), "not_found: resource does not exist");
    }
}
