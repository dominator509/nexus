//! Protocol version validation (directive H).
//!
//! The sidecar speaks exactly one protocol version. Older versions,
//! future major versions, missing versions, and conflicting versions
//! all fail closed; the sidecar never silently downgrades or
//! reinterprets a future request as the current version.

use crate::error::{SidecarError, SidecarErrorKind};

/// Canonical protocol version spoken by the sidecar transport.
pub const PROTOCOL_VERSION: &str = "1";

/// Validate a declared protocol version (directive H).
///
/// Only the exact current version is accepted. Anything else - older,
/// future, or malformed - is a typed fail-closed rejection.
pub fn validate_protocol_version(declared: Option<&str>) -> Result<(), SidecarError> {
    match declared {
        None => Err(SidecarError::new(
            SidecarErrorKind::ProtocolVersionMismatch,
            "protocol version missing: header or envelope must declare a version",
            None,
            None,
            None,
        )),
        Some(v) if v == PROTOCOL_VERSION => Ok(()),
        Some(other) => Err(SidecarError::new(
            SidecarErrorKind::ProtocolVersionMismatch,
            format!("unsupported protocol version: {other:?}"),
            None,
            None,
            None,
        )),
    }
}

/// Cross-check protocol versions supplied in multiple places
/// (directive H: conflicting sources fail closed).
pub fn reconcile_protocol_versions(
    header: Option<&str>,
    envelope: Option<&str>,
) -> Result<(), SidecarError> {
    match (header, envelope) {
        // The HTTP header is the transport-level declaration; a
        // missing header is a fail-closed protocol violation even
        // when the envelope carries a version (directive H).
        (None, _) => Err(SidecarError::new(
            SidecarErrorKind::ProtocolVersionMismatch,
            "protocol version header missing",
            None,
            None,
            None,
        )),
        (Some(h), None) => validate_protocol_version(Some(h)),
        (Some(h), Some(e)) if h == e => validate_protocol_version(Some(h)),
        (Some(h), Some(e)) => Err(SidecarError::new(
            SidecarErrorKind::ProtocolVersionMismatch,
            format!("conflicting protocol versions: header {h:?}, envelope {e:?}"),
            None,
            None,
            None,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SidecarErrorKind;

    #[test]
    fn ep011_unit_sidecar_version_accepts_current() {
        assert!(validate_protocol_version(Some(PROTOCOL_VERSION)).is_ok());
    }

    #[test]
    fn ep011_unit_sidecar_version_fails_closed() {
        assert!(matches!(
            validate_protocol_version(None),
            Err(SidecarError {
                kind: SidecarErrorKind::ProtocolVersionMismatch,
                ..
            })
        ));
        assert!(matches!(
            validate_protocol_version(Some("0")),
            Err(SidecarError {
                kind: SidecarErrorKind::ProtocolVersionMismatch,
                ..
            })
        ));
        assert!(matches!(
            validate_protocol_version(Some("2")),
            Err(SidecarError {
                kind: SidecarErrorKind::ProtocolVersionMismatch,
                ..
            })
        ));
        assert!(matches!(
            validate_protocol_version(Some("999")),
            Err(SidecarError {
                kind: SidecarErrorKind::ProtocolVersionMismatch,
                ..
            })
        ));
    }

    #[test]
    fn ep011_unit_sidecar_version_conflict_fails_closed() {
        assert!(reconcile_protocol_versions(Some("1"), Some("2")).is_err());
        assert!(reconcile_protocol_versions(Some("2"), Some("1")).is_err());
        // Missing header fails closed even when the envelope declares
        // a version (directive H: header is the transport contract).
        assert!(reconcile_protocol_versions(None, Some("1")).is_err());
        assert!(reconcile_protocol_versions(None, None).is_err());
        assert!(reconcile_protocol_versions(Some("1"), Some("1")).is_ok());
    }
}
