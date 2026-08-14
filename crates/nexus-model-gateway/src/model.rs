//! Model request/response contracts (SPEC-009 canonical terms
//! NexusControlObject, PromptSegment, EffortTier).

use crate::vocabulary::EffortTier;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Ordered prompt segment (SPEC-009 required behavior 4).
///
/// Segments are ordered from immutable constitution through schemas,
/// capability taxonomy, risk policy, examples, stable tenant context,
/// session context, and dynamic request. Volatile IDs and timestamps
/// stay in the tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptSegment {
    Constitution,
    Schemas,
    CapabilityTaxonomy,
    RiskPolicy,
    Examples,
    TenantContext,
    SessionContext,
    DynamicRequest,
}

impl PromptSegment {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Constitution => "CONSTITUTION",
            Self::Schemas => "SCHEMAS",
            Self::CapabilityTaxonomy => "CAPABILITY_TAXONOMY",
            Self::RiskPolicy => "RISK_POLICY",
            Self::Examples => "EXAMPLES",
            Self::TenantContext => "TENANT_CONTEXT",
            Self::SessionContext => "SESSION_CONTEXT",
            Self::DynamicRequest => "DYNAMIC_REQUEST",
        }
    }

    /// Canonical segment order: the immutable head first, the volatile
    /// tail last.
    pub fn order(self) -> u8 {
        match self {
            Self::Constitution => 0,
            Self::Schemas => 1,
            Self::CapabilityTaxonomy => 2,
            Self::RiskPolicy => 3,
            Self::Examples => 4,
            Self::TenantContext => 5,
            Self::SessionContext => 6,
            Self::DynamicRequest => 7,
        }
    }
}

impl std::str::FromStr for PromptSegment {
    type Err = crate::vocabulary::ModelGatewayVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "CONSTITUTION" => Ok(Self::Constitution),
            "SCHEMAS" => Ok(Self::Schemas),
            "CAPABILITY_TAXONOMY" => Ok(Self::CapabilityTaxonomy),
            "RISK_POLICY" => Ok(Self::RiskPolicy),
            "EXAMPLES" => Ok(Self::Examples),
            "TENANT_CONTEXT" => Ok(Self::TenantContext),
            "SESSION_CONTEXT" => Ok(Self::SessionContext),
            "DYNAMIC_REQUEST" => Ok(Self::DynamicRequest),
            other => Err(crate::vocabulary::ModelGatewayVocabularyError::unknown(
                "PromptSegment",
                other,
            )),
        }
    }
}

/// A single ordered prompt segment payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptSegmentPart {
    pub segment: PromptSegment,
    pub content: String,
}

/// Provider-neutral model request (SPEC-009).
///
/// Carries the AUTHENTICATED tenant and principal context, the effort
/// tier, ordered prompt segments, budget reference, and correlation
/// ids. Provider credentials are never part of a request; adapters
/// resolve credentials by reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub request_id: String,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub tenant_id: String,
    pub principal_id: String,
    pub effort_tier: EffortTier,
    pub segments: Vec<PromptSegmentPart>,
    pub budget_ref: Option<String>,
    pub schema_version: String,
}

impl ModelRequest {
    /// Ordered segments: the caller may pass any order; this returns
    /// the canonical SPEC-009 order (immutable head first, volatile
    /// tail last).
    pub fn ordered_segments(&self) -> Vec<&PromptSegmentPart> {
        let mut parts: Vec<&PromptSegmentPart> = self.segments.iter().collect();
        parts.sort_by_key(|p| p.segment.order());
        parts
    }
}

/// The canonical model response envelope (SPEC-009 canonical term
/// NexusControlObject).
///
/// Every provider returns the same schema; deterministic validation
/// rejects extra or invalid fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NexusControlObject {
    pub schema_version: String,
    /// The structured control decision (validated, not free text).
    pub control: Value,
    /// Provider name/fingerprint, never credentials.
    pub provider: String,
    pub model: String,
    pub usage: UsageReport,
}

/// Usage accounting (SPEC-009 budgets and cache discipline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageReport {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cache_hit_prompt_tokens: u64,
}

