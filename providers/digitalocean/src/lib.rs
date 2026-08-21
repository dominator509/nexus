//! EP-036 DigitalOcean provider binding identity (SPEC-016).
//!
//! M1 declares the provider-neutral binding identity for the
//! DigitalOcean provider: the canonical ProviderKind, region slug
//! validation, and credential-reference semantics. No SDK import, no
//! network, no fabricated availability. Real API integration and
//! OpenTofu modules are owned by later milestones (M2-M5).

#![forbid(unsafe_code)]

use nexus_compute::error::{ComputeError, ComputeResult};
use nexus_compute::model::{CloudCredentialRef, ProviderBinding};
use nexus_compute::vocabulary::ProviderKind;
use nexus_domain::TenantId;

/// Canonical DigitalOcean region slug pattern. DigitalOcean region slugs
/// are lowercase letters followed by a digit (e.g. `nyc1`, `sfo3`). The
/// exact available set is provider data and must be read back from the
/// provider; M1 only validates the canonical slug shape so malformed
/// region strings fail closed before any API call.
pub fn is_valid_region_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 16 {
        return false;
    }
    let bytes = slug.as_bytes();
    // First character must be a lowercase letter.
    if !bytes[0].is_ascii_lowercase() {
        return false;
    }
    // Last character must be a digit (DigitalOcean slugs end with a
    // site number, e.g. `nyc1`).
    if !bytes[bytes.len() - 1].is_ascii_digit() {
        return false;
    }
    // Remaining characters must be lowercase letters or ASCII digits.
    bytes[1..bytes.len() - 1]
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// Build a DigitalOcean provider binding from an opaque credential
/// reference. The reference never contains the raw API token (SPEC-016
/// requirement 3: provider credentials remain in the local setup process
/// or short-lived OAuth and are discarded after provisioning).
pub fn binding(
    tenant: TenantId,
    account: impl Into<String>,
    region: impl Into<String>,
    credential_ref: CloudCredentialRef,
) -> ComputeResult<ProviderBinding> {
    let region = region.into();
    if !is_valid_region_slug(&region) {
        return Err(ComputeError::validation(format!(
            "invalid DigitalOcean region slug: '{region}'"
        )));
    }
    ProviderBinding::new(
        ProviderKind::DigitalOcean,
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
        TenantId::new("00000000-0000-7000-8000-0000000000aa").expect("tenant")
    }
    fn ref_() -> CloudCredentialRef {
        CloudCredentialRef::new("cred://vault/do-main").expect("ref")
    }

    #[test]
    fn ep036_unit_digitalocean_region_slug_shape() {
        for good in [
            "nyc1", "sfo3", "ams3", "blr1", "fra1", "lon1", "sgp1", "tor1", "syd1",
        ] {
            assert!(is_valid_region_slug(good), "{good}");
        }
        for bad in [
            "",
            "Nyc1",
            "nyc_1",
            "new-york-1",
            "nyc",
            "nyc123456789012345",
        ] {
            assert!(!is_valid_region_slug(bad), "{bad}");
        }
    }

    #[test]
    fn ep036_unit_digitalocean_binding_rejects_bad_region() {
        assert!(binding(tenant(), "acct-1", "Nyc1", ref_()).is_err());
        assert!(binding(tenant(), "acct-1", "", ref_()).is_err());
    }

    #[test]
    fn ep036_unit_digitalocean_binding_is_provider_kind_do() {
        let b = binding(tenant(), "acct-1", "nyc1", ref_()).expect("binding");
        assert_eq!(b.provider, ProviderKind::DigitalOcean);
        assert_eq!(b.region, "nyc1");
    }
}
