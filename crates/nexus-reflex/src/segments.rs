//! Canonical, versioned prompt segments (SPEC-009 required behavior 4;
//! ADR-021).
//!
//! Prompt segments are ordered from immutable constitution through
//! schemas, capability taxonomy, risk policy, examples, stable tenant
//! context, session context, and dynamic request. Canonical
//! serialization fixes key ordering, whitespace, schema ordering, tool
//! ordering, and segment versions. Volatile IDs and timestamps stay in
//! the tail.

use crate::error::ReflexError;
use nexus_model_gateway::model::{PromptSegment, PromptSegmentPart};
use serde::{Deserialize, Serialize};

/// A versioned prompt segment content payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptSegmentVersion {
    /// Canonical segment kind (CONSTITUTION, SCHEMAS, ...).
    pub segment: PromptSegment,
    /// Content version for this segment (e.g. "1.0").
    pub version: String,
    /// The stable content. Never contains volatile ids or timestamps.
    pub content: String,
}

/// The stable prefix: the immutable head of the prompt (constitution
/// through stable tenant context). This is the cacheable prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StablePrefix {
    pub segments: Vec<PromptSegmentVersion>,
}

impl StablePrefix {
    pub fn new(segments: Vec<PromptSegmentVersion>) -> Result<Self, ReflexError> {
        let mut sorted = segments;
        sorted.sort_by_key(|s| s.segment.order());
        // Canonical order: constitution (0) ... tenant context (5).
        let orders: Vec<u8> = sorted.iter().map(|s| s.segment.order()).collect();
        let expected: Vec<u8> = (0..6).collect();
        if orders != expected {
            return Err(ReflexError::validation(
                "stable prefix must contain exactly constitution..tenant-context in canonical order",
                Some("prompt-segments".into()),
            ));
        }
        Ok(Self { segments: sorted })
    }

    /// Canonical serialization: fixed segment order, no volatile
    /// fields, version-tagged. Produces a byte-stable string.
    pub fn canonical(&self) -> String {
        let mut out = String::new();
        for s in &self.segments {
            out.push_str(s.segment.as_str());
            out.push(':');
            out.push_str(&s.version);
            out.push(':');
            out.push_str(&s.content);
            out.push('\n');
        }
        out
    }

    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

/// A full canonical prompt catalog: stable prefix plus volatile tail.
///
/// The volatile tail (session context, dynamic request) is NOT part of
/// the cacheable prefix. Volatile ids and timestamps live here only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptSegmentCatalog {
    pub prefix: StablePrefix,
    pub session_context: Option<PromptSegmentVersion>,
    pub dynamic_request: Option<PromptSegmentVersion>,
}

impl PromptSegmentCatalog {
    pub fn new(
        prefix: StablePrefix,
        session_context: Option<PromptSegmentVersion>,
        dynamic_request: Option<PromptSegmentVersion>,
    ) -> Self {
        Self {
            prefix,
            session_context,
            dynamic_request,
        }
    }

    /// Ordered full catalog: prefix, then session, then dynamic.
    pub fn ordered(&self) -> Vec<PromptSegmentVersion> {
        let mut out = self.prefix.segments.clone();
        if let Some(s) = &self.session_context {
            out.push(s.clone());
        }
        if let Some(d) = &self.dynamic_request {
            out.push(d.clone());
        }
        out
    }

    /// Canonical serialization of the full catalog (byte-stable).
    pub fn canonical(&self) -> String {
        let mut out = self.prefix.canonical();
        if let Some(s) = &self.session_context {
            out.push_str(s.segment.as_str());
            out.push(':');
            out.push_str(&s.version);
            out.push(':');
            out.push_str(&s.content);
            out.push('\n');
        }
        if let Some(d) = &self.dynamic_request {
            out.push_str(d.segment.as_str());
            out.push(':');
            out.push_str(&d.version);
            out.push(':');
            out.push_str(&d.content);
            out.push('\n');
        }
        out
    }
}

