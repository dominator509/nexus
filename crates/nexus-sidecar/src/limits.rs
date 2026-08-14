//! Bounded resource limits for the sidecar boundary (directive D/T/U).
//!
//! The M3 transport locked a 64 KiB request size; M4 preserves that
//! exact bound and adds a response bound plus phase-specific timeouts.
//! All limits are configurable at process start and never change
//! during a run, so behavior is deterministic.

use std::time::Duration;

/// Locked canonical request size bound (directive D: preserve the
/// exact M3 limit).
pub const MAX_REQUEST_BYTES: u64 = 64 * 1024;

/// Response bound enforced on provider payloads relayed to clients
/// (directive D/J.8: bounded rejection of oversized responses).
pub const MAX_RESPONSE_BYTES: u64 = 64 * 1024;

/// Maximum accepted correlation-id length (directive V).
pub const MAX_CORRELATION_ID_LEN: usize = 128;

/// Maximum accepted request-id length (directive V parity).
pub const MAX_REQUEST_ID_LEN: usize = 128;

/// Maximum accepted idempotency-key length.
pub const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;

/// Maximum accepted checkpoint cursor length (directive R).
pub const MAX_CURSOR_LEN: usize = 64;

/// Default client read timeout: how long the sidecar waits for the
/// caller to finish uploading a request body (directive U).
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Default provider timeout: how long the sidecar waits for the
/// provider process to respond (directive U/J.5).
pub const DEFAULT_PROVIDER_TIMEOUT: Duration = Duration::from_secs(5);

/// Default concurrency bound: concurrent in-flight dispatches the
/// sidecar accepts; excess is a typed overload (directive T).
pub const DEFAULT_MAX_CONCURRENCY: usize = 16;

/// Structured resource limits with deterministic defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum request body bytes (locked at 64 KiB).
    pub max_request_bytes: u64,
    /// Maximum provider response bytes relayed to clients.
    pub max_response_bytes: u64,
    /// Maximum concurrent in-flight dispatches.
    pub max_concurrency: usize,
    /// Timeout for the caller to finish uploading a request.
    pub read_timeout: Duration,
    /// Timeout for the provider to answer a dispatch.
    pub provider_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_request_bytes: MAX_REQUEST_BYTES,
            max_response_bytes: MAX_RESPONSE_BYTES,
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
            read_timeout: DEFAULT_READ_TIMEOUT,
            provider_timeout: DEFAULT_PROVIDER_TIMEOUT,
        }
    }
}

impl Limits {
    /// Construct a limits set, clamping to the locked bounds.
    pub fn new(
        max_request_bytes: u64,
        max_response_bytes: u64,
        max_concurrency: usize,
        read_timeout: Duration,
        provider_timeout: Duration,
    ) -> Self {
        Self {
            max_request_bytes: max_request_bytes.min(MAX_REQUEST_BYTES),
            max_response_bytes: max_response_bytes.clamp(1, MAX_RESPONSE_BYTES * 4),
            max_concurrency: max_concurrency.max(1),
            read_timeout,
            provider_timeout,
        }
    }
}

/// Validate a correlation id (directive V): bounded length, no
/// newlines or control characters, printable ASCII only.
pub fn validate_correlation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CORRELATION_ID_LEN
        && value
            .chars()
            .all(|c| !c.is_control() && !c.is_whitespace() && c.is_ascii_graphic())
}

/// Validate a request id with the same policy as correlation ids.
pub fn validate_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REQUEST_ID_LEN
        && value
            .chars()
            .all(|c| !c.is_control() && !c.is_whitespace() && c.is_ascii_graphic())
}

/// Validate an idempotency key: bounded, printable ASCII, no control.
pub fn validate_idempotency_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDEMPOTENCY_KEY_LEN
        && value
            .chars()
            .all(|c| !c.is_control() && c.is_ascii_graphic())
}

/// Validate a poller cursor: bounded printable ASCII, digits allowed
/// (checkpoint cursors are non-negative integers in the owned poller).
pub fn validate_cursor(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_CURSOR_LEN && value.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep011_unit_sidecar_limits_defaults_are_locked() {
        let limits = Limits::default();
        assert_eq!(limits.max_request_bytes, 64 * 1024);
        assert_eq!(limits.max_response_bytes, 64 * 1024);
        assert!(limits.max_concurrency >= 1);
    }

    #[test]
    fn ep011_unit_sidecar_limits_clamp_request_bound() {
        let limits = Limits::new(
            1024 * 1024,
            128,
            4,
            Duration::from_secs(1),
            Duration::from_secs(1),
        );
        assert_eq!(limits.max_request_bytes, MAX_REQUEST_BYTES);
    }

    #[test]
    fn ep011_unit_sidecar_correlation_rejects_injection() {
        assert!(validate_correlation_id(
            "018f0f6f-9c1e-7b6e-8000-000000000002"
        ));
        assert!(!validate_correlation_id("ok\nX-Injected: true"));
        assert!(!validate_correlation_id("ok\r\nheader"));
        assert!(!validate_correlation_id("has space"));
        assert!(!validate_correlation_id(""));
        assert!(!validate_correlation_id(&"x".repeat(129)));
    }

    #[test]
    fn ep011_unit_sidecar_cursor_digits_only() {
        assert!(validate_cursor("0"));
        assert!(validate_cursor("42"));
        assert!(!validate_cursor("-1"));
        assert!(!validate_cursor("../../etc/passwd"));
        assert!(!validate_cursor("12\n34"));
    }
}
