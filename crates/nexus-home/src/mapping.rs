//! Provider-neutral canonical mapping (SPEC-011; ADR-027).
//!
//! Deterministic mapping rules between provider domains and canonical
//! Nexus device categories, plus stable identity resolution guidance.
//! Home Assistant domain names never leak into user-facing semantics;
//! they are normalized here at the infrastructure boundary.
//!
//! Identity resolution prefers strong provider identifiers (HA device
//! id, entity registry identity, integration identifiers/connections)
//! over mutable display names. The mapping survives friendly-name
//! changes, room changes, restart, and discovery refresh.

use crate::vocabulary::DeviceCategory;

/// Deterministically map a provider domain (e.g. Home Assistant
/// `light`, `lock`, `climate`) into the canonical device category.
///
/// Unknown domains map to `Other`; the mapping is total and stable.
/// This table is provider-neutral by design; the Home Assistant adapter
/// calls it for every discovered entity.
pub fn category_from_provider_domain(domain: &str) -> DeviceCategory {
    match domain {
        "light" => DeviceCategory::Light,
        "switch" => DeviceCategory::Switch,
        "lock" => DeviceCategory::Lock,
        "climate" => DeviceCategory::Climate,
        "cover" => DeviceCategory::Cover,
        "sensor" => DeviceCategory::Sensor,
        "binary_sensor" => DeviceCategory::BinarySensor,
        "media_player" => DeviceCategory::MediaPlayer,
        "camera" => DeviceCategory::Camera,
        "fan" => DeviceCategory::Fan,
        "vacuum" => DeviceCategory::Vacuum,
        "alarm_control_panel" => DeviceCategory::Alarm,
        "scene" => DeviceCategory::Scene,
        "button" => DeviceCategory::Button,
        "number" => DeviceCategory::Number,
        "select" => DeviceCategory::Select,
        _ => DeviceCategory::Other,
    }
}

/// Stability rule: a provider identifier is a STRONG identity when it
/// is not a mutable display name. Home Assistant entity ids such as
/// `light.kitchen` and device registry ids are strong; `friendly_name`
/// is never strong.
pub fn is_strong_provider_identity(reference: &str) -> bool {
    // HA entity ids and device registry ids are stable dotted keys
    // (domain.object). Display names may contain spaces or any other
    // text and are not identity.
    !reference.is_empty()
        && reference.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-'
        })
}

/// Deterministic canonical action key for a category + provider
/// service. Kept provider-neutral: consumers reason about canonical
/// actions, never about `light/turn_on` directly.
pub fn canonical_action(category: DeviceCategory, action: &str) -> String {
    match action {
        "turn_on" => match category {
            DeviceCategory::Light
            | DeviceCategory::Switch
            | DeviceCategory::Fan
            | DeviceCategory::MediaPlayer => "turn_on".to_string(),
            _ => action.to_string(),
        },
        "turn_off" => match category {
            DeviceCategory::Light
            | DeviceCategory::Switch
            | DeviceCategory::Fan
            | DeviceCategory::MediaPlayer => "turn_off".to_string(),
            _ => action.to_string(),
        },
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep020_unit_mapping_covers_common_categories() {
        assert_eq!(
            category_from_provider_domain("light"),
            DeviceCategory::Light
        );
        assert_eq!(
            category_from_provider_domain("switch"),
            DeviceCategory::Switch
        );
        assert_eq!(category_from_provider_domain("lock"), DeviceCategory::Lock);
        assert_eq!(
            category_from_provider_domain("climate"),
            DeviceCategory::Climate
        );
        assert_eq!(
            category_from_provider_domain("cover"),
            DeviceCategory::Cover
        );
        assert_eq!(
            category_from_provider_domain("sensor"),
            DeviceCategory::Sensor
        );
        assert_eq!(
            category_from_provider_domain("binary_sensor"),
            DeviceCategory::BinarySensor
        );
        assert_eq!(
            category_from_provider_domain("media_player"),
            DeviceCategory::MediaPlayer
        );
        assert_eq!(
            category_from_provider_domain("camera"),
            DeviceCategory::Camera
        );
        assert_eq!(category_from_provider_domain("fan"), DeviceCategory::Fan);
        assert_eq!(
            category_from_provider_domain("vacuum"),
            DeviceCategory::Vacuum
        );
        assert_eq!(
            category_from_provider_domain("alarm_control_panel"),
            DeviceCategory::Alarm
        );
        assert_eq!(
            category_from_provider_domain("scene"),
            DeviceCategory::Scene
        );
        assert_eq!(
            category_from_provider_domain("button"),
            DeviceCategory::Button
        );
        assert_eq!(
            category_from_provider_domain("number"),
            DeviceCategory::Number
        );
        assert_eq!(
            category_from_provider_domain("select"),
            DeviceCategory::Select
        );
    }

    #[test]
    fn ep020_unit_mapping_unknown_domain_maps_other() {
        assert_eq!(
            category_from_provider_domain("future_thing"),
            DeviceCategory::Other
        );
        assert_eq!(category_from_provider_domain(""), DeviceCategory::Other);
        // Case matters: the adapter passes exact HA domain strings.
        assert_eq!(
            category_from_provider_domain("Light"),
            DeviceCategory::Other
        );
    }

    #[test]
    fn ep020_unit_mapping_is_deterministic_and_total() {
        // Every input maps to a category; repeated calls agree.
        for domain in ["light", "lock", "climate", "unknown_x", "camera", ""] {
            let a = category_from_provider_domain(domain);
            let b = category_from_provider_domain(domain);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn ep020_unit_strong_identity_rejects_display_names() {
        assert!(is_strong_provider_identity("light.kitchen"));
        assert!(is_strong_provider_identity("device_abc123"));
        assert!(is_strong_provider_identity("a1.b2-c3"));
        // Display names with spaces or mixed case are not identity.
        assert!(!is_strong_provider_identity("Kitchen Light"));
        assert!(!is_strong_provider_identity(""));
        assert!(!is_strong_provider_identity("Kitchen.Light"));
    }

    #[test]
    fn ep020_unit_canonical_action_is_stable() {
        assert_eq!(
            canonical_action(DeviceCategory::Light, "turn_on"),
            "turn_on"
        );
        assert_eq!(
            canonical_action(DeviceCategory::Switch, "turn_off"),
            "turn_off"
        );
        assert_eq!(canonical_action(DeviceCategory::Lock, "unlock"), "unlock");
    }
}
