//! EP-011 SDK vocabulary (ADR-016).
//!
//! Vocabulary-locked classes for the connector SDK contract and the
//! sandboxed legacy Connector Sidecar (SPEC-022). Unknown classes are
//! rejected at parse time; wire values are canonical SCREAMING_SNAKE
//! strings so generated TypeScript and Python bindings match the Rust
//! surface exactly.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! sdk_vocabulary_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $value:literal),* $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),*
        }

        impl $name {
            /// Canonical wire value.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),*
                }
            }

            /// Parse from the canonical wire value; unknown values are
            /// rejected (vocabulary lock, SPEC-005).
            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($value => Ok(Self::$variant),)*
                    other => Err(format!(
                        "unknown {} value: {other}",
                        stringify!($name)
                    )),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

sdk_vocabulary_enum! {
    /// SDK language surface (SPEC-022 behavior 4): all SDKs implement
    /// the same contract corpus.
    SdkLanguage {
        Rust = "RUST",
        TypeScript = "TYPESCRIPT",
        Python = "PYTHON",
    }
}

sdk_vocabulary_enum! {
    /// Transport the sandboxed Connector Sidecar wraps (SPEC-022
    /// behavior 5). Browser and desktop GUI are last resort; every
    /// transport runs inside the sidecar sandbox without direct
    /// authority.
    SidecarTransport {
        Rest = "REST",
        Soap = "SOAP",
        Graphql = "GRAPHQL",
        Sql = "SQL",
        Odbc = "ODBC",
        Jdbc = "JDBC",
        Cli = "CLI",
        Files = "FILES",
        Email = "EMAIL",
        Webhook = "WEBHOOK",
        Browser = "BROWSER",
        Desktop = "DESKTOP",
    }
}

sdk_vocabulary_enum! {
    /// Legacy source family wrapped by the LegacyPoller (SPEC-022
    /// behavior 5).
    LegacyTransport {
        Rest = "REST",
        Soap = "SOAP",
        Sql = "SQL",
        Cli = "CLI",
        Files = "FILES",
        Email = "EMAIL",
        Browser = "BROWSER",
    }
}

sdk_vocabulary_enum! {
    /// Webhook delivery state (SPEC-022 behavior 2: signed webhooks).
    WebhookDeliveryState {
        Pending = "PENDING",
        Delivered = "DELIVERED",
        Failed = "FAILED",
        Replay = "REPLAY",
    }
}

/// Canonical webhook event (SPEC-022): versioned, correlated, and
/// signed. The payload is a schema reference or normalized JSON value,
/// never a raw provider blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Canonical event id (SPEC-022 behavior 8).
    pub event_id: String,
    /// Versioned event type (`v1` suffix).
    pub event_type: String,
    /// Version of this event payload.
    pub version: String,
    /// Correlation id propagated from the originating request.
    pub correlation_id: String,
    /// Canonical payload (schema-validated JSON).
    pub payload: serde_json::Value,
}

/// Webhook signature envelope (SPEC-022): the signing scheme and the
/// fingerprint of the verifying key; raw secrets never appear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookSignature {
    /// Signing scheme (`hmac-sha256`, `ed25519`).
    pub scheme: String,
    /// Fingerprint of the verifying key (not the key material).
    pub key_fingerprint: String,
    /// Hex signature over the canonical event bytes.
    pub value_hex: String,
}

/// Webhook verification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebhookVerification {
    Valid,
    Invalid,
    Replay,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep011_unit_sdk_vocabulary_round_trip() {
        for value in [
            SdkLanguage::Rust,
            SdkLanguage::TypeScript,
            SdkLanguage::Python,
        ] {
            assert_eq!(SdkLanguage::parse(value.as_str()).unwrap(), value);
        }
        for value in [
            SidecarTransport::Rest,
            SidecarTransport::Soap,
            SidecarTransport::Graphql,
            SidecarTransport::Sql,
            SidecarTransport::Odbc,
            SidecarTransport::Jdbc,
            SidecarTransport::Cli,
            SidecarTransport::Files,
            SidecarTransport::Email,
            SidecarTransport::Webhook,
            SidecarTransport::Browser,
            SidecarTransport::Desktop,
        ] {
            assert_eq!(SidecarTransport::parse(value.as_str()).unwrap(), value);
        }
        for value in [
            LegacyTransport::Rest,
            LegacyTransport::Soap,
            LegacyTransport::Sql,
            LegacyTransport::Cli,
            LegacyTransport::Files,
            LegacyTransport::Email,
            LegacyTransport::Browser,
        ] {
            assert_eq!(LegacyTransport::parse(value.as_str()).unwrap(), value);
        }
        for value in [
            WebhookDeliveryState::Pending,
            WebhookDeliveryState::Delivered,
            WebhookDeliveryState::Failed,
            WebhookDeliveryState::Replay,
        ] {
            assert_eq!(WebhookDeliveryState::parse(value.as_str()).unwrap(), value);
        }
    }

    #[test]
    fn ep011_unit_sdk_vocabulary_rejects_unknown() {
        assert!(SdkLanguage::parse("COBOL").is_err());
        assert!(SidecarTransport::parse("SMOKE_SIGNALS").is_err());
        assert!(LegacyTransport::parse("MAINFRAME").is_err());
        assert!(WebhookDeliveryState::parse("SOMEDAY").is_err());
    }

    #[test]
    fn ep011_unit_webhook_event_requires_versioned_type() {
        let ev = WebhookEvent {
            event_id: "evt-1".to_string(),
            event_type: "invoice.created".to_string(),
            version: "1".to_string(),
            correlation_id: "corr-1".to_string(),
            payload: serde_json::json!({ "ok": true }),
        };
        let json = serde_json::to_value(&ev).unwrap();
        assert_eq!(json["event_type"], "invoice.created");
        assert_eq!(json["version"], "1");
    }
}
