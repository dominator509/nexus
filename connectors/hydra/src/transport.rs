//! EP-028 Hydra transport (M2): real HTTP transport over the
//! authenticated Hydra REST surface.
//!
//! Hydra is the CRM canonical source; Nexus orchestrates the provider
//! API and normalizes provider payloads at this infrastructure
//! boundary - free-form Hydra JSON never becomes a domain contract.
//!
//! The canonical transport surface is versioned with the M1 contract
//! shapes:
//! - GET  {base}/v1/context           read authorized business context
//! - GET  {base}/v1/capabilities      advertised capabilities
//! - POST {base}/v1/actions           submit a governed action
//! - GET  {base}/v1/actions/{id}      exact-target readback
//!
//! HTTP status mapping follows SPEC-006: 401/403 -> Authorization,
//! 404 -> NotFound, 409 -> Conflict, 429 -> RateLimit, 500/502/503 ->
//! Unavailable, silent peer -> Timeout, refused -> Unavailable,
//! malformed JSON -> External (fail closed). The bearer credential is
//! used ONLY for the header and never appears in errors or telemetry.

use std::time::Duration;

use nexus_hydra::{
    BusinessContext, HydraCapabilityKind, HydraCapabilityMap, HydraContextProjection, HydraError,
    HydraErrorCode,
};

/// Canonical provider capability advertisement (documented Hydra
/// shape, normalized at the boundary).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HydraCapabilityAd {
    pub kind: String,
    pub available: bool,
}

/// Canonical provider action envelope (documented Hydra shape).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HydraActionEnvelope {
    pub action_id: String,
    pub state: String,
}

/// Canonical provider event envelope (durable events; payload is
/// referenced, never inlined).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HydraProviderEvent {
    pub event_id: String,
    pub event_type: String,
    pub tenant_id: String,
    pub correlation: Option<String>,
    pub payload_ref: String,
    pub occurred_at: String,
    pub version: u32,
}

/// The Hydra transport port. Default implementations fail closed
/// (Unavailable) so an unbound transport never fabricates a session.
pub trait HydraTransport {
    /// Read the authorized business context projection.
    fn read_context(
        &self,
        context: &BusinessContext,
    ) -> Result<HydraContextProjection, HydraError> {
        let _ = context;
        Err(HydraError::new(
            HydraErrorCode::Unavailable,
            "hydra transport has no implementation bound",
            None,
            None,
            None,
            None,
        ))
    }

    /// Fetch the provider-advertised capability map.
    fn capabilities(&self) -> Result<HydraCapabilityMap, HydraError> {
        Err(HydraError::new(
            HydraErrorCode::Unavailable,
            "hydra transport has no implementation bound",
            None,
            None,
            None,
            None,
        ))
    }

    /// Submit a governed action and return the provider state.
    fn submit_action(&self, action: &serde_json::Value) -> Result<String, HydraError> {
        let _ = action;
        Err(HydraError::new(
            HydraErrorCode::Unavailable,
            "hydra transport has no implementation bound",
            None,
            None,
            None,
            None,
        ))
    }

    /// Read back one action by id (exact-target readback).
    fn read_action(&self, action_id: &str) -> Result<String, HydraError> {
        let _ = action_id;
        Err(HydraError::new(
            HydraErrorCode::Unavailable,
            "hydra transport has no implementation bound",
            None,
            None,
            None,
            None,
        ))
    }
}

fn classify_status(status: reqwest::StatusCode) -> HydraErrorCode {
    match status.as_u16() {
        401 | 403 => HydraErrorCode::Authorization,
        404 => HydraErrorCode::NotFound,
        409 => HydraErrorCode::Conflict,
        429 => HydraErrorCode::RateLimit,
        500 | 502 | 503 | 504 => HydraErrorCode::Unavailable,
        _ => HydraErrorCode::ExternalProvider,
    }
}

/// Real blocking HTTP Hydra transport.
pub struct HttpHydraTransport {
    client: reqwest::blocking::Client,
    base_url: String,
    /// Authenticated bearer credential. Used ONLY for the Authorization
    /// header; never logged, never embedded in errors.
    credential: String,
}

impl HttpHydraTransport {
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

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::blocking::RequestBuilder {
        self.client
            .request(method, format!("{}{}", self.base_url, path))
            .bearer_auth(&self.credential)
    }
}

