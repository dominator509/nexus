//! Typed sidecar failures (SPEC-006 parity, directive X).
//!
//! Every sidecar failure carries the canonical `SdkError` envelope so
//! Rust, TypeScript, and Python SDKs interpret it identically. The
//! `kind` gives the sidecar precise routing/telemetry semantics while
//! the `to_sdk_error` mapping keeps the wire codes canonical.
//!
//! Failures fail closed: an error is never converted into success.

use nexus_connector_sdk::{SdkError, SdkErrorCode};

/// Canonical sidecar failure classes beyond the SDK's base codes.
///
/// The SDK's canonical vocabulary has no `PAYLOAD_TOO_LARGE` or
/// `PROTOCOL_VERSION_MISMATCH` codes; directive X permits canonical
/// equivalents where the exact vocabulary differs. These kinds map to
/// `VALIDATION` on the wire with precise messages (M3 precedent: the
/// fixture already used `VALIDATION` for oversized bodies and
/// unsupported protocol versions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarErrorKind {
    /// Malformed/missing envelope fields, duplicate security keys,
    /// unknown top-level fields, invalid ids, wrong class.
    Validation,
    /// Provider process absent, unreachable, or unavailable.
    Unavailable,
    /// Provider exceeded the bounded timeout.
    Timeout,
    /// Provider returned a malformed/truncated/schema-invalid payload.
    ProviderError,
    /// Credential reference denied by scope (connector/tenant/reference).
    CredentialDenied,
    /// Request rejected for protocol version reasons.
    ProtocolVersionMismatch,
    /// Request body exceeded the bounded size.
    PayloadTooLarge,
    /// Provider response exceeded the bounded size.
    ResponseTooLarge,
    /// Webhook signature invalid or replay detected.
    WebhookRejected,
    /// Poller checkpoint/source corrupt or unsafe.
    PollerCorrupt,
    /// Concurrency bound exceeded (typed overload, directive T).
    Overloaded,
    /// Internal sidecar failure (never exposes internals).
    Internal,
}

/// Typed sidecar error with canonical SDK wire mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarError {
    /// Sidecar-specific failure class.
    pub kind: SidecarErrorKind,
    /// Human-readable reason (never contains secrets).
    pub message: String,
    /// Correlation id when known.
    pub correlation_id: Option<String>,
    /// Tenant id when known.
    pub tenant: Option<String>,
    /// Resource (connector/capability) when known.
    pub resource: Option<String>,
}

impl SidecarError {
    /// Construct a sidecar error with context.
    pub fn new(
        kind: SidecarErrorKind,
        message: impl Into<String>,
        correlation_id: Option<String>,
        tenant: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            correlation_id,
            tenant,
            resource,
        }
    }

    /// Construct a validation error.
    pub fn validation(message: impl Into<String>, correlation_id: Option<String>) -> Self {
        Self::new(
            SidecarErrorKind::Validation,
            message,
            correlation_id,
            None,
            None,
        )
    }

    /// Canonical wire code (directive X: vocabulary-preserving).
    pub fn wire_code(&self) -> SdkErrorCode {
        match self.kind {
            SidecarErrorKind::Validation
            | SidecarErrorKind::ProtocolVersionMismatch
            | SidecarErrorKind::PayloadTooLarge
            | SidecarErrorKind::ResponseTooLarge
            | SidecarErrorKind::PollerCorrupt => SdkErrorCode::Validation,
            SidecarErrorKind::Unavailable => SdkErrorCode::Unavailable,
            SidecarErrorKind::Timeout => SdkErrorCode::Timeout,
            SidecarErrorKind::ProviderError => SdkErrorCode::ExternalProvider,
            SidecarErrorKind::CredentialDenied => SdkErrorCode::Authorization,
            SidecarErrorKind::WebhookRejected => SdkErrorCode::Verification,
            SidecarErrorKind::Overloaded => SdkErrorCode::RateLimit,
            SidecarErrorKind::Internal => SdkErrorCode::Internal,
        }
    }

    /// Canonical SDK error envelope (directive X).
    pub fn to_sdk_error(&self) -> SdkError {
        SdkError::new(
            self.wire_code(),
            self.message.clone(),
            self.correlation_id.clone(),
            None,
            self.tenant.clone(),
            self.resource.clone(),
        )
    }

    /// HTTP status for this failure (wire-stable).
    pub fn http_status(&self) -> u16 {
        match self.kind {
            SidecarErrorKind::Validation => 400,
            SidecarErrorKind::Unavailable => 503,
            SidecarErrorKind::Timeout => 504,
            SidecarErrorKind::ProviderError => 502,
            SidecarErrorKind::CredentialDenied => 403,
            SidecarErrorKind::ProtocolVersionMismatch => 426,
            SidecarErrorKind::PayloadTooLarge => 413,
            SidecarErrorKind::ResponseTooLarge => 502,
            SidecarErrorKind::WebhookRejected => 401,
            SidecarErrorKind::PollerCorrupt => 500,
            SidecarErrorKind::Overloaded => 429,
            SidecarErrorKind::Internal => 500,
        }
    }
}

impl std::fmt::Display for SidecarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sidecar error: {}", self.message)
    }
}

impl std::error::Error for SidecarError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep011_unit_sidecar_error_wire_codes_are_canonical() {
        let cases = [
            (SidecarErrorKind::Validation, SdkErrorCode::Validation),
            (SidecarErrorKind::Unavailable, SdkErrorCode::Unavailable),
            (SidecarErrorKind::Timeout, SdkErrorCode::Timeout),
            (
                SidecarErrorKind::ProviderError,
                SdkErrorCode::ExternalProvider,
            ),
            (
                SidecarErrorKind::CredentialDenied,
                SdkErrorCode::Authorization,
            ),
            (
                SidecarErrorKind::ProtocolVersionMismatch,
                SdkErrorCode::Validation,
            ),
            (SidecarErrorKind::PayloadTooLarge, SdkErrorCode::Validation),
            (SidecarErrorKind::ResponseTooLarge, SdkErrorCode::Validation),
            (
                SidecarErrorKind::WebhookRejected,
                SdkErrorCode::Verification,
            ),
            (SidecarErrorKind::PollerCorrupt, SdkErrorCode::Validation),
            (SidecarErrorKind::Overloaded, SdkErrorCode::RateLimit),
            (SidecarErrorKind::Internal, SdkErrorCode::Internal),
        ];
        for (kind, expected) in cases {
            let err = SidecarError::new(kind, "x", None, None, None);
            assert_eq!(err.wire_code(), expected);
        }
    }

    #[test]
    fn ep011_unit_sidecar_error_serializes_canonical_envelope() {
        let err = SidecarError::new(
            SidecarErrorKind::CredentialDenied,
            "credential scope denied",
            Some("corr-1".to_string()),
            Some("tenant-1".to_string()),
            Some("fixture-connector".to_string()),
        );
        let json = serde_json::to_value(err.to_sdk_error()).unwrap();
        assert_eq!(json["code"], "AUTHORIZATION");
        assert_eq!(json["message"], "credential scope denied");
        assert_eq!(json["correlation_id"], "corr-1");
        assert_eq!(json["tenant"], "tenant-1");
    }
}
