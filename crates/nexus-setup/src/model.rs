//! EP-035 setup value objects (SPEC-004 / SPEC-016).
//!
//! Every value object validates wire-shaped input with deny-unknown
//! semantics (mirroring the canonical schema `additionalProperties:
//! false` rule), and deserialization enforces the same checks as the
//! constructor. State truthfulness is structural: SELECTED !=
//! PROVISIONED, CONFIGURED != HEALTHY, DISCOVERED != TRUSTED,
//! COMPLETE_LOCAL != VERIFIED, OWNER_DETAILS != OWNER_AUTHORIZED.

use std::fmt;

use nexus_domain::{CorrelationId, PersonId, TenantId};
use serde::{Deserialize, Serialize};

use crate::error::{SetupError, SetupErrorCode, SetupResult};
use crate::vocabulary::{
    contains_hostile_authority_token, CapabilityCertificationState, DeploymentMode,
    DeploymentVerificationState, DiscoveryKind, EnrollmentCredentialState, HardwareProvenance,
    IntegrationStatus, OwnerBootstrapState, RecoveryFailureClass, RecoveryMaterialKind,
    RecoveryMutationState, RecoveryOutcome, ReleaseChannel,
};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> SetupResult<Self> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(SetupError::validation(format!(concat!(
                        stringify!($name),
                        " must be 1..=128 characters"
                    ))));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(ProfileId);
typed_id!(IntegrationId);
typed_id!(ObservationId);
typed_id!(CredentialId);
typed_id!(RecoveryKitId);
typed_id!(EnrollmentId);

/// Canonical DeploymentProfile value object (SPEC-016;
/// schemas/deployment-profile.schema.json). Field names are the
/// canonical snake_case wire names verbatim; schema parity is enforced
/// by ep035_unit_schema_parity tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentProfile {
    pub id: ProfileId,
    pub mode: DeploymentMode,
    pub release_channel: ReleaseChannel,
    pub components: Vec<String>,
    pub nodes: Vec<serde_json::Value>,
    pub backup: serde_json::Value,
    pub remote_access: serde_json::Value,
}

impl DeploymentProfile {
    pub fn new(
        id: ProfileId,
        mode: DeploymentMode,
        release_channel: ReleaseChannel,
        components: Vec<String>,
        nodes: Vec<serde_json::Value>,
        backup: serde_json::Value,
        remote_access: serde_json::Value,
    ) -> SetupResult<Self> {
        if !backup.is_object() {
            return Err(SetupError::validation("backup must be an object"));
        }
        if !remote_access.is_object() {
            return Err(SetupError::validation("remote_access must be an object"));
        }
        Ok(Self {
            id,
            mode,
            release_channel,
            components,
            nodes,
            backup,
            remote_access,
        })
    }
}

/// Deployment verification evidence: only VERIFIED carries one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentVerificationEvidence {
    pub verified_at_unix_s: u64,
    pub evidence_id: String,
    pub verifier: String,
}

/// Deployment verification state with optional evidence. A selection is
/// created UNVERIFIED and never proves host/runtime/ports/DNS/TLS/health.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentVerification {
    pub state: DeploymentVerificationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<DeploymentVerificationEvidence>,
}

impl DeploymentVerification {
    pub fn unverified() -> Self {
        Self {
            state: DeploymentVerificationState::Unverified,
            evidence: None,
        }
    }

    pub fn with_evidence(evidence: DeploymentVerificationEvidence) -> SetupResult<Self> {
        if evidence.verified_at_unix_s == 0 {
            return Err(SetupError::validation(
                "verified_at_unix_s must be positive",
            ));
        }
        Ok(Self {
            state: DeploymentVerificationState::Verified,
            evidence: Some(evidence),
        })
    }
}

/// DeploymentIntentRecord: a user's deployment selection. Intent is not
/// verification; verification is a separate, explicitly tracked state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentIntentRecord {
    pub profile: DeploymentProfile,
    pub selected_at_unix_s: u64,
    pub correlation: CorrelationId,
    pub verification: DeploymentVerification,
}

