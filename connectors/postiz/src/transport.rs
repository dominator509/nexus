//! EP-029 Postiz transport (M2): real HTTP transport over the
//! DOCUMENTED Postiz public API.
//!
//! Postiz is the isolated AGPL sidecar for scheduling and connector
//! breadth (SPEC-015 behavior 4). Nexus orchestrates its authenticated
//! public API and normalizes provider payloads at this infrastructure
//! boundary - free-form Postiz JSON never becomes a domain contract.
//!
//! Canonical transport surface (verified against the official Postiz
//! documentation, docs.postiz.com/public-api):
//! - GET  {base}/integrations             list connected integrations
//! - POST {base}/posts                    create a post (type:
//!   draft|schedule|now; date RFC3339; posts[].integration.id;
//!   posts[].value[].content; posts[].settings.__type)
//! - GET  {base}/posts                    list posts
//! - PUT  {base}/posts/change-status      change a post's status
//! - POST {base}/upload                   upload a file (multipart)
//!
//! Authentication: `Authorization: <api-key>` header (or OAuth2 token
//! starting with `pos_`). The credential is used ONLY for the header
//! and never appears in errors or telemetry.
//!
//! HTTP status mapping follows SPEC-006: 400 -> Validation, 401/403 ->
//! Authorization, 404 -> NotFound, 409 -> Conflict, 429 -> RateLimit
//! (documented: 90 create-post requests per hour, API_LIMIT env),
//! 500/502/503/504 -> Unavailable, silent peer -> Timeout, refused ->
//! Unavailable, malformed JSON -> External (fail closed).

use std::time::Duration;

use nexus_social::{SocialError, SocialErrorCode};

/// Documented Postiz integration shape (normalized at the boundary).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PostizIntegration {
    /// Provider-side integration id.
    pub id: String,
    /// Platform name (e.g. "instagram", "linkedin").
    pub name: String,
    /// Integration type/identifier (e.g. "Instagram", "X").
    pub identifier: String,
    /// Provider-advertised availability.
    pub available: bool,
}

/// Documented Postiz post reference returned by create post.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PostizPostRef {
    /// Provider-side post id.
    pub id: String,
    /// Provider-side post status (e.g. "scheduled", "published").
    pub status: String,
}

/// The Postiz transport port. Default implementations fail closed
/// (Unavailable) so an unbound transport never fabricates a session.
pub trait PostizTransport {
    /// List connected integrations (documented GET /integrations).
    fn list_integrations(&self) -> Result<Vec<PostizIntegration>, SocialError> {
        Err(SocialError::unavailable(
            "postiz transport has no implementation bound",
        ))
    }

    /// Create a post (documented POST /posts). `payload` is the
    /// normalized canonical body built by the adapter.
    fn create_post(&self, payload: &serde_json::Value) -> Result<PostizPostRef, SocialError> {
        let _ = payload;
        Err(SocialError::unavailable(
            "postiz transport has no implementation bound",
        ))
    }

    /// List posts (documented GET /posts).
    fn list_posts(&self) -> Result<Vec<PostizPostRef>, SocialError> {
        Err(SocialError::unavailable(
            "postiz transport has no implementation bound",
        ))
    }

    /// Change a post's status (documented PUT /posts/change-status).
    fn change_post_status(&self, post_id: &str, status: &str) -> Result<(), SocialError> {
        let _ = (post_id, status);
        Err(SocialError::unavailable(
            "postiz transport has no implementation bound",
        ))
    }
}

fn classify_status(status: reqwest::StatusCode) -> SocialErrorCode {
    match status.as_u16() {
        400 => SocialErrorCode::Validation,
        401 | 403 => SocialErrorCode::Authorization,
        404 => SocialErrorCode::NotFound,
        409 => SocialErrorCode::Conflict,
        429 => SocialErrorCode::RateLimit,
        500 | 502 | 503 | 504 => SocialErrorCode::Unavailable,
        _ => SocialErrorCode::ExternalProvider,
    }
}

/// Real blocking HTTP Postiz transport over the documented public API.
pub struct HttpPostizTransport {
    client: reqwest::blocking::Client,
    base_url: String,
    /// Authenticated API key (or `pos_` OAuth2 token). Used ONLY for
    /// the Authorization header; never logged, never embedded in
    /// errors.
    credential: String,
}

impl HttpPostizTransport {
    pub fn new(
        base_url: impl Into<String>,
        credential: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client");
        Self {
            client,
            base_url: base_url.into(),
            credential: credential.into(),
        }
    }

