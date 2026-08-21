//! EP-036 Contabo provider binding identity (SPEC-016).
//!
//! M4 declares the provider-neutral binding identity for the Contabo
//! provider: the canonical ProviderKind, region code shape validation,
//! and credential-reference semantics. No SDK import, no network, no
//! fabricated availability, no invented API surface. Real API
//! integration (Contabo OAuth2 client-credentials flow and compute
//! instance endpoints) is owned by later milestones and must follow
//! Contabo's documented API only.

#![forbid(unsafe_code)]

use nexus_compute::error::{ComputeError, ComputeResult};
use nexus_compute::model::{CloudCredentialRef, ProviderBinding};
use nexus_compute::vocabulary::ProviderKind;
use nexus_domain::TenantId;

/// Canonical Contabo region code pattern. Contabo's documented region
/// codes are short uppercase letter codes (e.g. `EU`, `US`, `SGP`,
/// `AUS`). M4 only validates the canonical shape so malformed region
/// strings fail closed before any API call; the exact available set is
/// provider readback data owned by the provider-certification milestone.
pub fn is_valid_region_code(code: &str) -> bool {
    if code.is_empty() || code.len() > 4 {
        return false;
    }
    code.bytes().all(|b| b.is_ascii_uppercase())
}

/// Build a Contabo provider binding from an opaque credential
/// reference. The reference never contains the raw client secret or
/// API token (SPEC-016 requirement 3: provider credentials remain in
/// the local setup process or short-lived OAuth and are discarded
/// after provisioning unless infrastructure management is enabled).
pub fn binding(
    tenant: TenantId,
    account: impl Into<String>,
    region: impl Into<String>,
    credential_ref: CloudCredentialRef,
) -> ComputeResult<ProviderBinding> {
    let region = region.into();
    if !is_valid_region_code(&region) {
        return Err(ComputeError::validation(format!(
            "invalid Contabo region code: '{region}'"
        )));
    }
    ProviderBinding::new(
        ProviderKind::Contabo,
        tenant,
        account,
        region,
        credential_ref,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_compute::model::CloudCredentialRef;

    fn tenant() -> TenantId {
        TenantId::new("00000000-0000-7000-8000-0000000000be").expect("tenant")
    }
    fn ref_() -> CloudCredentialRef {
        CloudCredentialRef::new("cred://vault/contabo-main").expect("ref")
    }

    #[test]
    fn ep036_unit_contabo_region_code_shape() {
        for good in ["EU", "US", "SGP", "AUS"] {
            assert!(is_valid_region_code(good), "{good}");
        }
        for bad in ["", "eu", "Eu", "EU-1", "EUROPE", "US-EAST", "1"] {
            assert!(!is_valid_region_code(bad), "{bad}");
        }
    }

    #[test]
    fn ep036_unit_contabo_binding_rejects_bad_region() {
        assert!(binding(tenant(), "acct-1", "eu", ref_()).is_err());
        assert!(binding(tenant(), "acct-1", "", ref_()).is_err());
    }

    #[test]
    fn ep036_unit_contabo_binding_is_provider_kind_contabo() {
        let b = binding(tenant(), "acct-1", "EU", ref_()).expect("binding");
        assert_eq!(b.provider, ProviderKind::Contabo);
        assert_eq!(b.region, "EU");
    }
}