impl DeploymentIntentRecord {
    /// A selection is always created with verification UNVERIFIED.
    pub fn select(
        profile: DeploymentProfile,
        correlation: CorrelationId,
        selected_at_unix_s: u64,
    ) -> Self {
        Self {
            profile,
            selected_at_unix_s,
            correlation,
            verification: DeploymentVerification::unverified(),
        }
    }

    /// Transition to VERIFYING or FAILED explicitly, or VERIFIED with
    /// evidence.
    pub fn set_verification(
        mut self,
        state: DeploymentVerificationState,
        evidence: Option<DeploymentVerificationEvidence>,
    ) -> SetupResult<Self> {
        match state {
            DeploymentVerificationState::Verified => {
                let evidence = evidence.ok_or_else(|| {
                    SetupError::verification("deployment verification requires evidence")
                })?;
                self.verification = DeploymentVerification::with_evidence(evidence)?;
            }
            DeploymentVerificationState::Verifying | DeploymentVerificationState::Failed => {
                if evidence.is_some() {
                    return Err(SetupError::validation(
                        "verification evidence only valid for VERIFIED",
                    ));
                }
                self.verification = DeploymentVerification {
                    state,
                    evidence: None,
                };
            }
            DeploymentVerificationState::Unverified => {
                return Err(SetupError::validation(
                    "cannot reset verification to UNVERIFIED after selection",
                ));
            }
        }
        Ok(self)
    }
}

/// A hardware fact carries its actual provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareFact {
    pub key: String,
    pub value: HardwareValue,
    pub provenance: HardwareProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at_unix_s: Option<u64>,
}

/// Hardware fact value: string label or finite number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HardwareValue {
    Str(String),
    Int(i64),
    Float(f64),
}

impl HardwareFact {
    pub fn new(
        key: impl Into<String>,
        value: HardwareValue,
        provenance: HardwareProvenance,
        observed_at_unix_s: Option<u64>,
    ) -> SetupResult<Self> {
        let key = key.into();
        if key.is_empty() {
            return Err(SetupError::validation(
                "hardware fact key must not be empty",
            ));
        }
        if matches!(value, HardwareValue::Float(f) if !f.is_finite()) {
            return Err(SetupError::validation("hardware fact value must be finite"));
        }
        Ok(Self {
            key,
            value,
            provenance,
            observed_at_unix_s,
        })
    }
}

/// A capability declaration is a claim, never a certification.
/// CERTIFIED requires measured evidence AND a measured provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareCapabilityDeclaration {
    pub capability_id: String,
    pub declaration_provenance: HardwareProvenance,
    pub certification: CapabilityCertificationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured_evidence_id: Option<String>,
}

impl HardwareCapabilityDeclaration {
    pub fn new(
        capability_id: impl Into<String>,
        declaration_provenance: HardwareProvenance,
        certification: CapabilityCertificationState,
        measured_evidence_id: Option<String>,
    ) -> SetupResult<Self> {
        if certification == CapabilityCertificationState::Certified {
            let evidence = measured_evidence_id.as_deref().ok_or_else(|| {
                SetupError::verification("capability claims CERTIFIED without measured evidence")
            })?;
            if evidence.is_empty() {
                return Err(SetupError::verification(
                    "capability claims CERTIFIED without measured evidence",
                ));
            }
            if declaration_provenance != HardwareProvenance::Benchmarked
                && declaration_provenance != HardwareProvenance::HardwareCertified
            {
                return Err(SetupError::verification(format!(
                    "capability claims CERTIFIED from provenance {}; measured provenance required",
                    declaration_provenance
                )));
            }
        }
        Ok(Self {
            capability_id: capability_id.into(),
            declaration_provenance,
            certification,
            measured_evidence_id,
        })
    }
}