    fn get(&self, path: &str) -> Result<reqwest::blocking::Response, SocialError> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        self.client
            .get(&url)
            .header("Authorization", &self.credential)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    SocialError::new(
                        SocialErrorCode::Timeout,
                        "postiz transport timed out",
                        None,
                        None,
                        None,
                        None,
                    )
                } else if e.is_connect() {
                    SocialError::new(
                        SocialErrorCode::Unavailable,
                        "postiz transport refused connection",
                        None,
                        None,
                        None,
                        None,
                    )
                } else {
                    SocialError::new(
                        SocialErrorCode::ExternalProvider,
                        "postiz transport request failed",
                        None,
                        None,
                        None,
                        None,
                    )
                }
            })
    }

    fn post(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::blocking::Response, SocialError> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        self.client
            .post(&url)
            .header("Authorization", &self.credential)
            .json(body)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    SocialError::new(
                        SocialErrorCode::Timeout,
                        "postiz transport timed out",
                        None,
                        None,
                        None,
                        None,
                    )
                } else if e.is_connect() {
                    SocialError::new(
                        SocialErrorCode::Unavailable,
                        "postiz transport refused connection",
                        None,
                        None,
                        None,
                        None,
                    )
                } else {
                    SocialError::new(
                        SocialErrorCode::ExternalProvider,
                        "postiz transport request failed",
                        None,
                        None,
                        None,
                        None,
                    )
                }
            })
    }

    fn parse<T: serde::de::DeserializeOwned>(
        response: reqwest::blocking::Response,
    ) -> Result<T, SocialError> {
        let status = response.status();
        if !status.is_success() {
            return Err(SocialError::new(
                classify_status(status),
                format!("postiz transport returned HTTP {}", status.as_u16()),
                None,
                None,
                None,
                None,
            ));
        }
        response.json::<T>().map_err(|_| {
            SocialError::new(
                SocialErrorCode::ExternalProvider,
                "postiz transport returned malformed JSON",
                None,
                None,
                None,
                None,
            )
        })
    }
}

impl PostizTransport for HttpPostizTransport {
    fn list_integrations(&self) -> Result<Vec<PostizIntegration>, SocialError> {
        let response = self.get("/integrations")?;
        Self::parse(response)
    }

    fn create_post(&self, payload: &serde_json::Value) -> Result<PostizPostRef, SocialError> {
        let response = self.post("/posts", payload)?;
        Self::parse(response)
    }

    fn list_posts(&self) -> Result<Vec<PostizPostRef>, SocialError> {
        let response = self.get("/posts")?;
        Self::parse(response)
    }

    fn change_post_status(&self, post_id: &str, status: &str) -> Result<(), SocialError> {
        let body = serde_json::json!({
            "postId": post_id,
            "status": status,
        });
        let response = self.post("/posts/change-status", &body)?;
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(SocialError::new(
                classify_status(status),
                format!("postiz transport returned HTTP {}", status.as_u16()),
                None,
                None,
                None,
                None,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep029_unit_transport_fails_closed_unbound() {
        struct Unbound;
        impl PostizTransport for Unbound {}
        let t = Unbound;
        assert_eq!(
            t.list_integrations().unwrap_err().code,
            SocialErrorCode::Unavailable
        );
        assert_eq!(
            t.create_post(&serde_json::json!({})).unwrap_err().code,
            SocialErrorCode::Unavailable
        );
        assert_eq!(
            t.list_posts().unwrap_err().code,
            SocialErrorCode::Unavailable
        );
        assert_eq!(
            t.change_post_status("p-1", "published").unwrap_err().code,
            SocialErrorCode::Unavailable
        );
    }

    #[test]
    fn ep029_unit_transport_status_classification() {
        use reqwest::StatusCode;
        assert_eq!(
            classify_status(StatusCode::BAD_REQUEST),
            SocialErrorCode::Validation
        );
        assert_eq!(
            classify_status(StatusCode::UNAUTHORIZED),
            SocialErrorCode::Authorization
        );
        assert_eq!(
            classify_status(StatusCode::FORBIDDEN),
            SocialErrorCode::Authorization
        );
        assert_eq!(
            classify_status(StatusCode::NOT_FOUND),
            SocialErrorCode::NotFound
        );
        assert_eq!(
            classify_status(StatusCode::CONFLICT),
            SocialErrorCode::Conflict
        );
        assert_eq!(
            classify_status(StatusCode::TOO_MANY_REQUESTS),
            SocialErrorCode::RateLimit
        );
        assert_eq!(
            classify_status(StatusCode::INTERNAL_SERVER_ERROR),
            SocialErrorCode::Unavailable
        );
        assert_eq!(
            classify_status(StatusCode::BAD_GATEWAY),
            SocialErrorCode::Unavailable
        );
    }

    #[test]
    fn ep029_unit_transport_normalizes_documented_postiz_shape() {
        // The documented POST /posts response shape is normalized at
        // the boundary into PostizPostRef.
        let json = serde_json::json!({ "id": "post-123", "status": "scheduled" });
        let parsed: PostizPostRef = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.id, "post-123");
        assert_eq!(parsed.status, "scheduled");
        let integration = serde_json::json!({ "id": "ig-1", "name": "Instagram", "identifier": "Instagram", "available": true });
        let parsed_i: PostizIntegration = serde_json::from_value(integration).unwrap();
        assert_eq!(parsed_i.id, "ig-1");
    }
}
