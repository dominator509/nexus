//! EP-024 appliance mapping: real Home Assistant entities onto the
//! provider-neutral appliance model (SPEC-011; M3).
//!
//! Stable identity: the canonical `ApplianceDeviceId` is derived
//! deterministically from the provider entity id (the stable HA
//! identity), never from enumeration index, display name, or ordering.
//! This follows the EP-020 stable provider-identity principle
//! (`nexus-home-assistant::stable_device_id`): the same entity always
//! maps to the same canonical id across repeated discovery, provider
//! restart, and entity ordering changes.
//!
//! Capability mapping: capabilities are derived from REAL entity
//! features observed through the provider boundary (controllable
//! on/off surface, variable mode attribute). A capability is never
//! advertised merely because a device category usually has it; provider
//! discovery determines what is actually available.

use std::collections::BTreeMap;

use serde_json::Value;

use nexus_devices::mapper::DeviceCapabilityMapper;
use nexus_devices::vocabulary::ApplianceCapability;

use crate::error::{ApplianceError, ApplianceErrorCode};
use crate::transport::ApplianceEntity;

/// Canonical appliance selector.
///
/// Explicit entity allowlist: the appliance surface only maps the
/// entities this Nexus instance is configured to control. Unknown
/// entities are never invented into appliances, and internal HA
/// entities (sun, zone, person, ...) can never become appliances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplianceSelector {
    /// Exact provider entity ids (e.g. `fan.nexus_app_fan`).
    entity_ids: Vec<String>,
}

impl ApplianceSelector {
    /// Select exactly the given provider entity ids.
    pub fn entities<I, S>(entity_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut entity_ids: Vec<String> = entity_ids.into_iter().map(Into::into).collect();
        entity_ids.sort();
        entity_ids.dedup();
        Self { entity_ids }
    }

    /// True when the entity is selected as an appliance.
    pub fn contains(&self, entity_id: &str) -> bool {
        self.entity_ids.iter().any(|id| id == entity_id)
    }

    /// The configured entity ids (stable, sorted).
    pub fn configured(&self) -> &[String] {
        &self.entity_ids
    }
}

/// Deterministic opaque canonical appliance id derived from the stable
/// provider entity id (EP-020 stable-identity principle: FNV-1a mix ->
/// hex string). The same entity id always maps to the same canonical
/// id; ordering never matters.
pub fn stable_appliance_id(entity_id: &str) -> String {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut h1 = OFFSET;
    let mut h2 = OFFSET ^ 0x9e3779b97f4a7c15;
    for (i, b) in entity_id.bytes().enumerate() {
        h1 ^= u64::from(b);
        h1 = h1.wrapping_mul(PRIME);
        h2 ^= u64::from(b ^ (i as u8).wrapping_mul(0x5d));
        h2 = h2.wrapping_mul(PRIME);
    }
    format!("{h1:016x}{h2:016x}")
}

/// Derive the appliance capabilities from REAL entity features.
///
/// - PowerControl: the entity has a real controllable on/off surface
///   (input_boolean / switch / fan domains expose turn_on/turn_off).
/// - ModeControl: the entity exposes a variable mode attribute
///   (`percentage` for fan speed, `preset_mode` for presets).
/// - StatusReadback: every mapped entity has observable state.
///
/// Nothing is advertised from category defaults; the attributes are
/// the real provider-observed features.
pub fn capabilities_for(
    entity: &ApplianceEntity,
    mapper: &DeviceCapabilityMapper,
) -> Result<Vec<ApplianceCapability>, ApplianceError> {
    // The canonical appliance keys must map through the EP-010
    // taxonomy deterministically (closed table; unknown keys are
    // rejected, never invented).
    for key in ["appliance.power", "appliance.mode", "appliance.status"] {
        mapper.map(key).map_err(|error| {
            ApplianceError::new(
                ApplianceErrorCode::Internal,
                format!(
                    "canonical appliance key {key:?} rejected: {}",
                    error.message
                ),
                None,
                None,
            )
        })?;
    }

    let mut capabilities = Vec::new();
    if has_power_control(entity) {
        capabilities.push(ApplianceCapability::PowerControl);
    }
    if has_mode_control(entity) {
        capabilities.push(ApplianceCapability::ModeControl);
    }
    // Every selected entity has a readable provider state.
    capabilities.push(ApplianceCapability::StatusReadback);
    Ok(capabilities)
}