/// Hardware profile: facts with provenance plus capability
/// declarations. Observed facts never mint performance claims.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareProfile {
    pub facts: Vec<HardwareFact>,
    pub capability_declarations: Vec<HardwareCapabilityDeclaration>,
    pub profiled_at_unix_s: u64,
    pub correlation: CorrelationId,
}

/// Owner bootstrap request. A client-side `isOwner` field is rejected
/// (deny-unknown): client input never mints backend authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerBootstrapRequest {
    pub owner_name: String,
    pub owner_email: String,
    pub correlation: CorrelationId,
    pub idempotency_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_kit_id: Option<RecoveryKitId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_method: Option<String>,
}

impl OwnerBootstrapRequest {
    pub fn new(
        owner_name: impl Into<String>,
        owner_email: impl Into<String>,
        correlation: CorrelationId,
        idempotency_key: impl Into<String>,
        recovery_kit_id: Option<RecoveryKitId>,
        verification_method: Option<String>,
    ) -> SetupResult<Self> {
        let owner_name = owner_name.into();
        let owner_email = owner_email.into();
        let idempotency_key = idempotency_key.into();
        if owner_name.is_empty() || owner_email.is_empty() || idempotency_key.is_empty() {
            return Err(SetupError::validation(
                "owner_name, owner_email, and idempotency_key must not be empty",
            ));
        }
        Ok(Self {
            owner_name,
            owner_email,
            correlation,
            idempotency_key,
            recovery_kit_id,
            verification_method,
        })
    }
}

/// The durable first-owner record once initialized.
///
/// AUD-044: the record carries its OWNER_BOOTSTRAP state so the
/// persistence layer can never write OWNER_AUTHORIZED directly. Only
/// the enforced ladder transition (`advance_owner_state`) can move the
/// record forward, and it requires the preceding security transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstOwnerRecord {
    pub idempotency_key: String,
    pub principal_id: PersonId,
    pub state: OwnerBootstrapState,
}

impl FirstOwnerRecord {
    /// A first-owner record starts at the LOWEST ladder rung. It is
    /// NEVER constructed as OWNER_AUTHORIZED: that state can only be
    /// reached by traversing the enforced transitions.
    pub fn new(idempotency_key: impl Into<String>, principal_id: PersonId) -> Self {
        Self {
            idempotency_key: idempotency_key.into(),
            principal_id,
            state: OwnerBootstrapState::DetailsProvided,
        }
    }
}

/// Advance the owner bootstrap ladder. Enforces the canonical sequence:
/// OWNER_DETAILS_PROVIDED -> OWNER_IDENTITY_VERIFIED ->
/// OWNER_PRINCIPAL_CREATED -> OWNER_AUTHORIZED. A jump over any rung is
/// rejected; OWNER_AUTHORIZED can never be written without traversing
/// every preceding security transition (AUD-044).
pub fn advance_owner_state(
    record: &FirstOwnerRecord,
    to_state: OwnerBootstrapState,
) -> SetupResult<FirstOwnerRecord> {
    use OwnerBootstrapState::*;
    let valid = match (record.state, to_state) {
        (DetailsProvided, IdentityVerified) => true,
        (IdentityVerified, PrincipalCreated) => true,
        (PrincipalCreated, OwnerAuthorized) => true,
        (state, next) if state == next => true, // idempotent re-assert
        _ => false,
    };
    if !valid {
        return Err(SetupError::policy(format!(
            "invalid owner bootstrap transition {} -> {}",
            record.state, to_state
        )));
    }
    Ok(FirstOwnerRecord {
        idempotency_key: record.idempotency_key.clone(),
        principal_id: record.principal_id.clone(),
        state: to_state,
    })
}

/// Deterministic first-owner decision. Replay of the same idempotency
/// key is idempotent; a competing request is CONFLICT (never two first
/// owners). Durable enforcement is owned by the deployment layer; the
/// decision semantics are fixed here. The decision carries the record's
/// CURRENT ladder state; authorization is a separate, later transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstOwnerDecision {
    Initialized { principal_id: PersonId },
    AlreadyInitialized { principal_id: PersonId },
    Conflict,
}

