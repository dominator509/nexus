//! EP-037 M3 backup/restore integration over a REAL S3-compatible
//! MinIO container (SPEC-024: MinIO is compatibility-only but a real
//! S3-compatible backend; the community repository is archived).
//!
//! The test client is a minimal AWS SigV4 signer over plain HTTP/1.1
//! (std::net TcpStream) - no vendor SDK, real network to the real
//! container. The gate script starts the MinIO container with a pinned
//! image and exports NEXUS_MINIO_* variables; this test refuses to run
//! without them (fail closed, never a silent skip).

use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

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

/// AWS Signature V4 signer for S3 (path-style, minimal surface).
struct SigV4 {
    access_key: String,
    secret_key: String,
    region: String,
    #[allow(dead_code)]
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
        // now = epoch seconds
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

    /// Sign a request and return the Authorization header value plus
    /// x-amz-date. `payload_hash` must be the hex SHA-256 of the body
    /// (empty body = sha256 of "").
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
        let k_service = hmac_sha256(&k_region, b"s3");
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

/// Minimal S3 client for integration tests (path-style, HTTP/1.1).
pub struct S3Client {
    endpoint: String, // host:port
    signer: SigV4,
    pub bucket: String,
}

impl S3Client {
    pub fn connect(endpoint: &str, access_key: &str, secret_key: &str, bucket: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            signer: SigV4::new(access_key, secret_key),
            bucket: bucket.to_string(),
        }
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
        content_type: Option<&str>,
    ) -> Result<(u16, Vec<u8>), String> {
        let payload_hash = sha256_hex(body);
        let (auth, amz_date) =
            self.signer
                .sign(method, &self.endpoint, path, "", &payload_hash, now_epoch());
        let mut req = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: {auth}\r\nx-amz-date: {amz_date}\r\nx-amz-content-sha256: {payload_hash}\r\n",
            self.endpoint
        );
        if let Some(ct) = content_type {
            req.push_str(&format!("Content-Type: {ct}\r\n"));
        }
        req.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        ));
        let mut stream = TcpStream::connect(&self.endpoint).map_err(|e| format!("connect: {e}"))?;
        stream
            .write_all(req.as_bytes())
            .and_then(|_| stream.write_all(body))
            .map_err(|e| format!("write: {e}"))?;
        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .map_err(|e| format!("read: {e}"))?;
        let head_end = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or_else(|| "no header terminator".to_string())?;
        let head = String::from_utf8_lossy(&raw[..head_end]).to_string();
        let status: u16 = head
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("bad status line: {head}"))?;
        let body = raw[head_end + 4..].to_vec();
        Ok((status, body))
    }

    /// PUT bucket (location default us-east-1; empty body is canonical).
    pub fn create_bucket(&self) -> Result<(), String> {
        let (status, body) = self.request("PUT", &format!("/{}", self.bucket), b"", None)?;
        if status == 200 || status == 409 {
            Ok(()) // 409 = already exists (idempotent)
        } else {
            Err(format!(
                "create_bucket status {status}: {}",
                String::from_utf8_lossy(&body)
            ))
        }
    }

    /// PUT object with a given key; returns the digest of the bytes.
    pub fn put_object(&self, key: &str, bytes: &[u8]) -> Result<String, String> {
        let digest = sha256_hex(bytes);
        let (status, body) = self.request(
            "PUT",
            &format!("/{}/{}", self.bucket, key),
            bytes,
            Some("application/octet-stream"),
        )?;
        if status == 200 {
            Ok(digest)
        } else {
            Err(format!(
                "put_object status {status}: {}",
                String::from_utf8_lossy(&body)
            ))
        }
    }

    /// GET object; verifies the returned bytes' digest matches `expect`.
    pub fn get_object(&self, key: &str, expect: &str) -> Result<Vec<u8>, String> {
        let (status, body) =
            self.request("GET", &format!("/{}/{}", self.bucket, key), b"", None)?;
        if status == 200 {
            let actual = sha256_hex(&body);
            if actual != *expect {
                return Err(format!(
                    "get_object digest mismatch: expected {expect}, got {actual}"
                ));
            }
            Ok(body)
        } else {
            Err(format!(
                "get_object status {status}: {}",
                String::from_utf8_lossy(&body)
            ))
        }
    }

    /// DELETE object.
    pub fn delete_object(&self, key: &str) -> Result<(), String> {
        let (status, body) =
            self.request("DELETE", &format!("/{}/{}", self.bucket, key), b"", None)?;
        if status == 204 || status == 200 {
            Ok(())
        } else {
            Err(format!(
                "delete_object status {status}: {}",
                String::from_utf8_lossy(&body)
            ))
        }
    }
}

/// Read required integration environment (fail closed - never skip).
pub fn minio_env() -> (String, String, String, String) {
    let endpoint = env::var("NEXUS_MINIO_ENDPOINT").expect("NEXUS_MINIO_ENDPOINT must be set");
    let access = env::var("NEXUS_MINIO_ACCESS_KEY").expect("NEXUS_MINIO_ACCESS_KEY must be set");
    let secret = env::var("NEXUS_MINIO_PW_KEY").expect("NEXUS_MINIO_PW_KEY must be set");
    let bucket =
        env::var("NEXUS_MINIO_BUCKET").unwrap_or_else(|_| "nexus-backup-tests".to_string());
    (endpoint, access, secret, bucket)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigv4_civil_date_known_value() {
        // 2026-08-22T00:00:00Z = 1787356800
        let client = S3Client::connect("localhost:9000", "k", "s", "b");
        assert_eq!(client.signer.date_stamp(1787356800), "20260822");
    }

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
}
