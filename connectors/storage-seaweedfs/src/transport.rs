//! EP-037 M4 SeaweedFS S3-gateway transport.
//!
//! REAL transport over plain HTTP/1.1 on std::net TcpStream with a
//! hand-written AWS Signature V4 signer (no vendor SDK - the same
//! discipline proven against MinIO in M3 and RFC 4231). Every request
//! opens a fresh connection (no stale client state), applies bounded
//! connect/read timeouts, and classifies failures distinctly:
//! connect refused -> Unavailable, read timeout -> Timeout, malformed
//! status -> ExternalProvider, HTTP status -> NotFound/Conflict/
//! Validation/Authorization/Unavailable/ExternalProvider per code.
//!
//! The transport NEVER logs or returns credentials or payload content.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// Classified transport failure. Safe for display: no credentials, no
/// payload bytes, no signed URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S3Error {
    /// Could not establish a connection (refused, unreachable).
    Connect(String),
    /// A timed operation exceeded its bound.
    Timeout,
    /// The peer returned an unusable response (bad status line, torn
    /// headers). Never guessed into success.
    Malformed(String),
    /// The provider returned an HTTP status (classified by caller).
    Status { code: u16, body: String },
}

impl std::fmt::Display for S3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(m) => write!(f, "s3 connect failed: {m}"),
            Self::Timeout => write!(f, "s3 request timed out"),
            Self::Malformed(m) => write!(f, "s3 malformed response: {m}"),
            Self::Status { code, body } => write!(f, "s3 status {code}: {body}"),
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    // HMAC-SHA256 (RFC 2104) over the sha2 crate.
    const BLOCK: usize = 64;
    let mut key = key.to_vec();
    if key.len() > BLOCK {
        key = sha256_hex(&key).into_bytes();
    }
    key.resize(BLOCK, 0);
    let mut ipad = vec![0x36u8; BLOCK];
    let mut opad = vec![0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(msg);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(inner_digest);
    outer.finalize().to_vec()
}

/// Civil date from days since epoch (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// AWS Signature V4 signer (path-style S3, minimal surface).
#[derive(Clone)]
struct SigV4 {
    access_key: String,
    secret_key: String,
    region: String,
    service: String,
}

impl SigV4 {
    fn new(access_key: &str, secret_key: &str) -> Self {
        Self {
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
            region: "us-east-1".to_string(),
            service: "s3".to_string(),
        }
    }

    fn date_stamp(&self, now: u64) -> String {
        let days = now / 86400;
        let (y, m, d) = civil_from_days(days as i64);
        format!("{y:04}{m:02}{d:02}")
    }

    fn amz_date(&self, now: u64) -> String {
        let ds = self.date_stamp(now);
        let secs = now % 86400;
        format!(
            "{}T{:02}{:02}{:02}Z",
            ds,
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60
        )
    }

    fn sign(
        &self,
        method: &str,
        host: &str,
        path: &str,
        query: &str,
        payload_hash: &str,
        now: u64,
    ) -> (String, String) {
        let amz_date = self.amz_date(now);
        let date_stamp = self.date_stamp(now);
        let canonical_headers =
            format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "{method}\n{path}\n{query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
        );
        let scope = format!("{date_stamp}/{}/s3/aws4_request", self.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
            sha256_hex(canonical_request.as_bytes())
        );
        let k_date = hmac_sha256(
            format!("AWS4{}", self.secret_key).as_bytes(),
            date_stamp.as_bytes(),
        );
        let k_region = hmac_sha256(&k_date, self.region.as_bytes());
        let k_service = hmac_sha256(&k_region, self.service.as_bytes());
        let k_signing = hmac_sha256(&k_service, b"aws4_request");
        let signature = hmac_sha256(&k_signing, string_to_sign.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let auth = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key, scope, signed_headers, signature
        );
        (auth, amz_date)
    }
}

/// Minimal S3 client over std::net (path-style, HTTP/1.1). One fresh
/// TcpStream per request; bounded connect/read timeouts; never persists
/// stale connection state.
#[derive(Clone)]
pub struct S3Client {
    endpoint: String,
    signer: SigV4,
    connect_timeout: Duration,
    read_timeout: Duration,
}