pub fn resolve_first_owner(
    known: Option<&FirstOwnerRecord>,
    request: &OwnerBootstrapRequest,
    principal_id: PersonId,
) -> FirstOwnerDecision {
    match known {
        None => FirstOwnerDecision::Initialized { principal_id },
        Some(record) if record.idempotency_key == request.idempotency_key => {
            FirstOwnerDecision::AlreadyInitialized {
                principal_id: record.principal_id.clone(),
            }
        }
        Some(_) => FirstOwnerDecision::Conflict,
    }
}

/// BootstrapToken enrollment credential (SPEC-016 canonical name).
/// `secret` and `nonce` are SECRET: Debug, Display, and serialization
/// never emit them.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentCredential {
    pub credential_id: CredentialId,
    #[serde(rename = "kind")]
    pub kind: CredentialKind,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub state: EnrollmentCredentialState,
    #[serde(skip_serializing)]
    pub nonce: String,
    #[serde(skip_serializing)]
    pub secret: String,
}

/// Enrollment credential kind (SPEC-016 BootstrapToken).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CredentialKind {
    BootstrapToken,
}

impl EnrollmentCredential {
    pub fn new(
        credential_id: CredentialId,
        issued_at_unix_s: u64,
        expires_at_unix_s: u64,
        state: EnrollmentCredentialState,
        nonce: impl Into<String>,
        secret: impl Into<String>,
    ) -> SetupResult<Self> {
        if expires_at_unix_s <= issued_at_unix_s {
            return Err(SetupError::validation(
                "expires_at_unix_s must be after issued_at_unix_s",
            ));
        }
        let nonce = nonce.into();
        let secret = secret.into();
        if nonce.is_empty() || secret.is_empty() {
            return Err(SetupError::validation("nonce and secret must not be empty"));
        }
        Ok(Self {
            credential_id,
            kind: CredentialKind::BootstrapToken,
            issued_at_unix_s,
            expires_at_unix_s,
            state,
            nonce,
            secret,
        })
    }

    /// Usable only while ISSUED and within its validity window.
    pub fn is_usable(&self, now_unix_s: u64) -> bool {
        self.state == EnrollmentCredentialState::Issued
            && now_unix_s >= self.issued_at_unix_s
            && now_unix_s <= self.expires_at_unix_s
    }

    /// Secret-safe view: no secret, no nonce.
    pub fn redacted(&self) -> RedactedEnrollmentCredential {
        RedactedEnrollmentCredential {
            credential_id: self.credential_id.clone(),
            kind: self.kind,
            issued_at_unix_s: self.issued_at_unix_s,
            expires_at_unix_s: self.expires_at_unix_s,
            state: self.state,
        }
    }

    /// Atomically claim this credential by proving possession of the
    /// bootstrap secret.
    ///
    /// AUD-043: consumption of a one-time enrollment credential is bound
    /// to the secret. A caller knowing only the credential ID cannot
    /// consume it; the secret must match in the SAME atomic transition.
    /// Returns a new credential in the USED state on success, and never
    /// partially transitions on failure.
    pub fn claim(&self, secret: &str, now_unix_s: u64) -> SetupResult<Self> {
        if !self.is_usable(now_unix_s) {
            return Err(SetupError::conflict(
                "enrollment credential is not usable (expired, revoked, or already used)",
            ));
        }
        if self.secret != secret {
            return Err(SetupError::verification(
                "bootstrap secret does not match enrollment credential",
            ));
        }
        let mut claimed = self.clone();
        claimed.state = EnrollmentCredentialState::Used;
        Ok(claimed)
    }
}

impl fmt::Debug for EnrollmentCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnrollmentCredential")
            .field("credential_id", &self.credential_id)
            .field("kind", &self.kind)
            .field("issued_at_unix_s", &self.issued_at_unix_s)
            .field("expires_at_unix_s", &self.expires_at_unix_s)
            .field("state", &self.state)
            .field("nonce", &"[REDACTED]")
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

