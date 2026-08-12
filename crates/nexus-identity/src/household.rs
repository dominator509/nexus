//! Household (SPEC-001, EP-003).
//!
//! A household is an independently scoped group of people sharing a home
//! context. It holds member references, never the person records.

use std::fmt;

use nexus_domain::{HouseholdId, PersonId, TenantId};
use serde::{Deserialize, Serialize};

/// Error returned by household construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseholdError {
    /// The household name is empty or whitespace only.
    EmptyName,
}

impl fmt::Display for HouseholdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => f.write_str("household name must not be empty"),
        }
    }
}

impl std::error::Error for HouseholdError {}

/// A household: a bounded group of people sharing a home context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Household {
    /// Household identifier (SPEC-001).
    pub household_id: HouseholdId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Display name.
    pub name: String,
    /// Member references (independently scoped person records).
    pub member_ids: Vec<PersonId>,
}

impl Household {
    /// Construct a validated household.
    pub fn new(
        household_id: HouseholdId,
        tenant_id: TenantId,
        name: impl Into<String>,
        member_ids: Vec<PersonId>,
    ) -> Result<Self, HouseholdError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(HouseholdError::EmptyName);
        }
        Ok(Self {
            household_id,
            tenant_id,
            name,
            member_ids,
        })
    }

    /// Whether the given person is a member.
    pub fn contains_member(&self, person_id: &PersonId) -> bool {
        self.member_ids.contains(person_id)
    }

    /// Number of members.
    pub fn member_count(&self) -> usize {
        self.member_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";

    #[test]
    fn ep003_unit_household_constructs_with_members() {
        let m1 = PersonId::new(PID).unwrap();
        let m2 = PersonId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6110").unwrap();
        let h = Household::new(
            HouseholdId::new(HID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "The Lin Household",
            vec![m1.clone(), m2],
        )
        .unwrap();
        assert_eq!(h.member_count(), 2);
        assert!(h.contains_member(&m1));
    }

    #[test]
    fn ep003_unit_household_rejects_empty_name() {
        let res = Household::new(
            HouseholdId::new(HID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "",
            vec![],
        );
        assert_eq!(res, Err(HouseholdError::EmptyName));
    }

    #[test]
    fn ep003_unit_household_serde_roundtrip() {
        let h = Household::new(
            HouseholdId::new(HID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "Home",
            vec![],
        )
        .unwrap();
        let json = serde_json::to_string(&h).unwrap();
        let back: Household = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }
}