impl HydraTransport for HttpHydraTransport {
    fn read_context(
        &self,
        context: &BusinessContext,
    ) -> Result<HydraContextProjection, HydraError> {
        context.validate()?;
        let resp = match self.request(reqwest::Method::GET, "/v1/context").send() {
            Ok(r) => r,
            Err(e) if e.is_timeout() => {
                return Err(HydraError::new(
                    HydraErrorCode::Timeout,
                    "hydra context read timed out",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Err(e) if e.is_connect() => {
                return Err(HydraError::new(
                    HydraErrorCode::Unavailable,
                    "hydra context endpoint unreachable",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Err(_) => {
                return Err(HydraError::new(
                    HydraErrorCode::ExternalProvider,
                    "hydra context transport failure",
                    None,
                    None,
                    None,
                    None,
                ));
            }
        };
        if !resp.status().is_success() {
            return Err(HydraError::new(
                classify_status(resp.status()),
                "hydra context request failed",
                None,
                None,
                None,
                None,
            ));
        }
        resp.json::<HydraContextProjection>().map_err(|_| {
            HydraError::new(
                HydraErrorCode::ExternalProvider,
                "hydra context payload malformed",
                None,
                None,
                None,
                None,
            )
        })
    }

    fn capabilities(&self) -> Result<HydraCapabilityMap, HydraError> {
        let resp = match self
            .request(reqwest::Method::GET, "/v1/capabilities")
            .send()
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() => {
                return Err(HydraError::new(
                    HydraErrorCode::Timeout,
                    "hydra capabilities read timed out",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Err(e) if e.is_connect() => {
                return Err(HydraError::new(
                    HydraErrorCode::Unavailable,
                    "hydra capabilities endpoint unreachable",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Err(_) => {
                return Err(HydraError::new(
                    HydraErrorCode::ExternalProvider,
                    "hydra capabilities transport failure",
                    None,
                    None,
                    None,
                    None,
                ));
            }
        };
        if !resp.status().is_success() {
            return Err(HydraError::new(
                classify_status(resp.status()),
                "hydra capabilities request failed",
                None,
                None,
                None,
                None,
            ));
        }
        let ads: Vec<HydraCapabilityAd> = resp.json().map_err(|_| {
            HydraError::new(
                HydraErrorCode::ExternalProvider,
                "hydra capabilities payload malformed",
                None,
                None,
                None,
                None,
            )
        })?;
        let mut map = HydraCapabilityMap::new();
        for ad in ads {
            let Ok(kind) = ad.kind.parse::<HydraCapabilityKind>() else {
                // Unknown provider capability is never advertised
                // (fail closed; provider vocabulary cannot widen the
                // contract).
                continue;
            };
            map.advertise(
                kind,
                if ad.available {
                    nexus_domain::Availability::Available
                } else {
                    nexus_domain::Availability::Unavailable
                },
            );
        }
        Ok(map)
    }

    fn submit_action(&self, action: &serde_json::Value) -> Result<String, HydraError> {
        let resp = match self
            .request(reqwest::Method::POST, "/v1/actions")
            .json(action)
            .send()
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() => {
                return Err(HydraError::new(
                    HydraErrorCode::Timeout,
                    "hydra action submit timed out",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Err(e) if e.is_connect() => {
                return Err(HydraError::new(
                    HydraErrorCode::Unavailable,
                    "hydra actions endpoint unreachable",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Err(_) => {
                return Err(HydraError::new(
                    HydraErrorCode::ExternalProvider,
                    "hydra action transport failure",
                    None,
                    None,
                    None,
                    None,
                ));
            }
        };
        if !resp.status().is_success() {
            return Err(HydraError::new(
                classify_status(resp.status()),
                "hydra action request failed",
                None,
                None,
                None,
                None,
            ));
        }
        let env: HydraActionEnvelope = resp.json().map_err(|_| {
            HydraError::new(
                HydraErrorCode::ExternalProvider,
                "hydra action payload malformed",
                None,
                None,
                None,
                None,
            )
        })?;
        Ok(env.state)
    }

    fn read_action(&self, action_id: &str) -> Result<String, HydraError> {
        let resp = match self
            .request(
                reqwest::Method::GET,
                &format!("/v1/actions/{}", urlencode(action_id)),
            )
            .send()
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() => {
                return Err(HydraError::new(
                    HydraErrorCode::Timeout,
                    "hydra action readback timed out",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Err(e) if e.is_connect() => {
                return Err(HydraError::new(
                    HydraErrorCode::Unavailable,
                    "hydra actions endpoint unreachable",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Err(_) => {
                return Err(HydraError::new(
                    HydraErrorCode::ExternalProvider,
                    "hydra action readback transport failure",
                    None,
                    None,
                    None,
                    None,
                ));
            }
        };
        if !resp.status().is_success() {
            return Err(HydraError::new(
                classify_status(resp.status()),
                "hydra action readback failed",
                None,
                None,
                None,
                None,
            ));
        }
        let env: HydraActionEnvelope = resp.json().map_err(|_| {
            HydraError::new(
                HydraErrorCode::ExternalProvider,
                "hydra action readback payload malformed",
                None,
                None,
                None,
                None,
            )
        })?;
        Ok(env.state)
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn ep028_unit_urlencode_preserves_safe_and_escapes_rest() {
        assert_eq!(urlencode("abc-123_.~"), "abc-123_.~");
        assert_eq!(urlencode("a/b c"), "a%2Fb%20c");
    }

    #[test]
    fn ep028_unit_classify_status_maps_spec006() {
        assert_eq!(
            classify_status(reqwest::StatusCode::UNAUTHORIZED),
            HydraErrorCode::Authorization
        );
        assert_eq!(
            classify_status(reqwest::StatusCode::FORBIDDEN),
            HydraErrorCode::Authorization
        );
        assert_eq!(
            classify_status(reqwest::StatusCode::NOT_FOUND),
            HydraErrorCode::NotFound
        );
        assert_eq!(
            classify_status(reqwest::StatusCode::CONFLICT),
            HydraErrorCode::Conflict
        );
        assert_eq!(
            classify_status(reqwest::StatusCode::TOO_MANY_REQUESTS),
            HydraErrorCode::RateLimit
        );
        assert_eq!(
            classify_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR),
            HydraErrorCode::Unavailable
        );
        assert_eq!(
            classify_status(reqwest::StatusCode::BAD_GATEWAY),
            HydraErrorCode::Unavailable
        );
        assert_eq!(
            classify_status(reqwest::StatusCode::SERVICE_UNAVAILABLE),
            HydraErrorCode::Unavailable
        );
        assert_eq!(
            classify_status(reqwest::StatusCode::IM_A_TEAPOT),
            HydraErrorCode::ExternalProvider
        );
    }

    #[test]
    fn ep028_unit_unbound_transport_fails_closed() {
        struct Unbound;
        impl HydraTransport for Unbound {}
        let ctx = BusinessContext::portfolio(
            nexus_domain::TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            nexus_domain::PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        );
        let err = Unbound.read_context(&ctx).unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Unavailable);
        let err = Unbound.capabilities().unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Unavailable);
    }
}