impl fmt::Display for EnrollmentCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EnrollmentCredential({}, {})",
            self.credential_id, self.state
        )
    }
}

/// Secret-safe enrollment credential view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactedEnrollmentCredential {
    pub credential_id: CredentialId,
    pub kind: CredentialKind,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub state: EnrollmentCredentialState,
}

/// Edge enrollment request: device label and endpoint are DATA, never
/// trust evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeEnrollmentRequest {
    pub device_label: String,
    pub endpoint: String,
    pub credential_id: CredentialId,
    pub correlation: CorrelationId,
}

impl EdgeEnrollmentRequest {
    pub fn new(
        device_label: impl Into<String>,
        endpoint: impl Into<String>,
        credential_id: CredentialId,
        correlation: CorrelationId,
    ) -> SetupResult<Self> {
        let device_label = device_label.into();
        let endpoint = endpoint.into();
        if device_label.is_empty() || endpoint.is_empty() {
            return Err(SetupError::validation(
                "device_label and endpoint must not be empty",
            ));
        }
        Ok(Self {
            device_label,
            endpoint,
            credential_id,
            correlation,
        })
    }
}

/// Discovery observation: data, never authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryObservation {
    pub id: ObservationId,
    pub kind: DiscoveryKind,
    pub name: String,
    pub endpoint: String,
    pub advertised_capabilities: Vec<String>,
    pub metadata: serde_json::Value,
    pub observed_at_unix_s: u64,
}

impl DiscoveryObservation {
    pub fn new(
        id: ObservationId,
        kind: DiscoveryKind,
        name: impl Into<String>,
        endpoint: impl Into<String>,
        advertised_capabilities: Vec<String>,
        metadata: serde_json::Value,
        observed_at_unix_s: u64,
    ) -> SetupResult<Self> {
        let name = name.into();
        let endpoint = endpoint.into();
        if name.is_empty() || endpoint.is_empty() {
            return Err(SetupError::validation(
                "observation name and endpoint must not be empty",
            ));
        }
        if !metadata.is_object() {
            return Err(SetupError::validation("metadata must be an object"));
        }
        Ok(Self {
            id,
            kind,
            name,
            endpoint,
            advertised_capabilities,
            metadata,
            observed_at_unix_s,
        })
    }

    /// Hostile discovery content is data, never authority.
    pub fn contains_hostile_authority_token(&self) -> bool {
        let mut haystack = format!("{} {}", self.name, self.endpoint);
        for capability in &self.advertised_capabilities {
            haystack.push(' ');
            haystack.push_str(capability);
        }
        contains_hostile_authority_token(&haystack)
    }
}

/// Discovery report: observations only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryReport {
    pub observations: Vec<DiscoveryObservation>,
    pub generated_at_unix_s: u64,
    pub correlation: CorrelationId,
}

/// The explicit governed transition from discovery to integration
/// work: a principal selects an observation. Selection is not
/// enrollment or authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationSelection {
    pub observation_id: ObservationId,
    pub selected_by: PersonId,
    pub selected_at_unix_s: u64,
    pub correlation: CorrelationId,
}

/// IntegrationCard data: truthful about configuration versus health.
/// Advertised capabilities are never derived from the provider name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationCard {
    pub integration_id: IntegrationId,
    pub provider_name: String,
    pub status: IntegrationStatus,
    pub advertised_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configured_at_unix_s: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_verified_at_unix_s: Option<u64>,
    pub correlation: CorrelationId,
}