/// True when the entity exposes a real controllable on/off surface.
pub fn has_power_control(entity: &ApplianceEntity) -> bool {
    matches!(entity.domain.as_str(), "input_boolean" | "switch" | "fan")
}

/// True when the entity exposes a variable mode attribute (fan speed
/// percentage, preset mode). Provider-observed, never category-default.
pub fn has_mode_control(entity: &ApplianceEntity) -> bool {
    entity.attributes.contains_key("percentage") || entity.attributes.contains_key("preset_mode")
}

/// Extract the canonical power observation value ("ON"/"OFF") from the
/// real entity state. Provider-unavailable ("unavailable") and
/// uninitialized ("unknown") states are NOT mapped to a benign OFF.
pub fn power_value(entity: &ApplianceEntity) -> Option<String> {
    if entity.is_provider_unavailable() || entity.is_state_unknown() {
        None
    } else if entity.is_on() {
        Some("ON".to_string())
    } else {
        Some("OFF".to_string())
    }
}

/// Extract the canonical mode observation value (e.g. fan percentage
/// "37") from real attributes. HA reports percentages as JSON numbers
/// (37.0); integral floats are normalized to their exact integer form
/// so a canary value sent as 37 reads back as "37". None when the
/// entity has no mode.
pub fn mode_value(entity: &ApplianceEntity) -> Option<String> {
    entity
        .attributes
        .get("percentage")
        .map(normalize_number)
        .or_else(|| entity.attributes.get("preset_mode").map(Value::to_string))
}

/// Normalize a real provider attribute value to its canonical string
/// form: integral JSON numbers (37.0) become their integer form ("37"),
/// everything else keeps its JSON string form.
fn normalize_number(value: &Value) -> String {
    if let Some(number) = value.as_f64() {
        if number.fract() == 0.0 && number.abs() < 9_007_199_254_740_992.0 {
            return format!("{}", number as i64);
        }
    }
    value.to_string()
}

