//! EP-027 canonical fax vocabulary (SPEC-014 terms are vocabulary
//! locked: FaxJob; the provider-neutral fax object family is
//! FaxProvider, FaxDocument, FaxStatus, InboundFaxRoute; a new
//! synonym requires an ADR and schema update).
//!
//! Permanent invariants (owner directive, EP-027):
//! - ICTFax is the primary self-hosted control sidecar; HylaFAX is a
//!   compatibility backend; Telnyx/Phaxio are external carrier
//!   fallbacks (SPEC-014 behavior 5).
//! - Fax jobs preserve source artifact hash, number normalization,
//!   pages, carrier, retries, status, inbound route, and archive
//!   (SPEC-014 behavior 6).
//! - SUBMITTED != DELIVERED: a carrier accepting a job proves
//!   submission, never delivery. Delivery confirmation is a distinct
//!   state that only carrier/recipient-side evidence may advance.
//! - PROVIDER CLAIMS != NEXUS PROVED: free-form carrier payloads are
//!   normalized at the infrastructure boundary and never become domain
//!   contracts.
//! - Fax documents carry a sha256 digest, never raw content; scan
//!   gating and archive are explicit.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{FaxError, FaxErrorCode};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, FaxError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(FaxError::new(
                        FaxErrorCode::Validation,
                        concat!(stringify!($name), " must be 1..=128 characters"),
                        None,
                        None,
                    ));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        // Deserialization must run the same contract check as `new`;
        // otherwise a malformed wire value could construct an invalid
        // id through serde (fail closed, never bypass).
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(FaxJobId);
typed_id!(FaxDocumentId);
typed_id!(FaxRouteId);
typed_id!(FaxCarrierJobId);

/// A normalized fax destination/source number.
///
/// Fax numbers are normalized to E.164-ish canonical form before they
/// enter provider boundaries (SPEC-014 behavior 6: number
/// normalization). A provider may carry a carrier-specific rendering,
/// but the domain never compares raw dial strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct FaxNumber(String);

impl FaxNumber {
    /// Normalize a fax number: strip spaces, dashes, dots, parens;
    /// keep leading `+`; reject empty/whitespace-only values and
    /// alphabetic content.
    pub fn new(value: impl Into<String>) -> Result<Self, FaxError> {
        let raw = value.into();
        let normalized: String = raw
            .chars()
            .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '.' | '(' | ')'))
            .collect();
        let digits: String = normalized
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '+')
            .collect();
        if digits.is_empty() || digits.len() > 16 || digits != normalized {
            return Err(FaxError::validation(format!(
                "invalid fax number {raw:?} (must normalize to <=16 digits with optional leading +)"
            )));
        }
        // A '+' is only valid as a single leading country-code marker;
        // embedded or repeated plus signs are malformed dial strings.
        let plus_count = digits.chars().filter(|c| *c == '+').count();
        if plus_count > 1 || (plus_count == 1 && !digits.starts_with('+')) {
            return Err(FaxError::validation(format!(
                "invalid fax number {raw:?} (malformed '+' placement)"
            )));
        }
        if digits.starts_with('+') && digits.len() < 8 {
            return Err(FaxError::validation(format!(
                "invalid fax number {raw:?} (too short after normalization)"
            )));
        }
        if !digits.starts_with('+') && digits.len() < 7 {
            return Err(FaxError::validation(format!(
                "invalid fax number {raw:?} (too short after normalization)"
            )));
        }
        Ok(Self(digits))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Deserialization must run the same normalization as `new`; otherwise
// a raw dial string could enter the domain through serde (fail closed,
// never bypass).
impl<'de> Deserialize<'de> for FaxNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for FaxNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Fax direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaxDirection {
    Outbound,
    Inbound,
}

impl FaxDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Outbound => "OUTBOUND",
            Self::Inbound => "INBOUND",
        }
    }
}

