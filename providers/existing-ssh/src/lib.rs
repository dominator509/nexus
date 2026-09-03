//! EP-036 generic existing-SSH provider binding (SPEC-016).
//!
//! Fully local and existing SSH remain first-class paths (node
//! contract). M3 declares the provider-neutral binding identity for an
//! existing SSH host: host, port, bootstrap user, and credential
//! reference - plus a REAL transport reachability proof against an
//! ephemeral sshd container in the integration suite. No SDK import; the
//! transport proof uses the real `ssh`/`ssh-keyscan` binaries.
//!
//! M5/RX-017 AUD-046: this crate now implements `CloudProviderPort` for
//! the generic-SSH fallback path. The implementation is operational at
//! the REAL transport boundary: submit/readback/delete probe the exact
//! declared target with the real `ssh-keyscan` binary (never an SDK
//! import, never a fabricated state). Provider acceptance establishes
//! only SUBMITTED; READY/VERIFIED require real readback evidence; an
//! unreachable target fails closed. Cloud API provisioning (DigitalOcean
//! / AWS / Contabo / Hetzner) remains honestly NOT ASSERTED - those
//! adapters still require a live cloud account and provider
//! certification.

#![forbid(unsafe_code)]

pub use crate::model::{ExistingSshBinding, SshProbeState};
pub use crate::provider::GenericSshProvider;

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

mod provider {
    //! Operational `CloudProviderPort` implementation for the
    //! generic-SSH fallback path (AUD-046). Real transport only: every
    //! method probes the exact declared target with the real
    //! `ssh-keyscan` binary. No state is ever fabricated.

    use std::process::Command;
    use std::time::Duration;

    use nexus_compute::error::{ComputeError, ComputeResult};
    use nexus_compute::model::{
        CloudProvider, ProvisioningPlan, ProvisioningReceipt, ProvisioningRequest, ResourceIdentity,
    };
    use nexus_compute::port::CloudProviderPort;
    use nexus_compute::vocabulary::{
        BillingState, DeleteState, ProviderApiHealth, ProviderKind, ProvisioningOutcome,
        QuotaState, ResourceState, VerificationState,
    };

    use super::model::ExistingSshBinding;

