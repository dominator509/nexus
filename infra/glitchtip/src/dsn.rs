//! DSN handling for the GlitchTip/Sentry-compatible incident sink
//! (EP-038 M3; SPEC-007 behavior 3).
//!
//! The documented DSN shape (verified against the GlitchTip operator
//! guide and Sentry envelope documentation):
//!
//! ```text
//! https://<32-hex-public-key>@<host>/<numeric-project-id>
//! ```
//!
//! The public key is a credential: the DSN authenticates envelopes.
//! It is therefore treated as secret-shaped. We never print, log, or
//! otherwise emit the raw DSN or its public key; diagnostics expose
//! only a fingerprint (`fp:...`) or lengths.

use nexus_observability::model::short_fingerprint;

/// Parsed DSN. The raw DSN string is deliberately NOT stored here
/// once parsed -- only its parts, and the public key is never
/// displayed by any `Display` implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dsn {
    scheme: String,
    public_key: String,
    /// Host without trailing slash, may include port.
    host: String,
    project_id: u64,
}

impl Dsn {
    /// Parse a DSN in the documented form.
    ///
    /// Accepted: `https://<public-key>@<host>/<project-id>` (http is
    /// also accepted for local fixtures). Rejects malformed URLs,
    /// missing credentials, non-hex public keys, and non-numeric
    /// project ids.
    pub fn parse(raw: &str) -> Result<Self, DsnError> {
        let err = |reason: &'static str| DsnError {
            reason,
            fingerprint: short_fingerprint(raw),
        };
        let (scheme, rest) = match raw.split_once("://") {
            Some((s, r)) if s == "https" || s == "http" => (s, r),
            Some(_) => return Err(err("unsupported scheme")),
            None => return Err(err("missing scheme")),
        };
        let (creds_host, path) = match rest.split_once('/') {
            Some((c, p)) => (c, p),
            None => return Err(err("missing project id")),
        };
        let (public_key, host) = match creds_host.split_once('@') {
            Some((k, h)) => (k, h),
            None => return Err(err("missing public key")),
        };
        if public_key.is_empty() || host.is_empty() {
            return Err(err("empty credential or host"));
        }
        if public_key.len() != 32 || !public_key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(err("public key must be 32 hex characters"));
        }
        let project_id: u64 = path
            .trim_end_matches('/')
            .parse()
            .map_err(|_| err("project id must be numeric"))?;
        if project_id == 0 {
            return Err(err("project id must be nonzero"));
        }
        Ok(Self {
            scheme: scheme.to_string(),
            public_key: public_key.to_string(),
            host: host.to_string(),
            project_id,
        })
    }

    /// The envelope ingestion endpoint for this DSN:
    /// `POST {scheme}://{host}/api/{project_id}/envelope/`
    pub fn envelope_path(&self) -> String {
        format!("/api/{}/envelope/", self.project_id)
    }

    /// Host used for the TCP connection (may include a port).
    pub fn host(&self) -> &str {
        &self.host
    }

    /// The DSN scheme (`http` or `https`).
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// Whether this DSN requires TLS. An `https` DSN MUST negotiate
    /// TLS before any envelope byte is written; a plaintext send on an
    /// `https` DSN is the AUD-055 defect and fails closed.
    pub fn is_https(&self) -> bool {
        self.scheme == "https"
    }

    /// The numeric project id.
    pub fn project_id(&self) -> u64 {
        self.project_id
    }

    /// The public key -- the DSN credential. Only ever used for the
    /// `X-Sentry-Auth` header and the envelope `dsn` field; never
    /// rendered in diagnostics.
    pub(crate) fn public_key(&self) -> &str {
        &self.public_key
    }

    /// Full DSN (reconstructed) -- used only inside the envelope
    /// `dsn` header where the protocol requires self-authentication.
    pub(crate) fn full(&self) -> String {
        format!(
            "{}://{}@{}/{}",
            self.scheme, self.public_key, self.host, self.project_id
        )
    }

    /// Secret-safe diagnostic descriptor.
    pub fn describe(&self) -> String {
        format!(
            "DSN({}://[redacted]@{}/project:{})",
            self.scheme, self.host, self.project_id
        )
    }
}

impl std::fmt::Display for Dsn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render the public key.
        write!(f, "{}", self.describe())
    }
}

/// DSN parse/validation failure. Carries only a fingerprint of the
/// offending input, never the input itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DsnError {
    pub reason: &'static str,
    pub fingerprint: String,
}

impl std::fmt::Display for DsnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid DSN: {} ({})", self.reason, self.fingerprint)
    }
}

impl std::error::Error for DsnError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsn_parse_valid_https() {
        let dsn =
            Dsn::parse("https://0123456789abcdef0123456789abcdef@glitchtip.local/42").unwrap();
        assert_eq!(dsn.project_id(), 42);
        assert_eq!(dsn.envelope_path(), "/api/42/envelope/");
        assert!(dsn.host().contains("glitchtip.local"));
    }

    #[test]
    fn dsn_parse_valid_http_localhost_with_port() {
        let dsn = Dsn::parse("http://0123456789abcdef0123456789abcdef@127.0.0.1:8000/7").unwrap();
        assert_eq!(dsn.project_id(), 7);
        assert_eq!(dsn.envelope_path(), "/api/7/envelope/");
        assert!(dsn.host().contains("127.0.0.1:8000"));
    }

    #[test]
    fn dsn_parse_rejects_bad_public_key_length() {
        let err = Dsn::parse("https://short@glitchtip.local/42").unwrap_err();
        assert_eq!(err.reason, "public key must be 32 hex characters");
    }

    #[test]
    fn dsn_parse_rejects_non_hex_public_key() {
        let err =
            Dsn::parse("https://zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz@glitchtip.local/42").unwrap_err();
        assert_eq!(err.reason, "public key must be 32 hex characters");
    }

    #[test]
    fn dsn_parse_rejects_non_numeric_project_id() {
        let err =
            Dsn::parse("https://0123456789abcdef0123456789abcdef@glitchtip.local/abc").unwrap_err();
        assert_eq!(err.reason, "project id must be numeric");
    }

    #[test]
    fn dsn_parse_rejects_missing_credentials() {
        assert!(Dsn::parse("https://glitchtip.local/42").is_err());
    }

    #[test]
    fn dsn_parse_rejects_bad_scheme() {
        assert!(Dsn::parse("ftp://0123456789abcdef0123456789abcdef@glitchtip.local/42").is_err());
    }

    #[test]
    fn dsn_parse_rejects_zero_project() {
        assert!(Dsn::parse("https://0123456789abcdef0123456789abcdef@glitchtip.local/0").is_err());
    }

    #[test]
    fn dsn_display_never_contains_public_key() {
        let dsn =
            Dsn::parse("https://0123456789abcdef0123456789abcdef@glitchtip.local/42").unwrap();
        let rendered = dsn.to_string();
        assert!(!rendered.contains("0123456789abcdef0123456789abcdef"));
        assert!(rendered.contains("[redacted]"));
    }

    #[test]
    fn dsn_error_never_contains_raw_input() {
        let err = Dsn::parse("https://not-a-key@glitchtip.local/42").unwrap_err();
        assert!(!err.to_string().contains("not-a-key"));
        assert!(err.to_string().contains("fp:"));
    }
}
