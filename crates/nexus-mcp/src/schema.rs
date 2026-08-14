//! Deterministic JSON-Schema-subset validator (SPEC-003: declared
//! output schemas).
//!
//! The engine validates structured tool output against the tool's
//! declared output schema. This is a strict subset of JSON Schema
//! 2020-12 (type, properties presence, items type, required) - enough
//! for deterministic contract checking without pulling a schema
//! validator dependency into the MCP crate.

use serde_json::Value;

/// Result of a schema check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaCheck {
    Ok,
    Mismatch(String),
}

/// Minimal structural schema validator.
#[derive(Debug, Clone, Default)]
pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate `value` against a declared JSON-Schema-subset.
    ///
    /// Supports: `type` (object/array/string/number/boolean/integer),
    /// `properties` (each present property validated recursively),
    /// `required` (presence), and `items` (element type for arrays).
    /// Unknown keywords are ignored (permissive on extensions, strict
    /// on the checked subset).
    pub fn validate(schema: &Value, value: &Value) -> SchemaCheck {
        let Some(expected_type) = schema.get("type").and_then(Value::as_str) else {
            return SchemaCheck::Ok;
        };
        match expected_type {
            "object" => {
                if !value.is_object() {
                    return SchemaCheck::Mismatch(format!(
                        "expected object, got {}",
                        type_name(value)
                    ));
                }
                if let Some(required) = schema.get("required").and_then(Value::as_array) {
                    let obj = value.as_object().expect("checked above");
                    for key in required {
                        if let Some(name) = key.as_str()
                            && !obj.contains_key(name)
                        {
                            return SchemaCheck::Mismatch(format!(
                                "missing required property: {name}"
                            ));
                        }
                    }
                }
                if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                    let obj = value.as_object().expect("checked above");
                    for (name, prop_schema) in properties {
                        if let Some(prop_value) = obj.get(name)
                            && let SchemaCheck::Mismatch(msg) =
                                Self::validate(prop_schema, prop_value)
                        {
                            return SchemaCheck::Mismatch(format!("{name}: {msg}"));
                        }
                    }
                }
                SchemaCheck::Ok
            }
            "array" => {
                if !value.is_array() {
                    return SchemaCheck::Mismatch(format!(
                        "expected array, got {}",
                        type_name(value)
                    ));
                }
                if let Some(items) = schema.get("items") {
                    let arr = value.as_array().expect("checked above");
                    for (idx, item) in arr.iter().enumerate() {
                        if let SchemaCheck::Mismatch(msg) = Self::validate(items, item) {
                            return SchemaCheck::Mismatch(format!("[{idx}]: {msg}"));
                        }
                    }
                }
                SchemaCheck::Ok
            }
            "string" => {
                if !value.is_string() {
                    return SchemaCheck::Mismatch(format!(
                        "expected string, got {}",
                        type_name(value)
                    ));
                }
                SchemaCheck::Ok
            }
            "number" => {
                if !value.is_number() {
                    return SchemaCheck::Mismatch(format!(
                        "expected number, got {}",
                        type_name(value)
                    ));
                }
                SchemaCheck::Ok
            }
            "integer" => {
                if !value.is_i64() && !value.is_u64() {
                    return SchemaCheck::Mismatch(format!(
                        "expected integer, got {}",
                        type_name(value)
                    ));
                }
                SchemaCheck::Ok
            }
            "boolean" => {
                if !value.is_boolean() {
                    return SchemaCheck::Mismatch(format!(
                        "expected boolean, got {}",
                        type_name(value)
                    ));
                }
                SchemaCheck::Ok
            }
            other => SchemaCheck::Mismatch(format!("unsupported schema type: {other}")),
        }
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_mcp_schema_validates_object_output() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": {"type": "string"},
                "count": {"type": "integer"}
            }
        });
        assert_eq!(
            SchemaValidator::validate(&schema, &serde_json::json!({"id": "c1", "count": 2})),
            SchemaCheck::Ok
        );
        assert!(matches!(
            SchemaValidator::validate(&schema, &serde_json::json!({"count": 2})),
            SchemaCheck::Mismatch(_)
        ));
        assert!(matches!(
            SchemaValidator::validate(&schema, &serde_json::json!({"id": 7})),
            SchemaCheck::Mismatch(_)
        ));
    }

    #[test]
    fn ep012_unit_mcp_schema_validates_array_output() {
        let schema = serde_json::json!({"type": "array", "items": {"type": "string"}});
        assert_eq!(
            SchemaValidator::validate(&schema, &serde_json::json!(["a", "b"])),
            SchemaCheck::Ok
        );
        assert!(matches!(
            SchemaValidator::validate(&schema, &serde_json::json!([1])),
            SchemaCheck::Mismatch(_)
        ));
    }

    #[test]
    fn ep012_unit_mcp_schema_rejects_wrong_root_type() {
        let schema = serde_json::json!({"type": "object"});
        assert!(matches!(
            SchemaValidator::validate(&schema, &serde_json::json!("nope")),
            SchemaCheck::Mismatch(_)
        ));
    }
}
