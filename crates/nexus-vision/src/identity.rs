//! EP-023 visitor identity (SPEC-021 behavior 6).
//!
//! Known-person matching is advisory only: it can never unlock or
//! disarm by itself. The advisory-only constraint is enforced at
//! construction.

use serde::{Deserialize, Serialize};

use crate::error::{VisionError, VisionErrorCode};

/// A known visitor classification. `advisory_only` is always true and
/// enforced at construction: identity is evidence for a human/policy
/// decision, never authority for unlock or disarm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownVisitor {
    pub person_id: String,
    /// Matching confidence in 0.0..=1.0.
    pub confidence: f32,
    /// Always true (SPEC-021 behavior 6).
    pub advisory_only: bool,
}

impl KnownVisitor {
    pub fn new(person_id: impl Into<String>, confidence: f32) -> Result<Self, VisionError> {
        let person_id = person_id.into();
        if person_id.is_empty() || person_id.len() > 128 {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "person id must be 1..=128 characters",
                None,
                None,
            ));
        }
        if !(0.0..=1.0).contains(&confidence) {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "known visitor confidence must be in 0.0..=1.0",
                None,
                None,
            ));
        }
        Ok(Self {
            person_id,
            confidence,
            advisory_only: true,
        })
    }
}

/// Visitor identity classification (canonical terms KnownPerson /
/// UnknownPerson).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VisitorIdentity {
    Known(KnownVisitor),
    Unknown,
}

impl VisitorIdentity {
    /// Identity evidence is advisory: it never authorizes unlock or
    /// disarm by itself (SPEC-021 behavior 6).
    pub const fn is_advisory_only(&self) -> bool {
        true
    }
}