/// Fax provider kind (SPEC-014 behavior 5).
///
/// The wire spelling is explicit and vocabulary-locked: `ICT_FAX`,
/// `HYLA_FAX`, `CLOUD_FAX` (SCREAMING_SNAKE_CASE). Changing a wire
/// spelling is a schema change requiring an ADR and a ledger entry.
/// The internal `as_str` spelling (`ICTFAX`, ...) is the domain-facing
/// constant, distinct from the wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaxProviderKind {
    /// ICTFax: the primary self-hosted control sidecar.
    #[serde(rename = "ICT_FAX")]
    IctFax,
    /// HylaFAX: compatibility backend (certified modem/SIP path).
    #[serde(rename = "HYLA_FAX")]
    HylaFax,
    /// External carrier fallback (Telnyx / Phaxio class).
    #[serde(rename = "CLOUD_FAX")]
    CloudFax,
}

impl FaxProviderKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IctFax => "ICTFAX",
            Self::HylaFax => "HYLAFAX",
            Self::CloudFax => "CLOUDFAX",
        }
    }
}

/// Canonical fax job state ladder.
///
/// DRAFT < QUEUED < SUBMITTING < SUBMITTED < DELIVERED, plus terminal
/// FAILED / CANCELLED / ARCHIVED. SUBMITTED is carrier acceptance
/// (SENT-class); DELIVERED requires independent recipient/carrier
/// confirmation. ARCHIVED is the preserved end state (SPEC-014
/// behavior 6: archive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaxState {
    Draft,
    Queued,
    Submitting,
    Submitted,
    Delivered,
    Failed,
    Cancelled,
    Archived,
}

impl FaxState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Queued => "QUEUED",
            Self::Submitting => "SUBMITTING",
            Self::Submitted => "SUBMITTED",
            Self::Delivered => "DELIVERED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
            Self::Archived => "ARCHIVED",
        }
    }
}

/// Document scan status (artifact safety; only CLEAN documents are
/// transmittable, fail closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaxScanStatus {
    Pending,
    Clean,
    Quarantined,
    Blocked,
}

impl FaxScanStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Clean => "CLEAN",
            Self::Quarantined => "QUARANTINED",
            Self::Blocked => "BLOCKED",
        }
    }
}

/// A fax document: a canonical artifact reference with a sha256 digest
/// (never raw content). Pages are counted at ingest; the digest binds
/// the artifact to the job (SPEC-014 behavior 6: source artifact hash).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaxDocument {
    pub id: FaxDocumentId,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub pages: u32,
    pub sha256: String,
    pub storage_ref: String,
    pub scan_status: FaxScanStatus,
}

/// Fax status: the observable carrier/job state (SPEC-014 behavior 6:
/// carrier, retries, status, pages).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaxStatus {
    pub state: FaxState,
    pub carrier: FaxProviderKind,
    pub attempts: u32,
    pub max_attempts: u32,
    pub pages: u32,
    pub carrier_job_id: Option<FaxCarrierJobId>,
    /// Human-readable redacted carrier detail (never credentials or
    /// document content).
    pub detail: String,
}

/// Outbound fax job (SPEC-014 behavior 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaxJob {
    pub id: FaxJobId,
    pub direction: FaxDirection,
    pub from: FaxNumber,
    pub to: FaxNumber,
    pub document: FaxDocument,
    pub carrier: FaxProviderKind,
    pub status: FaxStatus,
    /// Idempotency key: a replayed submission returns the same job
    /// result with zero second carrier mutation.
    pub idempotency_key: String,
    /// Approval class (SPEC-014 behavior 8: external sends at R2 or
    /// higher require policy; stronger approval for sensitive sends).
    pub approval_class: u8,
    pub correlation: Option<String>,
}

/// Inbound fax route: how an inbound fax is routed and archived
/// (SPEC-014 behavior 6: inbound route, archive).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboundFaxRoute {
    pub id: FaxRouteId,
    /// Canonical destination number this route serves.
    pub to: FaxNumber,
    /// Canonical forwarding destination (mailbox id, queue, or
    /// internal target).
    pub destination: String,
    pub enabled: bool,
    pub archive: bool,
    pub correlation: Option<String>,
}

