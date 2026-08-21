//! EP-036 Hetzner provider binding identity (SPEC-016).
//!
//! M5 declares the provider-neutral binding identity for the Hetzner
//! provider: the canonical ProviderKind, location slug shape
//! validation, and credential-reference semantics. No SDK import, no
//! network, no fabricated availability, no invented API surface. Real
//! API integration (Hetzner Cloud API v1 token auth and server
//! endpoints) is owned by the later provider-certification milestone
//! and must follow Hetzner's documented API only.

#![forbid(unsafe_code)]

use nexus_compute::error::{ComputeError, ComputeResult};
use nexus_compute::model::{CloudCredentialRef, ProviderBinding};
use nexus_compute::vocabulary::ProviderKind;
use nexus_domain::TenantId;

/// Canonical Hetzner location slug pattern. Hetzner's documented
/// location slugs are short lowercase names (e.g. `fsn1`, `nbg1`,
/// `hel1`, `ash`, `hil`). M5 only validates the canonical shape so
/// malformed location strings fail closed before any API call; the
/// exact available set is provider readback data owned by the
/// provider-certification milestone.
pub fn is_valid_location_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 8 {
        return false;
    }
    // Hetzner location slugs begin with a letter and may end in a
    // digit (e.g. fsn1, nbg1, hel1, ash, hil); a digit-first string is
    // not a location slug.
    let mut chars = slug.bytes();
    let first = chars.next().expect("non-empty");
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// Build a Hetzner provider binding from an opaque credential
/// reference. The reference never contains the raw API token
/// (SPEC-016 requirement 3: provider credentials remain in the local
/// setup process or short-lived OAuth and are discarded after
/// provisioning unless infrastructure management is enabled).
pub fn binding(
    tenant: TenantId,
    account: impl Into<String>,
    region: impl Into<String>,
    credential_ref: CloudCredentialRef,
) -> ComputeResult<ProviderBinding> {
    let region = region.into();
    if !is_valid_location_slug(&region) {
        return Err(ComputeError::validation(format!(
            "invalid Hetzner location slug: '{region}'"
        )));
    }
    ProviderBinding::new(
        ProviderKind::Hetzner,
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
        TenantId::new("00000000-0000-7000-8000-0000000000bf").expect("tenant")
    }
    fn ref_() -> CloudCredentialRef {
        CloudCredentialRef::new("cred://vault/hetzner-main").expect("ref")
    }

    #[test]
    fn ep036_unit_hetzner_location_slug_shape() {
        for good in ["fsn1", "nbg1", "hel1", "ash", "hil"] {
            assert!(is_valid_location_slug(good), "{good}");
        }
        for bad in ["", "FSN1", "fsn-1", "fsn1_extra", "1fsn"] {
            assert!(!is_valid_location_slug(bad), "{bad}");
        }
    }

    #[test]
    fn ep036_unit_hetzner_binding_rejects_bad_location() {
        assert!(binding(tenant(), "acct-1", "FSN1", ref_()).is_err());
        assert!(binding(tenant(), "acct-1", "", ref_()).is_err());
    }

    #[test]
    fn ep036_unit_hetzner_binding_is_provider_kind_hetzner() {
        let b = binding(tenant(), "acct-1", "fsn1", ref_()).expect("binding");
        assert_eq!(b.provider, ProviderKind::Hetzner);
        assert_eq!(b.region, "fsn1");
    }
}