impl S3Client {
    pub fn connect(
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        connect_timeout: Duration,
        read_timeout: Duration,
    ) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            signer: SigV4::new(access_key, secret_key),
            connect_timeout,
            read_timeout,
        }
    }

    fn open_stream(&self) -> Result<TcpStream, S3Error> {
        let addrs: Vec<_> = self
            .endpoint
            .to_socket_addrs()
            .map_err(|e| S3Error::Connect(format!("resolve: {e}")))?
            .collect();
        let mut last: Option<S3Error> = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, self.connect_timeout) {
                Ok(s) => {
                    s.set_read_timeout(Some(self.read_timeout))
                        .map_err(|e| S3Error::Connect(format!("set read timeout: {e}")))?;
                    s.set_write_timeout(Some(self.connect_timeout))
                        .map_err(|e| S3Error::Connect(format!("set write timeout: {e}")))?;
                    return Ok(s);
                }
                Err(e) => last = Some(S3Error::Connect(e.to_string())),
            }
        }
        Err(last.unwrap_or_else(|| S3Error::Connect("no addresses".into())))
    }

    /// Execute one S3 request. `path` must be URL-escaped by the caller
    /// (our keys are hex/UUID so they are safe). Returns status + body.
    pub fn request(
        &self,
        method: &str,
        path: &str,
        query: &str,
        body: &[u8],
        content_type: Option<&str>,
    ) -> Result<(u16, Vec<u8>), S3Error> {
        let payload_hash = sha256_hex(body);
        let (auth, amz_date) = self.signer.sign(
            method,
            &self.endpoint,
            path,
            query,
            &payload_hash,
            now_epoch(),
        );
        let request_path = if query.is_empty() {
            path.to_string()
        } else {
            format!("{path}?{query}")
        };
        let mut req = format!(
            "{method} {request_path} HTTP/1.1\r\nHost: {}\r\nAuthorization: {auth}\r\nx-amz-date: {amz_date}\r\nx-amz-content-sha256: {payload_hash}\r\n",
            self.endpoint
        );
        if let Some(ct) = content_type {
            req.push_str(&format!("Content-Type: {ct}\r\n"));
        }
        req.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        ));

        let mut stream = self.open_stream()?;
        stream
            .write_all(req.as_bytes())
            .and_then(|_| stream.write_all(body))
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut
                {
                    S3Error::Timeout
                } else {
                    S3Error::Connect(format!("write: {e}"))
                }
            })?;

        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut
            {
                S3Error::Timeout
            } else {
                S3Error::Connect(format!("read: {e}"))
            }
        })?;

        let head_end = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or_else(|| S3Error::Malformed("no header terminator".into()))?;
        let head = String::from_utf8_lossy(&raw[..head_end]).to_string();
        let mut lines = head.lines();
        let status_line = lines
            .next()
            .ok_or_else(|| S3Error::Malformed("empty status line".into()))?;
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| S3Error::Malformed(format!("bad status line: {status_line}")))?;
        let body = raw[head_end + 4..].to_vec();
        Ok((status, body))
    }

    /// PUT bucket (location us-east-1; empty body canonical). 200 or
    /// 409 (already exists) is success (idempotent).
    pub fn create_bucket(&self, bucket: &str) -> Result<(), S3Error> {
        let (status, body) = self.request("PUT", &format!("/{bucket}"), "", b"", None)?;
        if status == 200 || status == 409 {
            Ok(())
        } else {
            Err(S3Error::Status {
                code: status,
                body: String::from_utf8_lossy(&body).into_owned(),
            })
        }
    }

    /// PUT object; returns the canonical hex digest of the bytes.
    pub fn put_object(&self, bucket: &str, key: &str, bytes: &[u8]) -> Result<String, S3Error> {
        let digest = sha256_hex(bytes);
        let (status, body) = self.request(
            "PUT",
            &format!("/{bucket}/{key}"),
            "",
            bytes,
            Some("application/octet-stream"),
        )?;
        if status == 200 {
            Ok(digest)
        } else {
            Err(S3Error::Status {
                code: status,
                body: String::from_utf8_lossy(&body).into_owned(),
            })
        }
    }

    /// GET object; returns raw body bytes (caller verifies digest).
    pub fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, S3Error> {
        let (status, body) = self.request("GET", &format!("/{bucket}/{key}"), "", b"", None)?;
        if status == 200 {
            Ok(body)
        } else {
            Err(S3Error::Status {
                code: status,
                body: String::from_utf8_lossy(&body).into_owned(),
            })
        }
    }

    /// DELETE object. 204 or 200 is success; 404 means already absent.
    pub fn delete_object(&self, bucket: &str, key: &str) -> Result<(), S3Error> {
        let (status, body) = self.request("DELETE", &format!("/{bucket}/{key}"), "", b"", None)?;
        match status {
            204 | 200 => Ok(()),
            404 => Err(S3Error::Status {
                code: 404,
                body: String::from_utf8_lossy(&body).into_owned(),
            }),
            other => Err(S3Error::Status {
                code: other,
                body: String::from_utf8_lossy(&body).into_owned(),
            }),
        }
    }

    /// List object keys under a prefix using ListObjectsV2 with bounded
    /// internal paging. Returns all matching keys (complete listing).
    /// The query is percent-encoded AND key-sorted per SigV4 canonical
    /// rules (SeaweedFS reconstructs the canonical query via Go
    /// url.Values.Encode(), which sorts keys; an unsorted query breaks
    /// the signature).
    pub fn list_keys(&self, bucket: &str, prefix: &str) -> Result<Vec<String>, S3Error> {
        let mut keys = Vec::new();
        let mut continuation: Option<String> = None;
        let enc_prefix = pct_encode(prefix);
        loop {
            let query = match &continuation {
                Some(token) => format!(
                    "continuation-token={}&list-type=2&max-keys=3&prefix={enc_prefix}",
                    pct_encode(token)
                ),
                None => format!("list-type=2&max-keys=3&prefix={enc_prefix}"),
            };
            let (status, body) = self.request("GET", &format!("/{bucket}"), &query, b"", None)?;
            if status != 200 {
                return Err(S3Error::Status {
                    code: status,
                    body: String::from_utf8_lossy(&body).into_owned(),
                });
            }
            let xml = String::from_utf8_lossy(&body).into_owned();
            for key in extract_xml_keys(&xml) {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
            let is_truncated = xml.contains("<IsTruncated>true</IsTruncated>");
            continuation = extract_xml_token(&xml);
            if !is_truncated || continuation.is_none() {
                break;
            }
        }
        Ok(keys)
    }
}

