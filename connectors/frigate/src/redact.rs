//! Secret redaction for Frigate/go2rtc URLs and error surfaces
//! (SPEC-021, SPEC-006, owner directive S).
//!
//! RTSP camera URLs commonly embed credentials (`rtsp://user:pass@host`).
//! Nothing in the adapter may log, serialize, or surface those
//! credentials. `redact_url` strips userinfo (and a small set of
//! sensitive query parameters) from any URL-like string.

/// Redact credentials from a URL-like string.
///
/// - `rtsp://user:pass@host/path` -> `rtsp://***@host/path`
/// - query params named `token`, `key`, `secret`, `password`, or
///   `api_key` are masked
/// - non-URL strings pass through unchanged (safe for error text)
pub fn redact_url(input: &str) -> String {
    let Some(url) = parse_url(input) else {
        return input.to_string();
    };
    let mut out = url;
    if take_userinfo(&mut out).is_some() {
        // userinfo replaced in place with a masked marker.
    }
    out = mask_query_secrets(&out);
    out
}

/// Split off userinfo (`user:pass@`) from a URL and replace it with a
/// masked marker. Returns the userinfo (discarded).
fn take_userinfo(url: &mut String) -> Option<String> {
    let scheme_end = url.find("://")? + 3;
    let at = url[scheme_end..].find('@')? + scheme_end;
    let userinfo = url[scheme_end..at].to_string();
    url.replace_range(scheme_end..at, "***");
    Some(userinfo)
}

/// Mask sensitive query parameters.
fn mask_query_secrets(url: &str) -> String {
    let Some(q) = url.find('?') else {
        return url.to_string();
    };
    let (head, query) = url.split_at(q);
    let mut params: Vec<String> = query[1..]
        .split('&')
        .map(|pair| {
            let mut it = pair.splitn(2, '=');
            let key = it.next().unwrap_or("");
            let val = it.next().unwrap_or("");
            if matches!(
                key,
                "token" | "key" | "secret" | "password" | "api_key" | "sig"
            ) {
                format!("{key}=***")
            } else {
                let _ = val;
                pair.to_string()
            }
        })
        .collect();
    params.sort();
    format!("{head}?{}", params.join("&"))
}

/// Minimal URL parser: finds the scheme and authority and normalizes a
/// string only when it clearly looks like an absolute URL.
fn parse_url(input: &str) -> Option<String> {
    let input = input.trim();
    let schemes = [
        "rtsp://", "rtsps://", "http://", "https://", "ws://", "wss://",
    ];
    if !schemes.iter().any(|s| input.starts_with(s)) {
        return None;
    }
    // Truncate at whitespace / quotes (error strings may embed URLs).
    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    let end = input[..end].find(['"', '\'', '>', ')', ']']).unwrap_or(end);
    Some(input[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep023_unit_frigate_redact_rtsp_credentials() {
        assert_eq!(
            redact_url("rtsp://user:secret@192.168.1.10:554/stream"),
            "rtsp://***@192.168.1.10:554/stream"
        );
    }

    #[test]
    fn ep023_unit_frigate_redact_http_basic_auth() {
        assert_eq!(
            redact_url("http://admin:hunter2@frigate.local/api/"),
            "http://***@frigate.local/api/"
        );
    }

    #[test]
    fn ep023_unit_frigate_redact_query_secrets() {
        let redacted = redact_url("http://frigate/api/events?token=abc&camera=front");
        assert!(!redacted.contains("abc"));
        assert!(redacted.contains("token=***"));
        assert!(redacted.contains("camera=front"));
    }

    #[test]
    fn ep023_unit_frigate_redact_plain_text_passes_through() {
        let text = "connection refused";
        assert_eq!(redact_url(text), text);
    }

    #[test]
    fn ep023_unit_frigate_redact_no_userinfo_no_change() {
        let url = "rtsp://192.168.1.10:554/stream";
        assert_eq!(redact_url(url), url);
    }
}
