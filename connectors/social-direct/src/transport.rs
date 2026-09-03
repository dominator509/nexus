//! EP-029 direct platform transport (M3): real HTTP transport over
//! the DOCUMENTED X API v2 surface (official direct API for strategic
//! gaps per SPEC-015 behavior 4: direct official APIs implement
//! strategic gaps).
//!
//! Canonical transport surface (verified against the official X API
//! v2 documentation, docs.x.com/x-api):
//! - GET  {base}/2/users/me                         authenticated user
//! - GET  {base}/2/users/{id}/mentions              mentions timeline
//!   (community/inbox source, paginated)
//! - GET  {base}/2/tweets/{id}?tweet.fields=public_metrics
//!   (analytics: like_count, retweet_count, reply_count,
//!   quote_count, impression_count, bookmark_count)
//! - POST {base}/2/tweets                           create a tweet
//!
//! Authentication: `Authorization: Bearer <token>` header. The token
//! is used ONLY for the header and never appears in errors or
//! telemetry.
//!
//! HTTP status mapping follows SPEC-006: 400 -> Validation, 401/403 ->
//! Authorization, 404 -> NotFound, 409 -> Conflict, 429 -> RateLimit,
//! 500/502/503/504 -> Unavailable, silent peer -> Timeout, refused ->
//! Unavailable, malformed JSON -> External (fail closed).

use std::time::Duration;

use nexus_social::{SocialError, SocialErrorCode};

/// Documented X API v2 user shape (normalized at the boundary).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct XUser {
    pub id: String,
    pub name: String,
    pub username: String,
}

/// Documented X API v2 mention shape (conversation/inbox source).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct XMention {
    pub id: String,
    pub text: String,
    pub author_id: String,
    pub created_at: Option<String>,
}

/// Documented X API v2 public metrics shape (analytics).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct XPublicMetrics {
    #[serde(default)]
    pub like_count: u64,
    #[serde(default)]
    pub retweet_count: u64,
    #[serde(default)]
    pub reply_count: u64,
    #[serde(default)]
    pub quote_count: u64,
    #[serde(default)]
    pub impression_count: u64,
    #[serde(default)]
    pub bookmark_count: u64,
}

/// Documented X API v2 tweet shape with optional public metrics.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct XTweet {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub public_metrics: XPublicMetrics,
}

/// Documented X API v2 create-tweet response (normalized at the
/// boundary).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct XCreateResponse {
    pub id: String,
    pub text: String,
}

/// The direct platform transport port. Default implementations fail
/// closed (Unavailable) so an unbound transport never fabricates a
/// session.
pub trait DirectPlatformTransport {
    /// Authenticated user (documented GET /2/users/me).
    fn me(&self) -> Result<XUser, SocialError> {
        Err(SocialError::unavailable(
            "direct platform transport has no implementation bound",
        ))
    }

    /// Mentions timeline for a user (documented GET
    /// /2/users/{id}/mentions; the community/inbox source).
    fn mentions(&self, user_id: &str) -> Result<Vec<XMention>, SocialError> {
        let _ = user_id;
        Err(SocialError::unavailable(
            "direct platform transport has no implementation bound",
        ))
    }

    /// Tweet with public metrics (documented GET
    /// /2/tweets/{id}?tweet.fields=public_metrics; analytics source).
    fn tweet_with_metrics(&self, tweet_id: &str) -> Result<XTweet, SocialError> {
        let _ = tweet_id;
        Err(SocialError::unavailable(
            "direct platform transport has no implementation bound",
        ))
    }

    /// Create a tweet (documented POST /2/tweets).
    fn create_tweet(&self, text: &str) -> Result<XCreateResponse, SocialError> {
        let _ = text;
        Err(SocialError::unavailable(
            "direct platform transport has no implementation bound",
        ))
    }