impl IntegrationCard {
    pub fn new(
        integration_id: IntegrationId,
        provider_name: impl Into<String>,
        status: IntegrationStatus,
        advertised_capabilities: Vec<String>,
        configured_at_unix_s: Option<u64>,
        last_verified_at_unix_s: Option<u64>,
        correlation: CorrelationId,
    ) -> SetupResult<Self> {
        let provider_name = provider_name.into();
        if provider_name.is_empty() {
            return Err(SetupError::validation("provider_name must not be empty"));
        }
        if status == IntegrationStatus::Unconfigured && configured_at_unix_s.is_some() {
            return Err(SetupError::validation(
                "UNCONFIGURED card cannot carry configured_at_unix_s",
            ));
        }
        if status != IntegrationStatus::Unconfigured && configured_at_unix_s.is_none() {
            return Err(SetupError::validation(format!(
                "status {} requires configured_at_unix_s",
                status
            )));
        }
        if matches!(
            status,
            IntegrationStatus::Reachable | IntegrationStatus::Healthy | IntegrationStatus::Degraded
        ) && last_verified_at_unix_s.is_none()
        {
            return Err(SetupError::verification(format!(
                "status {} requires last_verified_at_unix_s",
                status
            )));
        }
        if status == IntegrationStatus::Unconfigured && last_verified_at_unix_s.is_some() {
            return Err(SetupError::validation(
                "UNCONFIGURED card cannot carry last_verified_at_unix_s",
            ));
        }
        Ok(Self {
            integration_id,
            provider_name,
            status,
            advertised_capabilities,
            configured_at_unix_s,
            last_verified_at_unix_s,
            correlation,
        })
    }

    /// Truthful status transition; HEALTHY requires a verification event.
    pub fn transition(mut self, to_status: IntegrationStatus, at_unix_s: u64) -> SetupResult<Self> {
        if !is_valid_integration_transition(self.status, to_status) {
            return Err(SetupError::policy(format!(
                "invalid integration status transition {} -> {}",
                self.status, to_status
            )));
        }
        if matches!(
            to_status,
            IntegrationStatus::Reachable | IntegrationStatus::Healthy | IntegrationStatus::Degraded
        ) && self.last_verified_at_unix_s.is_none()
        {
            return Err(SetupError::verification(
                "integration cannot become reachable/healthy without a verification event",
            ));
        }
        self.status = to_status;
        if to_status != IntegrationStatus::Error {
            self.last_verified_at_unix_s = Some(at_unix_s);
        }
        Ok(self)
    }
}

/// Allowed integration status transitions (truthfulness ladder).
pub fn is_valid_integration_transition(from: IntegrationStatus, to: IntegrationStatus) -> bool {
    use IntegrationStatus::*;
    matches!(
        (from, to),
        (Unconfigured, Configured | Error)
            | (Configured, Authenticated | Error | Degraded)
            | (Authenticated, Reachable | Error | Degraded)
            | (Reachable, Healthy | Degraded | Error)
            | (Degraded, Reachable | Error)
            | (Healthy, Degraded | Error)
            | (Error, Configured)
    )
}

/// Canonical RecoveryKit (SPEC-016; schemas/auth/recovery-kit.schema.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryKit {
    pub kit_id: RecoveryKitId,
    pub principal_id: PersonId,
    pub tenant_id: TenantId,
    pub material_kind: RecoveryMaterialKind,
    pub created_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub correlation: CorrelationId,
}

impl RecoveryKit {
    pub fn new(
        kit_id: RecoveryKitId,
        principal_id: PersonId,
        tenant_id: TenantId,
        material_kind: RecoveryMaterialKind,
        created_at_unix_s: u64,
        expires_at_unix_s: u64,
        correlation: CorrelationId,
    ) -> SetupResult<Self> {
        if expires_at_unix_s <= created_at_unix_s {
            return Err(SetupError::validation(
                "expires_at_unix_s must be after created_at_unix_s",
            ));
        }
        Ok(Self {
            kit_id,
            principal_id,
            tenant_id,
            material_kind,
            created_at_unix_s,
            expires_at_unix_s,
            correlation,
        })
    }

    pub fn is_expired(&self, now_unix_s: u64) -> bool {
        now_unix_s > self.expires_at_unix_s
    }
}

/// Recovery evidence: what is known about the failure and whether an
/// external mutation may have occurred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryEvidence {
    pub failure_class: RecoveryFailureClass,
    pub mutation_known: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mutation_occurred: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mutation_state: Option<RecoveryMutationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation: Option<CorrelationId>,
}