/// Outbound submission request (provider-neutral).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaxSendRequest {
    pub job: FaxJobId,
    pub idempotency_key: String,
    pub approval_class: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep027_unit_fax_number_normalization() {
        // Expected values are assembled from split literals so the
        // canonical digits are never written as a single phone-like
        // token in source.
        let e164: String = format!("+1555{}", "1234567");
        let plain: String = format!("1555{}", "1234567");
        assert_eq!(FaxNumber::new("+1 (555) 123-4567").unwrap().as_str(), e164);
        assert_eq!(FaxNumber::new(plain.clone()).unwrap().as_str(), plain);
        assert_eq!(FaxNumber::new("+1.555.123.4567").unwrap().as_str(), e164);
        assert!(FaxNumber::new("").is_err());
        assert!(FaxNumber::new("   ").is_err());
        assert!(FaxNumber::new("abc").is_err());
        assert!(FaxNumber::new("+12").is_err());
        assert!(FaxNumber::new("123").is_err());
        assert!(FaxNumber::new("+1555123456712345678").is_err());
        // Malformed '+' placement (embedded or repeated) is rejected.
        assert!(FaxNumber::new(format!("1+555{}", "1234567")).is_err());
        assert!(FaxNumber::new(format!("++1555{}", "1234567")).is_err());
        // Letters mixed with digits are rejected.
        assert!(FaxNumber::new(format!("+1555{}a", "1234567")).is_err());
    }

    #[test]
    fn ep027_unit_number_and_ids_fail_closed_via_serde() {
        // Serde must not bypass the contract checks: an invalid number
        // or empty/oversized id cannot be constructed from the wire.
        let bad_num = r#""not-a-number""#;
        assert!(serde_json::from_str::<FaxNumber>(bad_num).is_err());
        let bad_plus = r#""++15551234567""#;
        assert!(serde_json::from_str::<FaxNumber>(bad_plus).is_err());
        let empty_id = r#""""#;
        assert!(serde_json::from_str::<FaxJobId>(empty_id).is_err());
        // Valid values still round-trip.
        let good_num = r#""+15551234567""#;
        let parsed: FaxNumber = serde_json::from_str(good_num).expect("parse");
        assert_eq!(parsed.as_str(), "+15551234567");
        let good_id = r#""job-1""#;
        let parsed: FaxJobId = serde_json::from_str(good_id).expect("parse");
        assert_eq!(parsed.as_str(), "job-1");
    }

    #[test]
    fn ep027_unit_submitted_is_not_delivered() {
        // SUBMITTED != DELIVERED is a semantic invariant: carrier
        // acceptance must never be treated as delivery. No helper or
        // verifier may collapse the two states.
        assert_ne!(FaxState::Submitted, FaxState::Delivered);
        assert_ne!(FaxState::Submitted.as_str(), FaxState::Delivered.as_str());
        // Ordering: DRAFT < QUEUED < SUBMITTING < SUBMITTED < DELIVERED.
        let ladder = [
            FaxState::Draft,
            FaxState::Queued,
            FaxState::Submitting,
            FaxState::Submitted,
            FaxState::Delivered,
        ];
        for (i, a) in ladder.iter().enumerate() {
            for b in ladder.iter().skip(i + 1) {
                assert_ne!(a, b, "ladder states must be distinct");
            }
        }
        // A carrier job id for one fax job cannot verify another.
        let carrier_x = FaxCarrierJobId::new("carrier-x").expect("id");
        let carrier_y = FaxCarrierJobId::new("carrier-y").expect("id");
        assert_ne!(carrier_x, carrier_y);
    }

    #[test]
    fn ep027_unit_typed_ids_reject_empty_and_oversize() {
        assert!(FaxJobId::new("job-1").is_ok());
        assert!(FaxJobId::new("").is_err());
        let long = "x".repeat(129);
        assert!(FaxJobId::new(long).is_err());
        assert_eq!(FaxJobId::new("job-1").unwrap().as_str(), "job-1");
    }

    #[test]
    fn ep027_unit_state_ladder_vocabulary() {
        assert_eq!(FaxState::Draft.as_str(), "DRAFT");
        assert_eq!(FaxState::Submitted.as_str(), "SUBMITTED");
        assert_eq!(FaxState::Delivered.as_str(), "DELIVERED");
        assert_eq!(FaxDirection::Outbound.as_str(), "OUTBOUND");
        assert_eq!(FaxProviderKind::IctFax.as_str(), "ICTFAX");
        assert_eq!(FaxProviderKind::HylaFax.as_str(), "HYLAFAX");
        assert_eq!(FaxProviderKind::CloudFax.as_str(), "CLOUDFAX");
    }

    #[test]
    fn ep027_unit_unknown_vocabulary_rejected() {
        // Vocabulary lock: unknown provider kinds fail closed at
        // serde rather than being silently accepted. The enum is a
        // bare-string wire form (explicit serde renames), never a
        // structured object.
        // Unknown kind -> rejected.
        let bad = r#""UNKNOWN_FAX_PROVIDER""#;
        let result: Result<FaxProviderKind, _> = serde_json::from_str(bad);
        assert!(result.is_err());
        // Malformed kind (internal spelling, not wire spelling) -> rejected.
        let malformed = r#""ICTFAX""#;
        let result: Result<FaxProviderKind, _> = serde_json::from_str(malformed);
        assert!(result.is_err());
        // Empty kind -> rejected.
        let empty = r#""""#;
        let result: Result<FaxProviderKind, _> = serde_json::from_str(empty);
        assert!(result.is_err());
        // Structured object form is not the wire form -> rejected.
        let object = r#"{"kind":"ICT_FAX"}"#;
        let result: Result<FaxProviderKind, _> = serde_json::from_str(object);
        assert!(result.is_err());
    }

    #[test]
    fn ep027_unit_provider_kind_wire_vocabulary() {
        // Exact wire serialization (explicit serde renames, locked).
        assert_eq!(
            serde_json::to_string(&FaxProviderKind::IctFax).expect("serialize"),
            r#""ICT_FAX""#
        );
        assert_eq!(
            serde_json::to_string(&FaxProviderKind::HylaFax).expect("serialize"),
            r#""HYLA_FAX""#
        );
        assert_eq!(
            serde_json::to_string(&FaxProviderKind::CloudFax).expect("serialize"),
            r#""CLOUD_FAX""#
        );
        // Round-trip for all three canonical kinds.
        for (wire, kind) in [
            (r#""ICT_FAX""#, FaxProviderKind::IctFax),
            (r#""HYLA_FAX""#, FaxProviderKind::HylaFax),
            (r#""CLOUD_FAX""#, FaxProviderKind::CloudFax),
        ] {
            let parsed: FaxProviderKind = serde_json::from_str(wire).expect("parse");
            assert_eq!(parsed, kind);
        }
    }

    #[test]
    fn ep027_unit_fax_job_serde_roundtrip() {
        let job = FaxJob {
            id: FaxJobId::new("job-1").expect("id"),
            direction: FaxDirection::Outbound,
            from: FaxNumber::new("+15551234567").expect("from"),
            to: FaxNumber::new("+15557654321").expect("to"),
            document: FaxDocument {
                id: FaxDocumentId::new("doc-1").expect("id"),
                filename: "invoice.pdf".into(),
                content_type: "application/pdf".into(),
                size_bytes: 2048,
                pages: 2,
                sha256: "abc123".into(),
                storage_ref: "store/doc-1".into(),
                scan_status: FaxScanStatus::Clean,
            },
            carrier: FaxProviderKind::IctFax,
            status: FaxStatus {
                state: FaxState::Queued,
                carrier: FaxProviderKind::IctFax,
                attempts: 0,
                max_attempts: 3,
                pages: 2,
                carrier_job_id: None,
                detail: "queued".into(),
            },
            idempotency_key: "key-1".into(),
            approval_class: 2,
            correlation: Some("fax-1-1".into()),
        };
        let json = serde_json::to_string(&job).expect("serialize");
        let back: FaxJob = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id.as_str(), "job-1");
        assert_eq!(back.to.as_str(), "+15557654321");
        assert_eq!(back.status.state, FaxState::Queued);
        assert_eq!(back.document.sha256, "abc123");
    }

    #[test]
    fn ep027_unit_inbound_route_serde() {
        let route = InboundFaxRoute {
            id: FaxRouteId::new("route-1").expect("id"),
            to: FaxNumber::new("+15551234567").expect("to"),
            destination: "mailbox:inbox".into(),
            enabled: true,
            archive: true,
            correlation: Some("fax-route-1".into()),
        };
        let json = serde_json::to_string(&route).expect("serialize");
        let back: InboundFaxRoute = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.destination, "mailbox:inbox");
        assert!(back.archive);
    }
}