    /// Reply to an existing tweet (documented POST /2/tweets with the
    /// official `reply.in_reply_to_tweet_id` object). The reply is
    /// NEVER a standalone post: the thread reference is carried in
    /// the provider request (AUD-024). Fails closed when unbound.
    fn reply_to_tweet(
        &self,
        text: &str,
        in_reply_to_tweet_id: &str,
    ) -> Result<XCreateResponse, SocialError> {
        let _ = (text, in_reply_to_tweet_id);
        Err(SocialError::unavailable(
            "direct platform transport has no implementation bound",
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

/// Real blocking HTTP direct platform transport over the documented
/// X API v2 surface.
pub struct HttpDirectPlatformTransport {
    client: reqwest::blocking::Client,
    base_url: String,
    /// Bearer token. Used ONLY for the Authorization header; never
    /// logged, never embedded in errors.
    bearer_token: String,
}

impl HttpDirectPlatformTransport {
    pub fn new(
        base_url: impl Into<String>,
        bearer_token: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client");
        Self {
            client,
            base_url: base_url.into(),
            bearer_token: bearer_token.into(),
        }
    }

    fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::blocking::Response, SocialError> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self
            .client
            .request(method, &url)
            .header("Authorization", format!("Bearer {}", self.bearer_token));
        if let Some(body) = body {
            request = request.json(&body);
        }
        request.send().map_err(|e| {
            if e.is_timeout() {
                SocialError::new(
                    SocialErrorCode::Timeout,
                    "direct platform transport timed out",
                    None,
                    None,
                    None,
                    None,
                )
            } else if e.is_connect() {
                SocialError::new(
                    SocialErrorCode::Unavailable,
                    "direct platform transport refused connection",
                    None,
                    None,
                    None,
                    None,
                )
            } else {
                SocialError::new(
                    SocialErrorCode::ExternalProvider,
                    "direct platform transport request failed",
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
                format!(
                    "direct platform transport returned HTTP {}",
                    status.as_u16()
                ),
                None,
                None,
                None,
                None,
            ));
        }
        response.json::<T>().map_err(|_| {
            SocialError::new(
                SocialErrorCode::ExternalProvider,
                "direct platform transport returned malformed JSON",
                None,
                None,
                None,
                None,
            )
        })
    }
}

impl DirectPlatformTransport for HttpDirectPlatformTransport {
    fn me(&self) -> Result<XUser, SocialError> {
        // Documented response: { "data": { "id", "name", "username" } }
        let response = self.send(reqwest::Method::GET, "/2/users/me", None)?;
        let envelope: serde_json::Value = Self::parse(response)?;
        serde_json::from_value(
            envelope
                .get("data")
                .cloned()
                .ok_or_else(|| external_error("direct platform response missing data"))?,
        )
        .map_err(|_| external_error("direct platform response missing data"))
    }

    fn mentions(&self, user_id: &str) -> Result<Vec<XMention>, SocialError> {
        // Documented response: { "data": [ { "id", "text",
        // "author_id", "created_at" } ] }
        let path = format!("/2/users/{user_id}/mentions?max_results=100");
        let response = self.send(reqwest::Method::GET, &path, None)?;
        let envelope: serde_json::Value = Self::parse(response)?;
        let data = envelope
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![]));
        serde_json::from_value(data)
            .map_err(|_| external_error("direct platform response malformed mentions"))
    }

    fn tweet_with_metrics(&self, tweet_id: &str) -> Result<XTweet, SocialError> {
        // Documented response: { "data": { "id", "text",
        // "public_metrics": { ... } } } with
        // tweet.fields=public_metrics.
        let path = format!("/2/tweets/{tweet_id}?tweet.fields=public_metrics");
        let response = self.send(reqwest::Method::GET, &path, None)?;
        let envelope: serde_json::Value = Self::parse(response)?;
        serde_json::from_value(
            envelope
                .get("data")
                .cloned()
                .ok_or_else(|| external_error("direct platform response missing data"))?,
        )
        .map_err(|_| external_error("direct platform response missing data"))
    }

    fn create_tweet(&self, text: &str) -> Result<XCreateResponse, SocialError> {
        // Documented request body: { "text": "..." }
        let body = serde_json::json!({ "text": text });
        let response = self.send(reqwest::Method::POST, "/2/tweets", Some(body))?;
        let envelope: serde_json::Value = Self::parse(response)?;
        serde_json::from_value(
            envelope
                .get("data")
                .cloned()
                .ok_or_else(|| external_error("direct platform response missing data"))?,
        )
        .map_err(|_| external_error("direct platform response missing data"))
    }

    fn reply_to_tweet(
        &self,
        text: &str,
        in_reply_to_tweet_id: &str,
    ) -> Result<XCreateResponse, SocialError> {
        // Documented request body: { "text": "...", "reply": {
        // "in_reply_to_tweet_id": "..." } } (official X API v2
        // CreatePostsReply schema; the id is a 1..=19 digit string).
        // The reply object is REQUIRED for a reply - a reply without
        // it is a standalone post, which is exactly the AUD-024
        // defect. Fail closed on a missing/invalid thread reference.
        let trimmed = in_reply_to_tweet_id.trim();
        if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_digit()) || trimmed.len() > 19
        {
            return Err(SocialError::new(
                SocialErrorCode::Validation,
                "reply requires a valid in_reply_to_tweet_id (1..=19 digits)",
                None,
                None,
                None,
                None,
            ));
        }
        let body = serde_json::json!({
            "text": text,
            "reply": { "in_reply_to_tweet_id": trimmed },
        });
        let response = self.send(reqwest::Method::POST, "/2/tweets", Some(body))?;
        let envelope: serde_json::Value = Self::parse(response)?;
        serde_json::from_value(
            envelope
                .get("data")
                .cloned()
                .ok_or_else(|| external_error("direct platform response missing data"))?,
        )
        .map_err(|_| external_error("direct platform response missing data"))
    }
}

fn external_error(message: &str) -> SocialError {
    SocialError::new(
        SocialErrorCode::ExternalProvider,
        message,
        None,
        None,
        None,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep029_unit_direct_transport_fails_closed_unbound() {
        struct Unbound;
        impl DirectPlatformTransport for Unbound {}
        let t = Unbound;
        assert_eq!(t.me().unwrap_err().code, SocialErrorCode::Unavailable);
        assert_eq!(
            t.mentions("user-1").unwrap_err().code,
            SocialErrorCode::Unavailable
        );
        assert_eq!(
            t.tweet_with_metrics("t-1").unwrap_err().code,
            SocialErrorCode::Unavailable
        );
        assert_eq!(
            t.create_tweet("hello").unwrap_err().code,
            SocialErrorCode::Unavailable
        );
    }

    #[test]
    fn ep029_unit_direct_transport_status_classification() {
        use reqwest::StatusCode;
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
            classify_status(StatusCode::TOO_MANY_REQUESTS),
            SocialErrorCode::RateLimit
        );
        assert_eq!(
            classify_status(StatusCode::INTERNAL_SERVER_ERROR),
            SocialErrorCode::Unavailable
        );
    }

    #[test]
    fn ep029_unit_direct_transport_normalizes_documented_x_shapes() {
        let user = serde_json::json!({ "id": "u-1", "name": "Nexus", "username": "nexus" });
        let parsed: XUser = serde_json::from_value(user).unwrap();
        assert_eq!(parsed.username, "nexus");
        let mention = serde_json::json!({ "id": "m-1", "text": "hi", "author_id": "a-1" });
        let parsed_m: XMention = serde_json::from_value(mention).unwrap();
        assert_eq!(parsed_m.author_id, "a-1");
        let tweet = serde_json::json!({ "id": "t-1", "text": "hi", "public_metrics": { "like_count": 3, "impression_count": 10 } });
        let parsed_t: XTweet = serde_json::from_value(tweet).unwrap();
        assert_eq!(parsed_t.public_metrics.like_count, 3);
        assert_eq!(parsed_t.public_metrics.impression_count, 10);
    }
}
