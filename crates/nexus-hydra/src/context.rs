//! EP-028 Hydra context projection (SPEC-015 behavior 1: Hydra remains
//! canonical; Nexus stores references and cross-domain projections).

use nexus_domain::BusinessId;
use serde::{Deserialize, Serialize};

use crate::model::{Campaign, CustomerReference};
use crate::vocabulary::HydraBindingId;

/// Cross-domain projection of Hydra context. This is a Nexus-side
/// projection, never a second CRM: it carries customer references and
/// campaign projections, not duplicated Hydra truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydraContextProjection {
    pub binding_id: HydraBindingId,
    pub business_id: BusinessId,
    pub customers: Vec<CustomerReference>,
    pub campaigns: Vec<Campaign>,
    /// RFC3339 timestamp when the projection was observed (data
    /// freshness).
    pub observed_at: String,
}

impl HydraContextProjection {
    pub fn new(
        binding_id: HydraBindingId,
        business_id: BusinessId,
        observed_at: impl Into<String>,
    ) -> Self {
        Self {
            binding_id,
            business_id,
            customers: Vec::new(),
            campaigns: Vec::new(),
            observed_at: observed_at.into(),
        }
    }

    pub fn with_customer(mut self, customer: CustomerReference) -> Self {
        self.customers.push(customer);
        self
    }

    pub fn with_campaign(mut self, campaign: Campaign) -> Self {
        self.campaigns.push(campaign);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::{CampaignId, CustomerReferenceId, IdentityResolutionClass};
    use nexus_domain::{BusinessId, PersonId};
    use std::str::FromStr;

    fn business() -> BusinessId {
        BusinessId::from_str("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap()
    }

    fn person() -> PersonId {
        PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
    }

    #[test]
    fn ep028_unit_projection_carries_references_only() {
        let projection = HydraContextProjection::new(
            HydraBindingId::new("binding-1").unwrap(),
            business(),
            "2026-08-19T00:00:00Z",
        )
        .with_customer(CustomerReference::new(
            CustomerReferenceId::new("cust-1").unwrap(),
            business(),
            person(),
            IdentityResolutionClass::Deterministic,
        ))
        .with_campaign(Campaign::new(
            CampaignId::new("camp-1").unwrap(),
            business(),
            "Q3",
        ));
        assert_eq!(projection.customers.len(), 1);
        assert_eq!(projection.campaigns.len(), 1);
        assert_eq!(projection.business_id, business());
        let json = serde_json::to_string(&projection).unwrap();
        let back: HydraContextProjection = serde_json::from_str(&json).unwrap();
        assert_eq!(back, projection);
    }
}