/// Build the real provider service payload for a mode command. The
/// value is passed through exactly (runtime-generated canary values
/// must read back unchanged). HA's fan.set_percentage schema coerces
/// integer percentages; numeric values are sent as JSON numbers (the
/// shape the real service expects), non-numeric mode values (e.g.
/// preset names) are sent as strings.
pub fn mode_payload(value: &str) -> BTreeMap<String, Value> {
    let mut data = BTreeMap::new();
    let value = match value.parse::<i64>() {
        Ok(number) => Value::from(number),
        Err(_) => Value::from(value),
    };
    data.insert("percentage".to_string(), value);
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn entity(entity_id: &str, domain: &str, state: &str) -> ApplianceEntity {
        ApplianceEntity {
            entity_id: entity_id.to_string(),
            domain: domain.to_string(),
            state: state.to_string(),
            attributes: BTreeMap::new(),
        }
    }

    #[test]
    fn ep024_unit_stable_appliance_id_is_deterministic_and_order_free() {
        let a1 = stable_appliance_id("fan.nexus_app_fan");
        let a2 = stable_appliance_id("fan.nexus_app_fan");
        assert_eq!(a1, a2);
        assert_eq!(a1.len(), 32);
        // Different entities -> different canonical ids.
        assert_ne!(
            stable_appliance_id("fan.nexus_app_fan"),
            stable_appliance_id("input_boolean.nexus_app_switch")
        );
    }

    #[test]
    fn ep024_unit_capabilities_from_real_features_switch() {
        let mapper = DeviceCapabilityMapper;
        let switch = entity("input_boolean.nexus_app_switch", "input_boolean", "off");
        let caps = capabilities_for(&switch, &mapper).expect("mapping");
        assert!(caps.contains(&ApplianceCapability::PowerControl));
        assert!(!caps.contains(&ApplianceCapability::ModeControl));
        assert!(caps.contains(&ApplianceCapability::StatusReadback));
    }

    #[test]
    fn ep024_unit_capabilities_from_real_features_fan() {
        let mapper = DeviceCapabilityMapper;
        let mut fan = entity("fan.nexus_app_fan", "fan", "on");
        fan.attributes
            .insert("percentage".to_string(), Value::from(37));
        let caps = capabilities_for(&fan, &mapper).expect("mapping");
        assert!(caps.contains(&ApplianceCapability::PowerControl));
        assert!(caps.contains(&ApplianceCapability::ModeControl));
        assert!(caps.contains(&ApplianceCapability::StatusReadback));
    }

    #[test]
    fn ep024_unit_no_capability_from_category_default() {
        // A sensor domain entity (no controllable surface) must NOT
        // gain PowerControl just because appliances usually have it.
        let mapper = DeviceCapabilityMapper;
        let sensor = entity("sensor.nexus_app_temp", "sensor", "21");
        let caps = capabilities_for(&sensor, &mapper).expect("mapping");
        assert!(!caps.contains(&ApplianceCapability::PowerControl));
        assert!(!caps.contains(&ApplianceCapability::ModeControl));
        assert!(caps.contains(&ApplianceCapability::StatusReadback));
    }

    #[test]
    fn ep024_unit_mode_payload_numeric_percentage_is_json_number() {
        let payload = mode_payload("37");
        assert_eq!(payload.get("percentage"), Some(&Value::from(37)));
        // The real fan.set_percentage service coerces int; the payload
        // must be the number 37, not the string "37".
        assert!(payload.get("percentage").unwrap().is_number());
    }

    #[test]
    fn ep024_unit_power_value_unavailable_never_off() {
        let mut e = entity("fan.nexus_app_fan", "fan", "unavailable");
        assert_eq!(power_value(&e), None);
        e.state = "unknown".to_string();
        assert_eq!(power_value(&e), None);
        e.state = "off".to_string();
        assert_eq!(power_value(&e), Some("OFF".to_string()));
        e.state = "on".to_string();
        assert_eq!(power_value(&e), Some("ON".to_string()));
    }

    #[test]
    fn ep024_unit_mode_value_normalizes_integral_float_percentage() {
        // The real provider reports percentage as a JSON number 37.0;
        // the canonical mode observation is the exact integer form.
        let mut fan = entity("fan.nexus_app_fan", "fan", "on");
        fan.attributes
            .insert("percentage".to_string(), Value::from(37.0));
        assert_eq!(mode_value(&fan), Some("37".to_string()));
        // Non-integral values keep their exact JSON form (honest).
        fan.attributes
            .insert("percentage".to_string(), Value::from(37.5));
        assert_eq!(mode_value(&fan), Some("37.5".to_string()));
        // No mode attribute -> None.
        let switch = entity("input_boolean.nexus_app_switch", "input_boolean", "off");
        assert_eq!(mode_value(&switch), None);
    }

    #[test]
    fn ep024_unit_selector_is_explicit_and_never_invents() {
        let selector =
            ApplianceSelector::entities(["fan.nexus_app_fan", "input_boolean.nexus_app_switch"]);
        assert!(selector.contains("fan.nexus_app_fan"));
        assert!(selector.contains("input_boolean.nexus_app_switch"));
        // Internal HA entities are never appliances.
        assert!(!selector.contains("sun.sun"));
        assert!(!selector.contains("zone.home"));
        assert!(!selector.contains("input_number.nexus_app_fan_speed"));
    }
}