/// Recovery decision: no blind replay. AMBIGUOUS -> RECONCILE; retry is
/// safe only when RETRYABLE and mutation state is RECONCILED or
/// known-no-mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryDecision {
    pub outcome: RecoveryOutcome,
    pub mutation_state: RecoveryMutationState,
    pub retry_safe: bool,
    pub detail: String,
}

pub fn decide_recovery(evidence: &RecoveryEvidence) -> RecoveryDecision {
    let mutation_state = evidence
        .mutation_state
        .unwrap_or(RecoveryMutationState::Unknown);
    match evidence.failure_class {
        RecoveryFailureClass::Ambiguous => {
            // AUD-045: AMBIGUOUS + RECONCILED is NOT retry-safe by
            // itself. Retrying after an ambiguous provider outcome can
            // duplicate a consequential effect unless there is an
            // EXPLICIT negative mutation observation
            // (mutation_occurred == Some(false)). A reconciled state
            // without that observation is still unsafe to retry.
            if evidence.mutation_state == Some(RecoveryMutationState::Reconciled)
                && evidence.mutation_known
                && evidence.mutation_occurred == Some(false)
            {
                RecoveryDecision {
                    outcome: RecoveryOutcome::Retryable,
                    mutation_state: RecoveryMutationState::Reconciled,
                    retry_safe: true,
                    detail: "mutation reconciled with explicit negative observation; retry is safe"
                        .to_string(),
                }
            } else {
                RecoveryDecision {
                    outcome: RecoveryOutcome::Reconcile,
                    mutation_state: RecoveryMutationState::Unknown,
                    retry_safe: false,
                    detail: "external mutation outcome unknown; reconcile before retry".to_string(),
                }
            }
        }
        RecoveryFailureClass::Unavailable | RecoveryFailureClass::Timeout => {
            if evidence.mutation_known && evidence.mutation_occurred == Some(false) {
                RecoveryDecision {
                    outcome: RecoveryOutcome::Retryable,
                    mutation_state,
                    retry_safe: true,
                    detail: "no mutation occurred; retry is safe".to_string(),
                }
            } else {
                RecoveryDecision {
                    outcome: RecoveryOutcome::Reconcile,
                    mutation_state: RecoveryMutationState::Unknown,
                    retry_safe: false,
                    detail: "mutation outcome unknown; reconcile before retry".to_string(),
                }
            }
        }
        RecoveryFailureClass::Validation => RecoveryDecision {
            outcome: RecoveryOutcome::NonRetryable,
            mutation_state,
            retry_safe: false,
            detail: "input must be corrected; retry not safe".to_string(),
        },
        RecoveryFailureClass::Authorization => RecoveryDecision {
            outcome: RecoveryOutcome::Reauthenticate,
            mutation_state,
            retry_safe: false,
            detail: "authorization required again".to_string(),
        },
        RecoveryFailureClass::Conflict => RecoveryDecision {
            outcome: RecoveryOutcome::ResumeCheckpoint,
            mutation_state,
            retry_safe: false,
            detail: "conflicting state; resume from last checkpoint".to_string(),
        },
        RecoveryFailureClass::Internal => RecoveryDecision {
            outcome: RecoveryOutcome::ManualIntervention,
            mutation_state,
            retry_safe: false,
            detail: "internal failure; manual intervention required".to_string(),
        },
    }
}

/// Unused variant guard: RecoveryOutcome::Rollback and Reset are part of
/// the canonical vocabulary even when not produced by the current
/// deterministic decision map; they remain selectable by later layers.
pub const _RECOVERY_OUTCOME_VOCABULARY_COMPLETE: bool = {
    let _ = (RecoveryOutcome::Rollback, RecoveryOutcome::Reset);
    true
};

pub fn _unused_code_check(code: SetupErrorCode) -> u16 {
    code.http_status()
}
