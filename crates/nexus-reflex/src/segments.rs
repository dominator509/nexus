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

    /// Load the canonical catalog from a directory of versioned segment
    /// JSON files (config/prompts/reflex/). Reads `catalog.json` for the
    /// declared order, then each segment file, validating that the
    /// stable prefix covers exactly constitution..tenant-context and
    /// that every segment is versioned and canonical.
    pub fn from_canonical_dir(dir: &std::path::Path) -> Result<Self, ReflexError> {
        use std::fs;

        let catalog_path = dir.join("catalog.json");
        let catalog_value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&catalog_path).map_err(|e| {
                ReflexError::validation(
                    format!("cannot read catalog.json: {e}"),
                    Some("prompt-segments".into()),
                )
            })?)
            .map_err(|e| {
                ReflexError::validation(
                    format!("catalog.json invalid JSON: {e}"),
                    Some("prompt-segments".into()),
                )
            })?;

        let stable_names: Vec<String> = catalog_value
            .get("stable_prefix")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                ReflexError::validation(
                    "catalog.json missing stable_prefix array",
                    Some("prompt-segments".into()),
                )
            })?
            .iter()
            .map(|v| {
                v.as_str().map(str::to_string).ok_or_else(|| {
                    ReflexError::validation(
                        "catalog.json stable_prefix entries must be strings",
                        Some("prompt-segments".into()),
                    )
                })
            })
            .collect::<Result<_, _>>()?;

        let volatile_names: Vec<String> = catalog_value
            .get("volatile_tail")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                ReflexError::validation(
                    "catalog.json missing volatile_tail array",
                    Some("prompt-segments".into()),
                )
            })?
            .iter()
            .map(|v| {
                v.as_str().map(str::to_string).ok_or_else(|| {
                    ReflexError::validation(
                        "catalog.json volatile_tail entries must be strings",
                        Some("prompt-segments".into()),
                    )
                })
            })
            .collect::<Result<_, _>>()?;

        let mut prefix_segments = Vec::new();
        let mut session_context = None;
        let mut dynamic_request = None;

        for name in stable_names.iter().chain(volatile_names.iter()) {
            let segment: PromptSegment = name.parse().map_err(|_| {
                ReflexError::validation(
                    format!("catalog.json references unknown segment: {name}"),
                    Some("prompt-segments".into()),
                )
            })?;
            let file_name = format!("{}.json", name.to_ascii_lowercase().replace('_', "-"));
            let path = dir.join(&file_name);
            let version: PromptSegmentVersion =
                serde_json::from_str(&fs::read_to_string(&path).map_err(|e| {
                    ReflexError::validation(
                        format!("cannot read segment file {file_name}: {e}"),
                        Some("prompt-segments".into()),
                    )
                })?)
                .map_err(|e| {
                    ReflexError::validation(
                        format!("segment file {file_name} invalid JSON: {e}"),
                        Some("prompt-segments".into()),
                    )
                })?;

            if version.segment != segment {
                return Err(ReflexError::validation(
                    format!(
                        "segment file {file_name} declares {} but catalog expects {}",
                        version.segment.as_str(),
                        segment.as_str()
                    ),
                    Some("prompt-segments".into()),
                ));
            }
            if version.version.is_empty() || version.content.is_empty() {
                return Err(ReflexError::validation(
                    format!("segment file {file_name} must be versioned and non-empty"),
                    Some("prompt-segments".into()),
                ));
            }

            match segment {
                PromptSegment::SessionContext => session_context = Some(version),
                PromptSegment::DynamicRequest => dynamic_request = Some(version),
                _ => prefix_segments.push(version),
            }
        }

        let prefix = StablePrefix::new(prefix_segments)?;
        Ok(Self::new(prefix, session_context, dynamic_request))
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
    fn ep014_unit_stable_prefix_identical_when_tail_changes() {
        // Same logical stable context, different request-specific tails:
        // the stable prefix bytes stay identical; only the tail changes.
        // This is the core cacheability invariant (SPEC-009 behavior 4).
        let catalog_a = PromptSegmentCatalog::new(
            stable(),
            Some(PromptSegmentVersion {
                segment: PromptSegment::SessionContext,
                version: "1.0".into(),
                content: "session-1".into(),
            }),
            Some(PromptSegmentVersion {
                segment: PromptSegment::DynamicRequest,
                version: "1.0".into(),
                content: "request-1".into(),
            }),
        );
        let catalog_b = PromptSegmentCatalog::new(
            stable(),
            Some(PromptSegmentVersion {
                segment: PromptSegment::SessionContext,
                version: "1.0".into(),
                content: "session-2".into(),
            }),
            Some(PromptSegmentVersion {
                segment: PromptSegment::DynamicRequest,
                version: "1.0".into(),
                content: "request-2".into(),
            }),
        );
        // Stable prefix: byte identical across the differing tails.
        assert_eq!(catalog_a.prefix.canonical(), catalog_b.prefix.canonical());
        // Full catalog: the dynamic request portion changed.
        assert_ne!(catalog_a.canonical(), catalog_b.canonical());
        assert!(
            catalog_b
                .canonical()
                .ends_with("DYNAMIC_REQUEST:1.0:request-2\n")
        );
    }

    #[test]
    fn ep014_unit_stable_prefix_fingerprint_changes_on_version_bump() {
        // Intentional invalidation (cache identity): a legitimate
        // stable-prefix input change must change the cacheable prefix
        // bytes, so Nexus never reuses cache identity after a
        // meaningful stable-context change.
        let base = stable();
        // Segment version bump invalidates the prefix fingerprint.
        let mut version_bump = stable();
        version_bump.segments[0].version = "1.1".into();
        assert_ne!(base.canonical(), version_bump.canonical());
        // Stable content change invalidates the prefix fingerprint.
        let mut content_bump = stable();
        content_bump.segments[5].content = "tenant v2".into();
        assert_ne!(base.canonical(), content_bump.canonical());
        // Unchanged inputs remain byte identical (control).
        assert_eq!(base.canonical(), stable().canonical());
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

    // ---- M2: real canonical config directory invariants ----

    fn canonical_dir() -> std::path::PathBuf {
        // crates/nexus-reflex -> repo root -> config/prompts/reflex
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/prompts/reflex")
    }

    #[test]
    fn ep014_unit_canonical_config_loads() {
        let catalog = PromptSegmentCatalog::from_canonical_dir(&canonical_dir()).unwrap();
        assert_eq!(catalog.prefix.len(), 6);
        assert!(catalog.session_context.is_some());
        assert!(catalog.dynamic_request.is_some());
        let parts = to_prompt_parts(&catalog);
        assert_eq!(parts.len(), 8);
        // Canonical order: constitution first, dynamic request last.
        assert_eq!(parts[0].segment, PromptSegment::Constitution);
        assert_eq!(parts[7].segment, PromptSegment::DynamicRequest);
    }

    #[test]
    fn ep014_unit_canonical_config_byte_stable() {
        let a = PromptSegmentCatalog::from_canonical_dir(&canonical_dir()).unwrap();
        let b = PromptSegmentCatalog::from_canonical_dir(&canonical_dir()).unwrap();
        let ca = a.canonical();
        let cb = b.canonical();
        assert_eq!(ca, cb);
        // Re-serialization is deterministic: same bytes every load.
        assert_eq!(ca.len(), cb.len());
        assert!(ca.starts_with("CONSTITUTION:"));
    }

    #[test]
    fn ep014_unit_canonical_config_prefix_is_cacheable_corpus() {
        let catalog = PromptSegmentCatalog::from_canonical_dir(&canonical_dir()).unwrap();
        let prefix = catalog.prefix.canonical();
        // The volatile tail is never part of the cacheable prefix.
        assert!(!prefix.contains("SESSION_CONTEXT"));
        assert!(!prefix.contains("DYNAMIC_REQUEST"));
        // Every prefix segment is versioned.
        assert!(prefix.contains("CONSTITUTION:1.0:"));
        assert!(prefix.contains("TENANT_CONTEXT:1.0:"));
    }

    #[test]
    fn ep014_unit_canonical_config_rejects_missing_segment() {
        let tmp = std::env::temp_dir().join(format!("ep014-bad-catalog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("catalog.json"),
            r#"{"stable_prefix":["CONSTITUTION"],"volatile_tail":[]}"#,
        )
        .unwrap();
        std::fs::write(
            tmp.join("constitution.json"),
            r#"{"segment":"CONSTITUTION","version":"1.0","content":"x"}"#,
        )
        .unwrap();
        // Missing SCHEMAS..TENANT_CONTEXT -> StablePrefix::new fails.
        let err = PromptSegmentCatalog::from_canonical_dir(&tmp).unwrap_err();
        assert_eq!(err.code, crate::error::ReflexErrorCode::Validation);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn ep014_unit_canonical_config_rejects_unversioned_segment() {
        let tmp = std::env::temp_dir().join(format!("ep014-bad-ver-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("catalog.json"),
            r#"{"stable_prefix":["CONSTITUTION","SCHEMAS","CAPABILITY_TAXONOMY","RISK_POLICY","EXAMPLES","TENANT_CONTEXT"],"volatile_tail":[]}"#,
        )
        .unwrap();
        let segments = [
            ("constitution.json", "CONSTITUTION"),
            ("schemas.json", "SCHEMAS"),
            ("capability-taxonomy.json", "CAPABILITY_TAXONOMY"),
            ("risk-policy.json", "RISK_POLICY"),
            ("examples.json", "EXAMPLES"),
            ("tenant-context.json", "TENANT_CONTEXT"),
        ];
        for (i, (file, name)) in segments.iter().enumerate() {
            let version = if *name == "CONSTITUTION" { "" } else { "1.0" };
            std::fs::write(
                tmp.join(file),
                format!(r#"{{"segment":"{name}","version":"{version}","content":"c{i}"}}"#),
            )
            .unwrap();
        }
        let err = PromptSegmentCatalog::from_canonical_dir(&tmp).unwrap_err();
        assert_eq!(err.code, crate::error::ReflexErrorCode::Validation);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
