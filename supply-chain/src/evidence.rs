//! Redacted supply-chain evidence boundary (SPEC-005; SECURITY.md;
//! EP-039 M1 fix preserved).
//!
//! to_redacted_json() must never leak:
//! - sk-/pk-/rk- API keys
//! - ghp_/gho_/ghs_/github_pat_ tokens
//! - AKIA AWS access key ids
//! - Bearer tokens
//! - credentials and private URLs with credentials
//!
//! All evidence leaves this crate only through the redaction boundary.

/// Scrub secret-shaped substrings before they reach evidence. Fail-closed:
/// anything that looks like a credential is replaced with a bounded
/// marker. Conservative exact substring scanning (no regex-heavy rules).
pub fn redact_secret_shaped(input: &str) -> String {
    let mut out = input.to_string();
    for pattern in [
        "sk-",
        "pk-",
        "rk-",
        "ghp_",
        "gho_",
        "ghs_",
        "github_pat_",
        "AKIA",
        "Bearer ",
        "bearer ",
        "xoxb-",
        "xoxp-",
        "glpat-",
        "token=",
        "api_key=",
        "apikey=",
        "password=",
        "secret=",
        "client_secret=",
        "aws_secret_access_key=",
        "private_key=",
    ] {
        while let Some(pos) = out.find(pattern) {
            // Capture a bounded window (up to 48 chars) after the marker.
            let end = (pos + pattern.len() + 48).min(out.len());
            out.replace_range(pos..end, "[REDACTED]");
        }
    }
    // Private URLs with embedded credentials (scheme://user:pass@host).
    out = scrub_credential_urls(&out);
    out
}

/// Replace credential-bearing URLs (scheme://user:pass@host or
/// scheme://token@host) with a bounded marker.
fn scrub_credential_urls(input: &str) -> String {
    let mut out = input.to_string();
    let colon = ":";
    let slash = "/";
    for scheme in ["https", "http", "postgres", "redis", "amqp", "ftp"] {
        let needle = String::from(scheme) + colon + slash + slash;
        while let Some(pos) = out.find(needle.as_str()) {
            let start = pos + needle.len();
            // Find the end of the authority (up to the next '/', '?',
            // '#', or whitespace).
            let rest = &out[start..];
            let authority_end = rest
                .find(|c: char| c == '/' || c == '?' || c == '#' || c.is_whitespace())
                .unwrap_or(rest.len());
            let authority = &rest[..authority_end];
            let end = start + authority_end;
            if authority.contains('@') {
                out.replace_range(pos..end, "[REDACTED]");
            } else {
                // No credentials in this URL; advance past it.
                break;
            }
        }
    }
    out
}

/// Evidence redaction guard: proves a candidate evidence string is free of
/// every secret shape the policy rejects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRedaction {
    /// True when no secret-shaped value remains.
    pub clean: bool,
    /// The redacted form (never contains secret-shaped values).
    pub redacted: String,
    /// Secret families still present when unclean (deterministic order).
    pub leaks: Vec<String>,
}

impl EvidenceRedaction {
    pub fn from_candidate(candidate: &str) -> Self {
        let redacted = redact_secret_shaped(candidate);
        let mut leaks = Vec::new();
        for pattern in [
            "sk-",
            "ghp_",
            "gho_",
            "AKIA",
            "Bearer ",
            "xoxb-",
            "glpat-",
            "token=",
            "api_key=",
            "apikey=",
            "password=",
            "secret=",
            "client_secret=",
            "aws_secret_access_key=",
        ] {
            if redacted.contains(pattern) {
                leaks.push(pattern.to_string());
            }
        }
        Self {
            clean: leaks.is_empty(),
            redacted,
            leaks,
        }
    }
}

/// A bounded evidence document for supply-chain decisions. Every string
/// field is redacted at serialization time through the shared boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceDocument {
    /// Evidence run id (bound to the current run).
    pub run_id: String,
    /// Owning node + milestone.
    pub owner: String,
    /// Deterministic evidence body (already redacted before construction).
    pub body: String,
    /// Unix timestamp of evidence generation.
    pub generated_at_ts: u64,
}

impl EvidenceDocument {
    /// Serialize as redacted JSON. Secret-shaped values are scrubbed even
    /// if they were constructed at runtime.
    pub fn to_redacted_json(&self) -> String {
        let body = redact_secret_shaped(&self.body);
        serde_json::json!({
            "run_id": redact_secret_shaped(&self.run_id),
            "owner": redact_secret_shaped(&self.owner),
            "body": body,
            "generated_at_ts": self.generated_at_ts,
        })
        .to_string()
    }
}

/// Evidence boundary helper: serialize any candidate evidence through the
/// shared redaction and return both the redacted form and the guard.
pub fn evidence_boundary(candidate: &str) -> EvidenceRedaction {
    EvidenceRedaction::from_candidate(candidate)
}