impl UsageReport {
    pub fn total_tokens(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Model response: canonical control object plus correlation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelResponse {
    pub request_id: String,
    pub correlation_id: String,
    pub control_object: NexusControlObject,
}

/// Tool call envelope (SPEC-009; EP-013 node contract).
///
/// A tool call produced by a model. The envelope is advisory only:
/// execution requires the canonical Nexus authorization path (EP-008).
/// A model can never grant scopes, approve actions, modify policies,
/// reveal secrets, or bypass output validation (SPEC-009 behavior 10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallEnvelope {
    pub tool_name: String,
    pub arguments: Value,
    pub call_id: String,
    pub tenant_id: String,
    pub principal_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep013_unit_prompt_segment_round_trip() {
        for (wire, expected) in [
            ("CONSTITUTION", PromptSegment::Constitution),
            ("SCHEMAS", PromptSegment::Schemas),
            ("CAPABILITY_TAXONOMY", PromptSegment::CapabilityTaxonomy),
            ("RISK_POLICY", PromptSegment::RiskPolicy),
            ("EXAMPLES", PromptSegment::Examples),
            ("TENANT_CONTEXT", PromptSegment::TenantContext),
            ("SESSION_CONTEXT", PromptSegment::SessionContext),
            ("DYNAMIC_REQUEST", PromptSegment::DynamicRequest),
        ] {
            assert_eq!(wire.parse::<PromptSegment>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("MEMORY".parse::<PromptSegment>().is_err());
    }

    #[test]
    fn ep013_unit_prompt_segment_order_is_canonical() {
        assert!(PromptSegment::Constitution.order() < PromptSegment::Schemas.order());
        assert!(PromptSegment::Schemas.order() < PromptSegment::CapabilityTaxonomy.order());
        assert!(PromptSegment::CapabilityTaxonomy.order() < PromptSegment::RiskPolicy.order());
        assert!(PromptSegment::RiskPolicy.order() < PromptSegment::Examples.order());
        assert!(PromptSegment::Examples.order() < PromptSegment::TenantContext.order());
        assert!(PromptSegment::TenantContext.order() < PromptSegment::SessionContext.order());
        assert!(PromptSegment::SessionContext.order() < PromptSegment::DynamicRequest.order());
    }

    #[test]
    fn ep013_unit_request_orders_segments() {
        let req = ModelRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            causation_id: None,
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
            effort_tier: EffortTier::Deterministic,
            segments: vec![
                PromptSegmentPart {
                    segment: PromptSegment::DynamicRequest,
                    content: "now".into(),
                },
                PromptSegmentPart {
                    segment: PromptSegment::Constitution,
                    content: "constitution".into(),
                },
                PromptSegmentPart {
                    segment: PromptSegment::RiskPolicy,
                    content: "policy".into(),
                },
            ],
            budget_ref: None,
            schema_version: "1.0".into(),
        };
        let ordered = req.ordered_segments();
        assert_eq!(ordered[0].segment, PromptSegment::Constitution);
        assert_eq!(ordered[1].segment, PromptSegment::RiskPolicy);
        assert_eq!(ordered[2].segment, PromptSegment::DynamicRequest);
    }

    #[test]
    fn ep013_unit_request_rejects_unknown_effort_tier() {
        // Vocabulary rejection happens at parse time (fail closed).
        assert!(
            serde_json::from_value::<ModelRequest>(serde_json::json!({
                "request_id": "r",
                "correlation_id": "c",
                "causation_id": null,
                "tenant_id": "t",
                "principal_id": "p",
                "effort_tier": "ULTRA",
                "segments": [],
                "budget_ref": null,
                "schema_version": "1.0"
            }))
            .is_err()
        );
    }

    #[test]
    fn ep013_unit_usage_total() {
        let u = UsageReport {
            prompt_tokens: 100,
            completion_tokens: 20,
            cache_hit_prompt_tokens: 90,
        };
        assert_eq!(u.total_tokens(), 120);
    }

    #[test]
    fn ep013_unit_tool_call_envelope_round_trip() {
        let e = ToolCallEnvelope {
            tool_name: "contacts.query".into(),
            arguments: serde_json::json!({"q": "a"}),
            call_id: "call-1".into(),
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
        };
        let v = serde_json::to_value(&e).unwrap();
        let back: ToolCallEnvelope = serde_json::from_value(v).unwrap();
        assert_eq!(back.tool_name, "contacts.query");
        assert_eq!(back.call_id, "call-1");
    }

    #[test]
    fn ep013_unit_control_object_round_trip() {
        let c = NexusControlObject {
            schema_version: "1.0".into(),
            control: serde_json::json!({"action": "query", "target": "contacts"}),
            provider: "deepseek-v4-flash".into(),
            model: "deepseek-v4-flash".into(),
            usage: UsageReport {
                prompt_tokens: 10,
                completion_tokens: 5,
                cache_hit_prompt_tokens: 0,
            },
        };
        let v = serde_json::to_value(&c).unwrap();
        let back: NexusControlObject = serde_json::from_value(v).unwrap();
        assert_eq!(back.provider, "deepseek-v4-flash");
        assert_eq!(back.control["action"], "query");
    }
}
