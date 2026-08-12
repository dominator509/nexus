//! Person profile (SPEC-001, EP-003).
//!
//! A person is independently scoped from households, businesses, devices,
//! and sessions. The profile carries identity references and lifecycle
//! state; it never embeds another entity's primary record.

use std::fmt;

use nexus_domain::{BusinessId, HouseholdId, PersonId, TenantId};
use serde::{Deserialize, Serialize};

/// Lifecycle state of a world entity (SPEC-001; ADR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleState {
    Pending,
    Active,
    Suspended,
    Disabled,
    Archived,
}

impl LifecycleState {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Active => "ACTIVE",
            Self::Suspended => "SUSPENDED",
            Self::Disabled => "DISABLED",
            Self::Archived => "ARCHIVED",
        }
    }
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned by person profile construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersonProfileError {
    /// The display name is empty or whitespace only.
    EmptyDisplayName,
}

impl fmt::Display for PersonProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDisplayName => f.write_str("person display name must not be empty"),
        }
    }
}

impl std::error::Error for PersonProfileError {}

/// A person profile: identity references and lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonProfile {
    /// Person identifier (SPEC-001).
    pub person_id: PersonId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Display name shown in UI and voice responses.
    pub display_name: String,
    /// Current lifecycle state.
    pub lifecycle_state: LifecycleState,
    /// Household membership reference (independently scoped).
    pub household_id: Option<HouseholdId>,
    /// Business membership references (independently scoped).
    pub business_ids: Vec<BusinessId>,
}

impl PersonProfile {
    /// Construct a validated person profile.
    pub fn new(
        person_id: PersonId,
        tenant_id: TenantId,
        display_name: impl Into<String>,
        lifecycle_state: LifecycleState,
        household_id: Option<HouseholdId>,
        business_ids: Vec<BusinessId>,
    ) -> Result<Self, PersonProfileError> {
        let display_name = display_name.into();
        if display_name.trim().is_empty() {
            return Err(PersonProfileError::EmptyDisplayName);
        }
        Ok(Self {
            person_id,
            tenant_id,
            display_name,
            lifecycle_state,
            household_id,
            business_ids,
        })
    }

    /// Whether this person is active and usable as an actor.
    pub fn is_active(&self) -> bool {
        self.lifecycle_state == LifecycleState::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{BusinessId, HouseholdId};

    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const HID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103";
    const BID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6104";

    #[test]
    fn ep003_unit_person_profile_constructs_and_is_active() {
        let p = PersonProfile::new(
            PersonId::new(PID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "Lin",
            LifecycleState::Active,
            Some(HouseholdId::new(HID).unwrap()),
            vec![BusinessId::new(BID).unwrap()],
        )
        .unwrap();
        assert!(p.is_active());
        assert_eq!(p.display_name, "Lin");
        assert_eq!(p.business_ids.len(), 1);
    }

    #[test]
    fn ep003_unit_person_profile_rejects_empty_display_name() {
        let res = PersonProfile::new(
            PersonId::new(PID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "   ",
            LifecycleState::Active,
            None,
            vec![],
        );
        assert_eq!(res, Err(PersonProfileError::EmptyDisplayName));
    }

    #[test]
    fn ep003_unit_person_profile_serde_roundtrip() {
        let p = PersonProfile::new(
            PersonId::new(PID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "Lin",
            LifecycleState::Pending,
            None,
            vec![],
        )
        .unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let back: PersonProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
        assert!(json.contains("\"lifecycle_state\":\"PENDING\""));
        assert!(json.contains("\"display_name\":\"Lin\""));
    }

    #[test]
    fn ep003_unit_person_profile_is_independently_scoped() {
        // A PersonProfile references HouseholdId/BusinessId by value; it can
        // never BE a Household or a Business at the type level.
        let p = PersonProfile::new(
            PersonId::new(PID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "Lin",
            LifecycleState::Active,
            None,
            vec![],
        )
        .unwrap();
        assert_eq!(p.household_id, None);
        // Compile-time proof: PersonId cannot be passed where HouseholdId is
        // expected. The following line would not compile:
        // let _h: HouseholdId = p.person_id;
    }
}
