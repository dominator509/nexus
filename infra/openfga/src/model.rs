//! Minimal Nexus authorization model (EP-008 M3).
//!
//! The model is deliberately small: it proves Nexus relationship
//! semantics (principal, household, business, device, resource,
//! capability/action target) without becoming a generalized policy
//! language. Contextual risk, time, auth strength, and approval are
//! NOT modeled here - they belong to OPA / nexus-policy /
//! action-gateway (responsibility boundary recorded in the Decision
//! Log). OpenFGA proves relationship authorization only.
//!
//! Wire format verified against the pinned OpenFGA 1.18.1 container:
//! `type_definitions` with `relations` and
//! `metadata.relations.*.directly_related_user_types` (the `this:{}`
//! shape alone is rejected with `invalid_authorization_model`).

use serde_json::{Value, json};

/// OpenFGA schema version used by the Nexus model.
pub const NEXUS_MODEL_SCHEMA_VERSION: &str = "1.1";

/// Canonical object types in the Nexus model.
pub mod object_type {
    /// A Nexus principal (HUMAN/SERVICE/AGENT/DEVICE/SYSTEM all map to
    /// the OpenFGA `user` type; the canonical actor type is recorded in
    /// telemetry, not in the relationship model).
    pub const USER: &str = "user";
    /// Household (owner/member/admin).
    pub const HOUSEHOLD: &str = "household";
    /// Business (admin/member).
    pub const BUSINESS: &str = "business";
    /// Device (operator).
    pub const DEVICE: &str = "device";
    /// Resource (viewer/editor/owner).
    pub const RESOURCE: &str = "resource";
    /// Capability / action target (delegated).
    pub const CAPABILITY: &str = "capability";
    /// Action target checked by the deterministic gateway (relation
    /// `actor`); the gateway hardcodes object type `action` for its
    /// relationship stage.
    pub const ACTION: &str = "action";
}

/// Canonical relation names in the Nexus model.
pub mod relation {
    pub const OWNER: &str = "owner";
    pub const MEMBER: &str = "member";
    pub const ADMIN: &str = "admin";
    pub const OPERATOR: &str = "operator";
    pub const VIEWER: &str = "viewer";
    pub const EDITOR: &str = "editor";
    pub const DELEGATED: &str = "delegated";
}

/// The Nexus authorization model as OpenFGA `type_definitions`
/// (verified against the pinned container).
///
/// Semantics:
/// - household.admin is a computed userset of household.owner (an
///   owner administers the household; a member does not).
/// - resource.viewer/editor accept both direct users and the
///   transitive userset `business#admin` (a business admin may manage
///   business resources through the model, not through extra tuples).
/// - explicit deny is not modeled: absence of a relationship is the
///   denial (fail closed). No typed wildcards (`user:*`) exist in this
///   model, so no accidental wildcard can grant.
pub fn nexus_model_type_definitions() -> Value {
    json!([
        { "type": object_type::USER },
        {
            "type": object_type::HOUSEHOLD,
            "relations": {
                "owner": { "this": {} },
                "member": { "this": {} },
                "admin": { "computedUserset": { "relation": relation::OWNER } }
            },
            "metadata": {
                "relations": {
                    "owner": { "directly_related_user_types": [{ "type": object_type::USER }] },
                    "member": { "directly_related_user_types": [{ "type": object_type::USER }] }
                }
            }
        },
        {
            "type": object_type::BUSINESS,
            "relations": {
                "admin": { "this": {} },
                "member": { "this": {} }
            },
            "metadata": {
                "relations": {
                    "admin": { "directly_related_user_types": [{ "type": object_type::USER }] },
                    "member": { "directly_related_user_types": [{ "type": object_type::USER }] }
                }
            }
        },
        {
            "type": object_type::DEVICE,
            "relations": {
                "operator": { "this": {} }
            },
            "metadata": {
                "relations": {
                    "operator": { "directly_related_user_types": [{ "type": object_type::USER }] }
                }
            }
        },
        {
            "type": object_type::RESOURCE,
            "relations": {
                "viewer": { "this": {} },
                "editor": { "this": {} },
                "owner": { "this": {} }
            },
            "metadata": {
                "relations": {
                    "viewer": {
                        "directly_related_user_types": [
                            { "type": object_type::USER },
                            { "type": object_type::BUSINESS, "relation": relation::ADMIN }
                        ]
                    },
                    "editor": {
                        "directly_related_user_types": [
                            { "type": object_type::USER },
                            { "type": object_type::BUSINESS, "relation": relation::ADMIN }
                        ]
                    },
                    "owner": { "directly_related_user_types": [{ "type": object_type::USER }] }
                }
            }
        },
        {
            "type": object_type::CAPABILITY,
            "relations": {
                "delegated": { "this": {} }
            },
            "metadata": {
                "relations": {
                    "delegated": { "directly_related_user_types": [{ "type": object_type::USER }] }
                }
            }
        },
        {
            "type": object_type::ACTION,
            "relations": {
                "actor": { "this": {} }
            },
            "metadata": {
                "relations": {
                    "actor": { "directly_related_user_types": [{ "type": object_type::USER }] }
                }
            }
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep008_unit_nexus_model_has_expected_types() {
        let defs = nexus_model_type_definitions();
        let types: Vec<&str> = defs
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["type"].as_str().unwrap())
            .collect();
        assert_eq!(
            types,
            vec![
                "user",
                "household",
                "business",
                "device",
                "resource",
                "capability",
                "action"
            ]
        );
    }

    #[test]
    fn ep008_unit_nexus_model_household_admin_is_computed_owner() {
        let defs = nexus_model_type_definitions();
        let household = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["type"] == "household")
            .unwrap();
        assert_eq!(
            household["relations"]["admin"]["computedUserset"]["relation"],
            "owner"
        );
    }

    #[test]
    fn ep008_unit_nexus_model_resource_accepts_business_admin_userset() {
        let defs = nexus_model_type_definitions();
        let resource = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["type"] == "resource")
            .unwrap();
        let viewer_types =
            &resource["metadata"]["relations"]["viewer"]["directly_related_user_types"];
        assert!(
            viewer_types
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["type"] == "business" && t["relation"] == "admin")
        );
    }

    #[test]
    fn ep008_unit_nexus_model_has_no_wildcards() {
        let defs = nexus_model_type_definitions();
        let text = serde_json::to_string(&defs).unwrap();
        assert!(
            !text.contains(":*"),
            "model must not contain typed wildcards"
        );
    }
}
