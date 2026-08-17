//! EP-023 camera events (SPEC-021 behavior 5): camera, time, object,
//! zones, confidence, media references, retention, and privacy class.

use serde::{Deserialize, Serialize};

use crate::error::{VisionError, VisionErrorCode};
use crate::stream::StreamRef;
use crate::vocabulary::{CameraId, PrivacyClass};

/// A canonical camera event normalized at the infrastructure boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraEvent {
    pub camera_id: CameraId,
    /// Epoch milliseconds of the event.
    pub timestamp_ms: u64,
    /// Detected object label (free-form from the detector, non-empty).
    pub object: String,
    /// Detector zones the object appeared in (empty when none).
    pub zones: Vec<String>,
    /// Detection confidence in 0.0..=1.0.
    pub confidence: f32,
    /// Media references (clips/snapshots); raw media is never in the
    /// event payload itself.
    pub media_refs: Vec<StreamRef>,
    /// Retention in days for the event media.
    pub retention_days: u32,
    /// Privacy class (SPEC-021 behavior 5).
    pub privacy_class: PrivacyClass,
}

impl CameraEvent {
    pub fn new(
        camera_id: CameraId,
        timestamp_ms: u64,
        object: impl Into<String>,
        confidence: f32,
        retention_days: u32,
        privacy_class: PrivacyClass,
    ) -> Result<Self, VisionError> {
        let object = object.into();
        if object.is_empty() {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "camera event object must not be empty",
                None,
                None,
            ));
        }
        if !(0.0..=1.0).contains(&confidence) {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "camera event confidence must be in 0.0..=1.0",
                None,
                None,
            ));
        }
        Ok(Self {
            camera_id,
            timestamp_ms,
            object,
            zones: Vec::new(),
            confidence,
            media_refs: Vec::new(),
            retention_days,
            privacy_class,
        })
    }

    pub fn with_zone(mut self, zone: impl Into<String>) -> Self {
        self.zones.push(zone.into());
        self
    }

    pub fn with_media(mut self, media: StreamRef) -> Self {
        self.media_refs.push(media);
        self
    }
}

/// A review item (recorded clip/snapshot reference).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewItem {
    pub review_id: String,
    pub camera_id: CameraId,
    pub timestamp_ms: u64,
    pub media_refs: Vec<StreamRef>,
    pub retention_days: u32,
    pub privacy_class: PrivacyClass,
}

impl ReviewItem {
    pub fn new(
        review_id: impl Into<String>,
        camera_id: CameraId,
        timestamp_ms: u64,
        retention_days: u32,
        privacy_class: PrivacyClass,
    ) -> Result<Self, VisionError> {
        let review_id = review_id.into();
        if review_id.is_empty() || review_id.len() > 128 {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "review id must be 1..=128 characters",
                None,
                None,
            ));
        }
        Ok(Self {
            review_id,
            camera_id,
            timestamp_ms,
            media_refs: Vec::new(),
            retention_days,
            privacy_class,
        })
    }

    pub fn with_media(mut self, media: StreamRef) -> Self {
        self.media_refs.push(media);
        self
    }
}

/// A visitor event produced from a person camera event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisitorEvent {
    pub event_id: String,
    pub camera_id: CameraId,
    pub timestamp_ms: u64,
    /// Identity classification is ADVISORY and can never unlock or
    /// disarm by itself (SPEC-021 behavior 6).
    pub known_person: bool,
    pub confidence: f32,
    pub media_refs: Vec<StreamRef>,
    pub privacy_class: PrivacyClass,
}

impl VisitorEvent {
    pub fn new(
        event_id: impl Into<String>,
        camera_id: CameraId,
        timestamp_ms: u64,
        known_person: bool,
        confidence: f32,
        privacy_class: PrivacyClass,
    ) -> Result<Self, VisionError> {
        let event_id = event_id.into();
        if event_id.is_empty() || event_id.len() > 128 {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "visitor event id must be 1..=128 characters",
                None,
                None,
            ));
        }
        if !(0.0..=1.0).contains(&confidence) {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "visitor event confidence must be in 0.0..=1.0",
                None,
                None,
            ));
        }
        Ok(Self {
            event_id,
            camera_id,
            timestamp_ms,
            known_person,
            confidence,
            media_refs: Vec::new(),
            privacy_class,
        })
    }

    pub fn with_media(mut self, media: StreamRef) -> Self {
        self.media_refs.push(media);
        self
    }
}
