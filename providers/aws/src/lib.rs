//! EP-036 AWS provider binding identity (SPEC-016).
//!
//! M2 declares the provider-neutral binding identity for the AWS
//! provider: the canonical ProviderKind, region slug validation, and
//! credential-reference semantics. No SDK import, no network, no
//! fabricated availability. Real API integration and OpenTofu modules
//! are owned by later milestones (M3-M5).

#![forbid(unsafe_code)]

use nexus_compute::error::{ComputeError, ComputeResult};
use nexus_compute::model::{CloudCredentialRef, ProviderBinding};
use nexus_compute::vocabulary::ProviderKind;
use nexus_domain::TenantId;

/// Canonical AWS region slug pattern. AWS region slugs are two lowercase
/// letters, a dash, a region name, a dash, and a digit (e.g. `us-east-1`,
/// `eu-west-2`, `ap-southeast-1`). The exact available set is provider
/// data and must be read back from the provider; M2 only validates the
/// canonical slug shape so malformed region strings fail closed before
/// any API call.
pub fn is_valid_region_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 24 {
        return false;
    }
    let parts: Vec<&str> = slug.split('-').collect();
    if parts.len() < 3 {
        return false;
    }
    // First part must be two lowercase letters (partition, e.g. us/eu/ap).
    if parts[0].len() != 2 || !parts[0].bytes().all(|b| b.is_ascii_lowercase()) {
        return false;
    }
    // Middle parts must be lowercase letters.
    for part in &parts[1..parts.len() - 1] {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_lowercase()) {
            return false;
        }
    }
    // Last part must be a single digit.
    let last = parts[parts.len() - 1];
    last.len() == 1 && last.bytes().all(|b| b.is_ascii_digit())
}

/// Build an AWS provider binding from an opaque credential reference.
/// The reference never contains the raw access key or secret
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
    if !is_valid_region_slug(&region) {
        return Err(ComputeError::validation(format!(
            "invalid AWS region slug: '{region}'"
        )));
    }
    ProviderBinding::new(ProviderKind::Aws, tenant, account, region, credential_ref)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_compute::model::CloudCredentialRef;

    fn tenant() -> TenantId {
        TenantId::new("00000000-0000-7000-8000-0000000000bb").expect("tenant")
    }
    fn ref_() -> CloudCredentialRef {
        CloudCredentialRef::new("cred://vault/aws-main").expect("ref")
    }

    #[test]
    fn ep036_unit_aws_region_slug_shape() {
        for good in [
            "us-east-1",
            "us-west-2",
            "eu-west-2",
            "ap-southeast-1",
            "ap-northeast-3",
            "sa-east-1",
            "ca-central-1",
            "me-south-1",
            "af-south-1",
        ] {
            assert!(is_valid_region_slug(good), "{good}");
        }
        for bad in [
            "",
            "USEast1",
            "us_east_1",
            "us-east",
            "us-east-12",
            "us-east-1-extra",
            "1-east-1",
        ] {
            assert!(!is_valid_region_slug(bad), "{bad}");
        }
    }

    #[test]
    fn ep036_unit_aws_binding_rejects_bad_region() {
        assert!(binding(tenant(), "acct-1", "USEast1", ref_()).is_err());
        assert!(binding(tenant(), "acct-1", "", ref_()).is_err());
    }

    #[test]
    fn ep036_unit_aws_binding_is_provider_kind_aws() {
        let b = binding(tenant(), "acct-1", "us-east-1", ref_()).expect("binding");
        assert_eq!(b.provider, ProviderKind::Aws);
        assert_eq!(b.region, "us-east-1");
    }
}