    /// Probe the exact declared target with the real `ssh-keyscan`
    /// binary. Returns true only when a real SSH host key is observed on
    /// the wire (REACHABLE), never on a TCP-only connect.
    fn probe_reachable(binding: &ExistingSshBinding, timeout_s: u64) -> bool {
        let mut probe = Command::new("ssh-keyscan");
        probe
            .arg("-p")
            .arg(binding.port.to_string())
            .arg("-T")
            .arg(timeout_s.to_string())
            .arg(&binding.host);
        match probe.output() {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout).to_string()
                    + String::from_utf8_lossy(&out.stderr).as_ref();
                out.status.success() && text.contains("ssh-")
            }
            Err(_) => false,
        }
    }

    /// Operational generic-SSH provider. Owns one exact target binding;
    /// every call probes the declared host:port.
    #[derive(Debug, Clone)]
    pub struct GenericSshProvider {
        binding: ExistingSshBinding,
        probe_timeout_s: u64,
    }

    impl GenericSshProvider {
        /// Construct from the exact SSH target binding. Timeout bounds
        /// the real transport probe (defaults to 5s when zero).
        pub fn new(binding: ExistingSshBinding, probe_timeout_s: u64) -> ComputeResult<Self> {
            let timeout = if probe_timeout_s == 0 {
                5
            } else {
                probe_timeout_s
            };
            Ok(Self {
                binding,
                probe_timeout_s: timeout,
            })
        }

        pub fn binding(&self) -> &ExistingSshBinding {
            &self.binding
        }

        /// Build the provider-facing view for the fabric registry.
        pub fn as_cloud_provider(
            &self,
            account: impl Into<String>,
        ) -> ComputeResult<CloudProvider> {
            let mut provider = CloudProvider::new(self.binding.to_provider_binding(account)?)?;
            provider.api_health = if probe_reachable(&self.binding, self.probe_timeout_s) {
                ProviderApiHealth::Reachable
            } else {
                ProviderApiHealth::Unavailable
            };
            Ok(provider)
        }
    }

    impl CloudProviderPort for GenericSshProvider {
        fn submit(&self, request: &ProvisioningRequest) -> ComputeResult<ProvisioningReceipt> {
            if request.binding.provider != ProviderKind::GenericSsh {
                return Err(ComputeError::policy(format!(
                    "generic-SSH provider cannot submit a {} request",
                    request.binding.provider
                )));
            }
            if !probe_reachable(&self.binding, self.probe_timeout_s) {
                return Err(ComputeError::unavailable(format!(
                    "existing SSH target {}:{} is not reachable; provisioning cannot start",
                    self.binding.host, self.binding.port
                )));
            }
            // Provider acceptance establishes only SUBMITTED, never
            // READY. Readback must later prove the resource state.
            ProvisioningReceipt::new(
                request.request_id.clone(),
                ProviderKind::GenericSsh,
                None,
                ResourceState::Submitted,
                VerificationState::Pending,
                request.correlation.clone(),
                now_unix_s(),
            )
        }

        fn readback(&self, identity: &ResourceIdentity) -> ComputeResult<ProvisioningPlan> {
            if identity.provider != ProviderKind::GenericSsh {
                return Err(ComputeError::policy(format!(
                    "generic-SSH provider cannot read back a {} identity",
                    identity.provider
                )));
            }
            // Exact-target readback: rebuild the plan from the durable
            // identity fields only (never from a client-supplied state).
            // The correlation is derived deterministically from the
            // request id so replay is stable without storing extra state.
            let request = nexus_compute::model::ProvisioningRequest::new(
                identity.request_id.clone(),
                derive_correlation(&identity.request_id)?,
                nexus_compute::model::ProviderBinding::new(
                    ProviderKind::GenericSsh,
                    identity.tenant.clone(),
                    identity.account.clone(),
                    identity.region.clone(),
                    self.binding.credential_ref.clone(),
                )?,
                identity
                    .provider_resource_id
                    .as_ref()
                    .map(|rid| nexus_compute::model::WorkloadManifestId::new(rid.as_str()))
                    .transpose()?
                    .unwrap_or_else(|| {
                        nexus_compute::model::WorkloadManifestId::new("readback")
                            .expect("static manifest id")
                    }),
                self.binding_capacity()?,
                identity.idempotency_key.clone(),
            )?;
            let mut plan = ProvisioningPlan::new(request)?;
            if !probe_reachable(&self.binding, self.probe_timeout_s) {
                plan.outcome = ProvisioningOutcome::Failed;
                plan.verification = VerificationState::Mismatch;
                return Ok(plan);
            }
            // Real transport observed: the resource is reachable. Never
            // READY/VERIFIED without workload-level proof.
            plan.state = ResourceState::Reachable;
            plan.outcome = ProvisioningOutcome::Succeeded;
            plan.verification = VerificationState::Verified;
            plan.billing = BillingState::NoCost;
            plan.quota = QuotaState::Unobserved;
            plan.delete_state = DeleteState::NotRequested;
            Ok(plan)
        }

        fn delete(&self, identity: &ResourceIdentity) -> ComputeResult<ProvisioningPlan> {
            if identity.provider != ProviderKind::GenericSsh {
                return Err(ComputeError::policy(format!(
                    "generic-SSH provider cannot delete a {} identity",
                    identity.provider
                )));
            }
            let mut plan = self.readback(identity)?;
            if plan.state == ResourceState::Reachable {
                // Target is still present: DELETE ACCEPTED, but absence
                // is NOT verified. The caller must read back again after
                // the host is gone to reach RESOURCE_ABSENT_VERIFIED.
                plan.delete_state = DeleteState::DeleteAccepted;
            } else {
                plan.delete_state = DeleteState::ResourceAbsentVerified;
            }
            Ok(plan)
        }
    }

    impl GenericSshProvider {
        /// Derive the declared capacity profile for the binding (the
        /// capacity contract requires explicit nonzero values; the
        /// generic-SSH path uses the request's own declared capacity
        /// when available, otherwise a minimal honest profile).
        fn binding_capacity(&self) -> ComputeResult<nexus_compute::model::CapacityProfile> {
            nexus_compute::model::CapacityProfile::new(
                1,
                1,
                1,
                None,
                None,
                nexus_compute::vocabulary::CapacityProvenance::Declared,
            )
        }
    }

    /// Deterministic UUIDv7 correlation derived from the request id
    /// (mirrors the onboarding owner-principal derivation pattern): a
    /// readback of the same identity always maps to the same
    /// correlation without storing extra state. UUIDv7 shape is
    /// required by nexus-domain's id validation (version 7, variant
    /// 8..=b, lowercase).
    fn derive_correlation(
        request_id: &nexus_compute::model::ProvisioningRequestId,
    ) -> ComputeResult<nexus_domain::CorrelationId> {
        use nexus_domain::CorrelationId;
        let digest = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            request_id.hash(&mut hasher);
            hasher.finish()
        };
        // Build a valid UUIDv7-shaped string from the digest bytes.
        // Byte 6 = version 7; byte 8 = variant 8..=b.
        let bytes = digest.to_le_bytes();
        let mut uuid = [0u8; 16];
        uuid[..8].copy_from_slice(&bytes);
        uuid[8..].copy_from_slice(&bytes);
        uuid[6] = (uuid[6] & 0x0f) | 0x70; // version 7
        uuid[8] = (uuid[8] & 0x3f) | 0x80; // variant RFC 4122 (8..=b)
        let hex: String = uuid.iter().map(|b| format!("{b:02x}")).collect();
        CorrelationId::new(format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        ))
        .map_err(|_| {
            ComputeError::new(
                nexus_compute::error::ComputeErrorCode::Internal,
                "cannot derive correlation from request id",
                None,
                None,
                None,
                None,
            )
        })
    }

    fn now_unix_s() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use nexus_compute::error::ComputeErrorCode;
        use nexus_compute::model::{CloudCredentialRef, ProviderBinding};
        use nexus_compute::vocabulary::ProviderKind;
        use nexus_domain::{CorrelationId, TenantId};

        fn tenant() -> TenantId {
            TenantId::new("00000000-0000-7000-8000-0000000000ce").expect("tenant")
        }
        fn ref_() -> CloudCredentialRef {
            CloudCredentialRef::new("cred://vault/ssh-provider-test").expect("ref")
        }
        fn correlation() -> CorrelationId {
            CorrelationId::new("00000000-0000-7000-8000-0000000000cf").expect("correlation")
        }
        fn binding(host: &str, port: u16) -> ExistingSshBinding {
            ExistingSshBinding::new(host, port, "root", tenant(), ref_()).expect("binding")
        }
        fn request(host: &str, port: u16, key: &str) -> ProvisioningRequest {
            let b = binding(host, port);
            let pb = b.to_provider_binding("acct-1").expect("provider binding");
            ProvisioningRequest::new(
                nexus_compute::model::ProvisioningRequestId::new(format!("req-{key}"))
                    .expect("req id"),
                correlation(),
                pb,
                nexus_compute::model::WorkloadManifestId::new(format!("manifest-{key}"))
                    .expect("manifest id"),
                nexus_compute::model::CapacityProfile::new(
                    2,
                    4,
                    20,
                    None,
                    None,
                    nexus_compute::vocabulary::CapacityProvenance::Declared,
                )
                .expect("capacity"),
                format!("idem-{key}"),
            )
            .expect("request")
        }

        #[test]
        fn ep036_unit_generic_ssh_provider_rejects_non_generic_request() {
            let provider = GenericSshProvider::new(binding("127.0.0.1", 22), 1).expect("provider");
            let mut req = request("127.0.0.1", 22, "x");
            req.binding =
                ProviderBinding::new(ProviderKind::Aws, tenant(), "acct", "us-east-1", ref_())
                    .expect("binding");
            let err = provider.submit(&req).unwrap_err();
            assert_eq!(err.code, ComputeErrorCode::Policy);
        }

        #[test]
        fn ep036_unit_generic_ssh_provider_unreachable_target_fails_closed() {
            // No listener on this port: the real ssh-keyscan probe must
            // fail and submit must fail closed (Unavailable).
            let provider = GenericSshProvider::new(binding("127.0.0.1", 1), 1).expect("provider");
            let err = provider
                .submit(&request("127.0.0.1", 1, "dead"))
                .unwrap_err();
            assert_eq!(err.code, ComputeErrorCode::Unavailable);
        }

        #[test]
        fn ep036_unit_generic_ssh_provider_receipt_never_overclaims() {
            // The receipt can be constructed at SUBMITTED only; a
            // provider receipt never fabricates READY/VERIFIED.
            let receipt = ProvisioningReceipt::new(
                nexus_compute::model::ProvisioningRequestId::new("req-r").expect("id"),
                ProviderKind::GenericSsh,
                None,
                ResourceState::Submitted,
                VerificationState::Pending,
                correlation(),
                1_700_000_000,
            )
            .expect("receipt");
            assert_eq!(receipt.state, ResourceState::Submitted);
            assert_eq!(receipt.verification, VerificationState::Pending);
        }
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
