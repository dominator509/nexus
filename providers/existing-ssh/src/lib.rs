//! EP-036 generic existing-SSH provider binding (SPEC-016).
//!
//! Fully local and existing SSH remain first-class paths (node
//! contract). M3 declares the provider-neutral binding identity for an
//! existing SSH host: host, port, bootstrap user, and credential
//! reference - plus a REAL transport reachability proof against an
//! ephemeral sshd container in the integration suite. No SDK import; the
//! transport proof uses the real `ssh`/`ssh-keyscan` binaries.

#![forbid(unsafe_code)]

pub use crate::model::{ExistingSshBinding, SshProbeState};

mod model {
    //! Existing-SSH binding value objects kept in a private module and
    //! re-exported so the public surface stays explicit.

    use nexus_compute::error::{ComputeError, ComputeResult};
    use nexus_compute::model::{CloudCredentialRef, ProviderBinding};
    use nexus_compute::vocabulary::ProviderKind;
    use nexus_domain::TenantId;

    /// An existing SSH host binding: the exact target the fabric may use.
    /// Host identity is explicit (never inferred from display labels);
    /// reachability is proven separately by transport readback.
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ExistingSshBinding {
        pub host: String,
        pub port: u16,
        pub user: String,
        pub tenant: TenantId,
        pub credential_ref: CloudCredentialRef,
    }

    impl ExistingSshBinding {
        pub fn new(
            host: impl Into<String>,
            port: u16,
            user: impl Into<String>,
            tenant: TenantId,
            credential_ref: CloudCredentialRef,
        ) -> ComputeResult<Self> {
            let host = host.into();
            let user = user.into();
            if host.is_empty() || host.len() > 253 {
                return Err(ComputeError::validation("host must be 1..=253 characters"));
            }
            if port == 0 {
                return Err(ComputeError::validation("port must be > 0"));
            }
            if user.is_empty() || user.len() > 64 {
                return Err(ComputeError::validation("user must be 1..=64 characters"));
            }
            Ok(Self {
                host,
                port,
                user,
                tenant,
                credential_ref,
            })
        }

        /// Build the provider binding used by the fabric for this host.
        pub fn to_provider_binding(
            &self,
            account: impl Into<String>,
        ) -> ComputeResult<ProviderBinding> {
            ProviderBinding::new(
                ProviderKind::GenericSsh,
                self.tenant.clone(),
                account,
                format!("ssh:{}", self.host),
                self.credential_ref.clone(),
            )
        }
    }

    /// Transport reachability probe result. A reachable SSH endpoint
    /// proves only REACHABLE, never READY and never WORKLOAD HEALTHY.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SshProbeState {
        Reachable,
        Unreachable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_compute::model::CloudCredentialRef;
    use nexus_compute::vocabulary::ProviderKind;
    use nexus_domain::TenantId;

    fn tenant() -> TenantId {
        TenantId::new("00000000-0000-7000-8000-0000000000cc").expect("tenant")
    }
    fn ref_() -> CloudCredentialRef {
        CloudCredentialRef::new("cred://vault/ssh-main").expect("ref")
    }

    #[test]
    fn ep036_unit_existing_ssh_binding_validates() {
        let b = ExistingSshBinding::new("10.0.0.5", 22, "root", tenant(), ref_()).expect("binding");
        assert_eq!(b.host, "10.0.0.5");
        assert_eq!(b.port, 22);
        assert_eq!(b.user, "root");
    }

    #[test]
    fn ep036_unit_existing_ssh_binding_rejects_bad_host() {
        assert!(ExistingSshBinding::new("", 22, "root", tenant(), ref_()).is_err());
        assert!(ExistingSshBinding::new("host", 0, "root", tenant(), ref_()).is_err());
        assert!(ExistingSshBinding::new("host", 22, "", tenant(), ref_()).is_err());
    }

    #[test]
    fn ep036_unit_existing_ssh_binding_maps_to_generic_ssh() {
        let b = ExistingSshBinding::new("10.0.0.5", 22, "root", tenant(), ref_()).expect("binding");
        let pb = b.to_provider_binding("acct-1").expect("provider binding");
        assert_eq!(pb.provider, ProviderKind::GenericSsh);
        assert_eq!(pb.region, "ssh:10.0.0.5");
    }
}