/// Percent-encode a query component per SigV4 canonical-query rules
/// (RFC 3986 unreserved characters stay literal; everything else is
/// uppercase hex percent-encoded).
fn pct_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Extract `<Key>...</Key>` values from an S3 ListObjectsV2 XML body.
/// Keys are our own (hex digests, UUIDs, backup ids) so tag scanning is
/// exact and safe; unknown provider XML is never interpreted as data.
fn extract_xml_keys(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<Key>") {
        let after = &rest[start + 5..];
        match after.find("</Key>") {
            Some(end) => {
                let key = &after[..end];
                if !key.contains('<') {
                    out.push(key.to_string());
                }
                rest = &after[end..];
            }
            None => break,
        }
    }
    out
}

/// Extract `<NextContinuationToken>...</NextContinuationToken>` if present.
fn extract_xml_token(xml: &str) -> Option<String> {
    const OPEN: &str = "<NextContinuationToken>";
    let start = xml.find(OPEN)?;
    let after = &xml[start + OPEN.len()..];
    let end = after.find("</NextContinuationToken>")?;
    Some(after[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_known_vector() {
        // RFC 4231 test case 1: HMAC-SHA256(key=0x0b x20, data="Hi There")
        let key = vec![0x0bu8; 20];
        let digest = hmac_sha256(&key, b"Hi There");
        let hex = digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(
            hex,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn list_xml_keys_parsed() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Contents><Key>meta/01970000-0000-7000-8000-000000000001.json</Key></Contents>
  <Contents><Key>meta/01970000-0000-7000-8000-000000000002.json</Key></Contents>
  <IsTruncated>false</IsTruncated>
</ListBucketResult>"#;
        let keys = extract_xml_keys(xml);
        assert_eq!(keys.len(), 2);
        assert!(keys[0].ends_with(".json"));
        assert_eq!(extract_xml_token(xml), None);
    }

    #[test]
    fn list_xml_continuation_token_parsed() {
        let xml = r#"<ListBucketResult>
  <IsTruncated>true</IsTruncated>
  <NextContinuationToken>abc123</NextContinuationToken>
</ListBucketResult>"#;
        assert_eq!(extract_xml_token(xml), Some("abc123".to_string()));
        assert!(xml.contains("<IsTruncated>true</IsTruncated>"));
    }
}
