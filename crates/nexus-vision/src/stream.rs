//! EP-023 stream references (SPEC-021 behavior 1, acceptance
//! obligation 3).
//!
//! No unverified RTSP or ONVIF claim is made: a StreamRef is
//! Unverified unless real verification evidence exists (go2rtc
//! normalization proof or equivalent).

use serde::{Deserialize, Serialize};

use crate::error::{VisionError, VisionErrorCode};

/// Verification status of a stream reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    /// Real verification evidence exists (go2rtc normalization proof).
    VerifiedLocal,
    /// No verification evidence yet; claims are forbidden.
    Unverified,
}

impl VerificationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerifiedLocal => "VERIFIED_LOCAL",
            Self::Unverified => "UNVERIFIED",
        }
    }
}

/// A camera stream reference. Verified streams carry an evidence
/// reference; unverified streams can never be advertised as
/// operational (Reality rule, SPEC-021 behavior 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamRef {
    pub stream_id: String,
    pub url: String,
    pub status: VerificationStatus,
    /// Evidence reference required for VerifiedLocal (e.g. the go2rtc
    /// probe id). Empty for unverified streams.
    pub evidence_ref: Option<String>,
}

impl StreamRef {
    /// Create an unverified stream reference. The URL scheme must be a
    /// supported camera scheme (rtsp, http, https); no operational
    /// claim is made.
    pub fn new_unverified(
        stream_id: impl Into<String>,
        url: impl Into<String>,
    ) -> Result<Self, VisionError> {
        let stream_id = stream_id.into();
        let url = url.into();
        if stream_id.is_empty() || stream_id.len() > 128 {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "stream id must be 1..=128 characters",
                None,
                None,
            ));
        }
        let supported = ["rtsp://", "http://", "https://"];
        if !supported.iter().any(|prefix| url.starts_with(prefix)) {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "stream url must use rtsp, http, or https scheme",
                None,
                None,
            ));
        }
        Ok(Self {
            stream_id,
            url,
            status: VerificationStatus::Unverified,
            evidence_ref: None,
        })
    }

    /// Mark a stream verified only with real evidence. Refuses to
    /// fabricate verification (acceptance obligation 3).
    pub fn verified(mut self, evidence_ref: impl Into<String>) -> Result<Self, VisionError> {
        let evidence_ref = evidence_ref.into();
        if evidence_ref.is_empty() {
            return Err(VisionError::new(
                VisionErrorCode::Verification,
                "verified stream requires a real evidence reference",
                None,
                None,
            ));
        }
        self.status = VerificationStatus::VerifiedLocal;
        self.evidence_ref = Some(evidence_ref);
        Ok(self)
    }
}