/// Convert catalog segments to transport prompt parts in canonical
/// order.
pub fn to_prompt_parts(catalog: &PromptSegmentCatalog) -> Vec<PromptSegmentPart> {
    catalog
        .ordered()
        .into_iter()
        .map(|s| PromptSegmentPart {
            segment: s.segment,
            content: s.content,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stable() -> StablePrefix {
        StablePrefix::new(vec![
            PromptSegmentVersion {
                segment: PromptSegment::RiskPolicy,
                version: "1.0".into(),
                content: "risk policy".into(),
            },
            PromptSegmentVersion {
                segment: PromptSegment::Constitution,
                version: "1.0".into(),
                content: "constitution".into(),
            },
            PromptSegmentVersion {
                segment: PromptSegment::Schemas,
                version: "1.0".into(),
                content: "schemas".into(),
            },
            PromptSegmentVersion {
                segment: PromptSegment::CapabilityTaxonomy,
                version: "1.0".into(),
                content: "capabilities".into(),
            },
            PromptSegmentVersion {
                segment: PromptSegment::Examples,
                version: "1.0".into(),
                content: "examples".into(),
            },
            PromptSegmentVersion {
                segment: PromptSegment::TenantContext,
                version: "1.0".into(),
                content: "tenant".into(),
            },
        ])
        .unwrap()
    }

    #[test]
    fn ep014_unit_stable_prefix_orders_canonically() {
        let prefix = stable();
        assert_eq!(prefix.len(), 6);
        assert_eq!(prefix.segments[0].segment, PromptSegment::Constitution);
        assert_eq!(prefix.segments[1].segment, PromptSegment::Schemas);
        assert_eq!(prefix.segments[5].segment, PromptSegment::TenantContext);
    }

    #[test]
    fn ep014_unit_stable_prefix_rejects_wrong_segments() {
        // Missing constitution..tenant-context coverage must fail.
        let bad = vec![PromptSegmentVersion {
            segment: PromptSegment::Constitution,
            version: "1.0".into(),
            content: "constitution".into(),
        }];
        assert!(StablePrefix::new(bad).is_err());
    }

    #[test]
    fn ep014_unit_canonical_serialization_is_byte_stable() {
        let a = PromptSegmentCatalog::new(stable(), None, None);
        let b = PromptSegmentCatalog::new(stable(), None, None);
        assert_eq!(a.canonical(), b.canonical());
        assert_eq!(a.canonical().len(), b.canonical().len());
        // Constitution must serialize first.
        assert!(a.canonical().starts_with("CONSTITUTION:"));
    }

    #[test]
    fn ep014_unit_volatile_tail_is_not_in_prefix() {
        let catalog = PromptSegmentCatalog::new(
            stable(),
            Some(PromptSegmentVersion {
                segment: PromptSegment::SessionContext,
                version: "1.0".into(),
                content: "session".into(),
            }),
            Some(PromptSegmentVersion {
                segment: PromptSegment::DynamicRequest,
                version: "1.0".into(),
                content: "request".into(),
            }),
        );
        let canonical = catalog.canonical();
        // Session and dynamic serialized AFTER the stable prefix.
        assert!(canonical.ends_with("DYNAMIC_REQUEST:1.0:request\n"));
        assert!(canonical.contains("SESSION_CONTEXT:1.0:session\n"));
        // The stable prefix itself never contains the tail.
        let prefix = catalog.prefix.canonical();
        assert!(!prefix.contains("SESSION_CONTEXT"));
        assert!(!prefix.contains("DYNAMIC_REQUEST"));
    }

    #[test]
    fn ep014_unit_catalog_serde_round_trip() {
        let catalog = PromptSegmentCatalog::new(stable(), None, None);
        let v = serde_json::to_value(&catalog).unwrap();
        let back: PromptSegmentCatalog = serde_json::from_value(v).unwrap();
        assert_eq!(back, catalog);
    }

    #[test]
    fn ep014_unit_to_prompt_parts_preserves_order() {
        let catalog = PromptSegmentCatalog::new(stable(), None, None);
        let parts = to_prompt_parts(&catalog);
        assert_eq!(parts.len(), 6);
        assert_eq!(parts[0].segment, PromptSegment::Constitution);
        assert_eq!(parts[5].segment, PromptSegment::TenantContext);
    }
}
