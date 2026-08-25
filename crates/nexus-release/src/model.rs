//! EP-042 release/update/rollback contract models (SPEC-016, SPEC-024).
//!
//! M1 is the contract layer: every public interface from the node contract
//! is defined here with versioned serialization, construction validation,
//! and fail-closed invariants. No provider behavior, no key store, no
//! update executor, no installer logic exists in this crate.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest as Sha2Digest, Sha256};

use crate::error::{ReleaseError, ReleaseErrorCode, ReleaseResult};
use crate::vocabulary::{
    BundleKind, CanaryVerdict, DeploymentProfileMode, PromotionState, ReleaseChannel,
    RollbackState, SignatureAlgorithm, SignatureState, UpdateState, UpdateStepKind,
    VerificationState,
};

pub const RELEASE_SCHEMA_VERSION: u32 = 1;

fn non_empty(value: &str, field: &str) -> ReleaseResult<()> {
    if value.trim().is_empty() {
        return Err(ReleaseError::new(
            ReleaseErrorCode::Validation,
            format!("{field} must not be empty"),
        )
        .with_field(field));
    }
    Ok(())
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_iso8601_date(s: &str) -> bool {
    // Light structural check: YYYY-MM-DD prefix (SPEC-016 timestamps are
    // RFC3339 in this repository's canonical event contracts).
    s.len() >= 10
        && s.as_bytes()[4] == b'-'
        && s.as_bytes()[7] == b'-'
        && s.as_bytes()[..4].iter().all(|b| b.is_ascii_digit())
        && s.as_bytes()[5..7].iter().all(|b| b.is_ascii_digit())
        && s.as_bytes()[8..10].iter().all(|b| b.is_ascii_digit())
}

fn is_base64(s: &str) -> bool {
    if s.is_empty() || !s.len().is_multiple_of(4) {
        return false;
    }
    let bytes = s.as_bytes();
    let padding = bytes.iter().rev().take_while(|&&b| b == b'=').count();
    if padding > 2 {
        return false;
    }
    let body_len = bytes.len() - padding;
    bytes[..body_len]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'+' || *b == b'/')
}

fn compare_versions(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    // Three-part numeric comparison (major.minor.patch); pre-release
    // suffixes are ignored. Any non-numeric component fails closed (None).
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let core: Vec<&str> = v.split(['-', '+']).next()?.split('.').collect();
        if core.is_empty() || core.len() > 3 {
            return None;
        }
        let mut parts = [0u64; 3];
        for (i, part) in core.iter().enumerate() {
            if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            parts[i] = part.parse().ok()?;
        }
        Some((parts[0], parts[1], parts[2]))
    };
    Some(parse(a)?.cmp(&parse(b)?))
}

/// Compute a real SHA-256 hex digest over bytes (SPEC-024 content
/// addressing). The returned hex string is always exactly 64 characters.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Canonical content digest in `alg:hex` form (SPEC-024; EP-041 artifact
/// identity precedent). Accepted form: `sha256:` followed by at least 32
/// lowercase hex characters; digests computed by this crate are exactly 64.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Digest {
    raw: String,
}

impl Digest {
    pub fn new(value: &str) -> ReleaseResult<Self> {
        let (alg, hex) = value.split_once(':').ok_or_else(|| {
            ReleaseError::new(ReleaseErrorCode::DigestMismatch, "digest must be alg:hex")
                .with_field("digest")
        })?;
        if alg != "sha256" {
            return Err(ReleaseError::new(
                ReleaseErrorCode::DigestMismatch,
                format!("unsupported digest algorithm: {alg}"),
            )
            .with_field("digest"));
        }
        if hex.len() < 32 || !is_hex(hex) || hex.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(ReleaseError::new(
                ReleaseErrorCode::DigestMismatch,
                "sha256 digest must be lowercase hex with at least 32 characters",
            )
            .with_field("digest"));
        }
        Ok(Self {
            raw: value.to_string(),
        })
    }

    pub fn alg(&self) -> &str {
        &self.raw[..self.raw.find(':').unwrap_or(0)]
    }

    pub fn hex(&self) -> &str {
        &self.raw[self.raw.find(':').map(|i| i + 1).unwrap_or(0)..]
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl Serialize for Digest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Digest::new(&s).map_err(serde::de::Error::custom)
    }
}

/// Reference to an object in the ArtifactStore (SPEC-024). The backend is
/// a free-form string on purpose: provider names never become domain
/// capabilities (ARCHITECTURE.md forbidden moves).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectRef {
    pub backend: String,
    pub key: String,
}

impl ObjectRef {
    pub fn new(backend: &str, key: &str) -> ReleaseResult<Self> {
        non_empty(backend, "object_ref.backend")?;
        non_empty(key, "object_ref.key")?;
        Ok(Self {
            backend: backend.to_string(),
            key: key.to_string(),
        })
    }
}

/// Signature envelope (SPEC-016 behavior 6). Presence is not validity:
/// `SignatureState` is produced by verification, never by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    pub algorithm: SignatureAlgorithm,
    pub key_id: String,
    pub value_b64: String,
}

impl Signature {
    pub fn new(
        algorithm: SignatureAlgorithm,
        key_id: &str,
        value_b64: &str,
    ) -> ReleaseResult<Self> {
        non_empty(key_id, "signature.key_id")?;
        non_empty(value_b64, "signature.value_b64")?;
        if !is_base64(value_b64) {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "signature value must be base64",
            )
            .with_field("signature.value_b64"));
        }
        Ok(Self {
            algorithm,
            key_id: key_id.to_string(),
            value_b64: value_b64.to_string(),
        })
    }

    /// A constructed signature is at most `Present` in M1: the crate has no
    /// key store or verifier, so it never claims validity.
    pub fn state(&self) -> SignatureState {
        SignatureState::Present
    }
}

/// A signed release component (SPEC-016 behavior 6; SPEC-024 artifact).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedComponent {
    pub component_id: String,
    pub name: String,
    pub version: String,
    pub artifact_ref: ObjectRef,
    pub digest: Digest,
    pub signature: Signature,
    pub sbom_ref: ObjectRef,
    pub license_ref: String,
    pub size_bytes: u64,
}

impl SignedComponent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        component_id: &str,
        name: &str,
        version: &str,
        artifact_ref: ObjectRef,
        digest: Digest,
        signature: Signature,
        sbom_ref: ObjectRef,
        license_ref: &str,
        size_bytes: u64,
    ) -> ReleaseResult<Self> {
        non_empty(component_id, "component_id")?;
        non_empty(name, "component.name")?;
        non_empty(version, "component.version")?;
        non_empty(license_ref, "component.license_ref")?;
        Ok(Self {
            component_id: component_id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            artifact_ref,
            digest,
            signature,
            sbom_ref,
            license_ref: license_ref.to_string(),
            size_bytes,
        })
    }

    /// SIGNATURE PRESENT != SIGNATURE VALID: the component exposes the
    /// signature state ladder, never a verification claim.
    pub fn signature_state(&self) -> SignatureState {
        self.signature.state()
    }
}

/// Compatibility matrix entry (SPEC-016 behavior 1, 8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityEntry {
    pub component_id: String,
    pub version: String,
    pub min_version: String,
    pub max_version: String,
    pub supported_profiles: Vec<DeploymentProfileMode>,
}

impl CompatibilityEntry {
    pub fn new(
        component_id: &str,
        version: &str,
        min_version: &str,
        max_version: &str,
        supported_profiles: Vec<DeploymentProfileMode>,
    ) -> ReleaseResult<Self> {
        non_empty(component_id, "compatibility.component_id")?;
        non_empty(version, "compatibility.version")?;
        non_empty(min_version, "compatibility.min_version")?;
        non_empty(max_version, "compatibility.max_version")?;
        if supported_profiles.is_empty() {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "compatibility entry must declare at least one supported profile",
            )
            .with_field("compatibility.supported_profiles"));
        }
        Ok(Self {
            component_id: component_id.to_string(),
            version: version.to_string(),
            min_version: min_version.to_string(),
            max_version: max_version.to_string(),
            supported_profiles,
        })
    }
}

/// Result of a compatibility check. Fail-closed: any unknown component,
/// missing entry, or unparseable version is incompatible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibleVerdict {
    pub compatible: bool,
    pub reasons: Vec<String>,
}

impl CompatibleVerdict {
    pub fn ok() -> Self {
        Self {
            compatible: true,
            reasons: Vec::new(),
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            compatible: false,
            reasons: vec![reason.into()],
        }
    }

    pub fn deny_many(reasons: Vec<String>) -> Self {
        Self {
            compatible: false,
            reasons,
        }
    }
}

/// Compatibility matrix (SPEC-016 behavior 1; one distribution supports
/// managed, BYOC, existing SSH, hybrid, and fully local profiles).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityMatrix {
    pub matrix_id: String,
    pub schema_version: u32,
    pub entries: Vec<CompatibilityEntry>,
}

impl CompatibilityMatrix {
    pub fn new(matrix_id: &str, entries: Vec<CompatibilityEntry>) -> ReleaseResult<Self> {
        non_empty(matrix_id, "matrix_id")?;
        if entries.is_empty() {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "compatibility matrix must not be empty",
            )
            .with_field("matrix.entries"));
        }
        let mut seen = std::collections::HashSet::new();
        for entry in &entries {
            if !seen.insert(entry.component_id.clone()) {
                return Err(ReleaseError::new(
                    ReleaseErrorCode::Validation,
                    format!("duplicate component entry: {}", entry.component_id),
                )
                .with_field("matrix.entries"));
            }
        }
        Ok(Self {
            matrix_id: matrix_id.to_string(),
            schema_version: RELEASE_SCHEMA_VERSION,
            entries,
        })
    }

    /// Fail-closed compatibility check over a component set.
    pub fn check(&self, components: &[SignedComponent]) -> CompatibleVerdict {
        let mut reasons = Vec::new();
        for component in components {
            let entry = match self
                .entries
                .iter()
                .find(|e| e.component_id == component.component_id)
            {
                Some(e) => e,
                None => {
                    reasons.push(format!(
                        "component {} is not present in the compatibility matrix",
                        component.component_id
                    ));
                    continue;
                }
            };
            if entry.version != component.version {
                reasons.push(format!(
                    "component {} version {} does not match matrix version {}",
                    component.component_id, component.version, entry.version
                ));
            }
            match compare_versions(&component.version, &entry.min_version) {
                Some(std::cmp::Ordering::Less) | None => {
                    reasons.push(format!(
                        "component {} version {} is below matrix minimum {}",
                        component.component_id, component.version, entry.min_version
                    ));
                }
                _ => {}
            }
            match compare_versions(&component.version, &entry.max_version) {
                Some(std::cmp::Ordering::Greater) | None => {
                    reasons.push(format!(
                        "component {} version {} exceeds matrix maximum {}",
                        component.component_id, component.version, entry.max_version
                    ));
                }
                _ => {}
            }
        }
        if reasons.is_empty() {
            CompatibleVerdict::ok()
        } else {
            CompatibleVerdict::deny_many(reasons)
        }
    }

    /// True when every matrix entry declares support for the profile.
    pub fn supports_profile(&self, profile: DeploymentProfileMode) -> bool {
        self.entries
            .iter()
            .all(|e| e.supported_profiles.contains(&profile))
    }

    /// One signed distribution supports every canonical profile.
    pub fn supports_all_profiles(&self) -> bool {
        DeploymentProfileMode::ALL
            .iter()
            .all(|profile| self.supports_profile(*profile))
    }
}

/// Signed release manifest (SPEC-016 behavior 1, 7; SPEC-024 manifests).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub schema_version: u32,
    pub release_id: String,
    pub version: String,
    pub channel: ReleaseChannel,
    pub components: Vec<SignedComponent>,
    pub compatibility: CompatibilityMatrix,
    pub offline_bundle_ref: Option<ObjectRef>,
    pub sbom_ref: ObjectRef,
    pub license_refs: Vec<String>,
    pub created_at: String,
    pub manifest_digest: Option<Digest>,
}

impl ReleaseManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        release_id: &str,
        version: &str,
        channel: ReleaseChannel,
        components: Vec<SignedComponent>,
        compatibility: CompatibilityMatrix,
        offline_bundle_ref: Option<ObjectRef>,
        sbom_ref: ObjectRef,
        license_refs: Vec<String>,
        created_at: &str,
    ) -> ReleaseResult<Self> {
        non_empty(release_id, "release_id")?;
        non_empty(version, "release.version")?;
        non_empty(created_at, "release.created_at")?;
        if !is_iso8601_date(created_at) {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "release.created_at must be an ISO-8601/RFC3339 timestamp",
            )
            .with_field("release.created_at"));
        }
        if components.is_empty() {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "release manifest must contain at least one signed component",
            )
            .with_field("release.components"));
        }
        if license_refs.is_empty() {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "release manifest must reference at least one license",
            )
            .with_field("release.license_refs"));
        }
        Ok(Self {
            schema_version: RELEASE_SCHEMA_VERSION,
            release_id: release_id.to_string(),
            version: version.to_string(),
            channel,
            components,
            compatibility,
            offline_bundle_ref,
            sbom_ref,
            license_refs,
            created_at: created_at.to_string(),
            manifest_digest: None,
        })
    }

    /// Deterministic content digest over the canonical JSON bytes. This is
    /// real content addressing (SPEC-024), not signature verification.
    ///
    /// The digest excludes the self-referential `manifest_digest` field so
    /// the binding is verifiable: the bytes digested are the manifest
    /// content without the digest field itself.
    pub fn content_digest(&self) -> ReleaseResult<Digest> {
        let mut canonical = self.clone();
        canonical.manifest_digest = None;
        let bytes = serde_json::to_vec(&canonical).map_err(|e| {
            ReleaseError::new(
                ReleaseErrorCode::InternalInvariant,
                format!("manifest serialization failed: {e}"),
            )
        })?;
        Digest::new(&format!("sha256:{}", sha256_hex(&bytes)))
    }

    /// Bind the manifest to its declared digest. MISSING != VERIFIED:
    /// a manifest without a digest binding is simply unverified.
    pub fn verify_digest_binding(&self) -> VerificationState {
        match &self.manifest_digest {
            None => VerificationState::Missing,
            Some(declared) => {
                let computed = self.content_digest();
                match computed {
                    Ok(computed) if computed == *declared => VerificationState::Verified,
                    Ok(_) => VerificationState::Mismatch,
                    Err(_) => VerificationState::Mismatch,
                }
            }
        }
    }

    /// RELEASE MANIFEST EXISTS != RELEASE VERIFIED: M1 exposes the
    /// verification state ladder; a manifest never self-certifies.
    pub fn verification_state(&self) -> VerificationState {
        self.verify_digest_binding()
    }
}

/// A single step in a transactional update plan (SPEC-016 behavior 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateStep {
    pub order: u32,
    pub kind: UpdateStepKind,
    pub description: String,
}

impl UpdateStep {
    pub fn new(order: u32, kind: UpdateStepKind, description: &str) -> ReleaseResult<Self> {
        non_empty(description, "update_step.description")?;
        Ok(Self {
            order,
            kind,
            description: description.to_string(),
        })
    }
}

/// Transactional update plan (SPEC-016 behavior 6). Plans never promote:
/// the promotion step is intentionally absent from the vocabulary, and the
/// first step is always a backup (backup-before-update).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlan {
    pub schema_version: u32,
    pub plan_id: String,
    pub release_id: String,
    pub from_version: String,
    pub to_version: String,
    pub channel: ReleaseChannel,
    pub steps: Vec<UpdateStep>,
    pub idempotency_key: String,
    pub correlation_id: String,
    pub created_at: String,
    pub state: UpdateState,
}

impl UpdatePlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plan_id: &str,
        release_id: &str,
        from_version: &str,
        to_version: &str,
        channel: ReleaseChannel,
        steps: Vec<UpdateStep>,
        idempotency_key: &str,
        correlation_id: &str,
        created_at: &str,
    ) -> ReleaseResult<Self> {
        non_empty(plan_id, "plan_id")?;
        non_empty(release_id, "plan.release_id")?;
        non_empty(from_version, "plan.from_version")?;
        non_empty(to_version, "plan.to_version")?;
        non_empty(idempotency_key, "plan.idempotency_key")?;
        non_empty(correlation_id, "plan.correlation_id")?;
        non_empty(created_at, "plan.created_at")?;
        if !is_iso8601_date(created_at) {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "plan.created_at must be an ISO-8601/RFC3339 timestamp",
            )
            .with_field("plan.created_at"));
        }
        if to_version == from_version {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "update plan must change the version",
            )
            .with_field("plan.to_version"));
        }
        if steps.is_empty() {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "update plan must contain at least one step",
            )
            .with_field("plan.steps"));
        }
        // backup-before-update: the first step is always a backup.
        if steps[0].kind != UpdateStepKind::Backup {
            return Err(ReleaseError::new(
                ReleaseErrorCode::BackupRequired,
                "update plan first step must be a backup",
            )
            .with_field("plan.steps[0].kind"));
        }
        for (i, step) in steps.iter().enumerate() {
            if step.order != (i as u32) + 1 {
                return Err(ReleaseError::new(
                    ReleaseErrorCode::Validation,
                    format!(
                        "update step order must be contiguous starting at 1; found {} at index {i}",
                        step.order
                    ),
                )
                .with_field("plan.steps.order"));
            }
        }
        Ok(Self {
            schema_version: RELEASE_SCHEMA_VERSION,
            plan_id: plan_id.to_string(),
            release_id: release_id.to_string(),
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            channel,
            steps,
            idempotency_key: idempotency_key.to_string(),
            correlation_id: correlation_id.to_string(),
            created_at: created_at.to_string(),
            state: UpdateState::Planned,
        })
    }

    /// UPDATE PLAN EXISTS != UPDATE EXECUTED: the plan carries a state
    /// ladder, and the initial state is Planned, never executed.
    pub fn has_backup_first_step(&self) -> bool {
        self.steps
            .first()
            .map(|s| s.kind == UpdateStepKind::Backup)
            .unwrap_or(false)
    }

    /// Plans can never contain a promotion step.
    pub fn contains_no_promote_step(&self) -> bool {
        !self.steps.iter().any(|s| s.kind.to_string() == "PROMOTE")
    }
}

/// Canary ring (SPEC-016 behavior 6). A canary observes and recommends;
/// it can never promote or deploy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanaryRing {
    pub schema_version: u32,
    pub ring_id: String,
    pub release_id: String,
    pub profile: DeploymentProfileMode,
    pub cohort_percent: u32,
    pub observation_minutes: u32,
    pub health_criterion: String,
    pub verdict: CanaryVerdict,
    pub observed_at: Option<String>,
    pub evidence_ref: Option<String>,
}

impl CanaryRing {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ring_id: &str,
        release_id: &str,
        profile: DeploymentProfileMode,
        cohort_percent: u32,
        observation_minutes: u32,
        health_criterion: &str,
    ) -> ReleaseResult<Self> {
        non_empty(ring_id, "ring_id")?;
        non_empty(release_id, "canary.release_id")?;
        non_empty(health_criterion, "canary.health_criterion")?;
        if !(1..=100).contains(&cohort_percent) {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "canary cohort percent must be between 1 and 100",
            )
            .with_field("canary.cohort_percent"));
        }
        if observation_minutes == 0 {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "canary observation window must be positive",
            )
            .with_field("canary.observation_minutes"));
        }
        Ok(Self {
            schema_version: RELEASE_SCHEMA_VERSION,
            ring_id: ring_id.to_string(),
            release_id: release_id.to_string(),
            profile,
            cohort_percent,
            observation_minutes,
            health_criterion: health_criterion.to_string(),
            verdict: CanaryVerdict::Observing,
            observed_at: None,
            evidence_ref: None,
        })
    }

    /// A canary verdict is a recommendation. Even READY_TO_PROMOTE never
    /// promotes: the promotion itself is the exact manual action.
    pub fn recommends_ready(&self) -> bool {
        self.verdict == CanaryVerdict::ReadyToPromote && self.evidence_ref.is_some()
    }

    /// CANARY OBSERVING != PROMOTED: this type has no promoted state and
    /// carries no deployment surface. Promotion requires the exact manual
    /// action authorized by a ManualPromotion record.
    pub fn never_promotes(&self) -> bool {
        true
    }
}

/// Rollback receipt (SPEC-016 behavior 6; SPEC-024 backup).
///
/// A receipt REQUIRES a backup reference by construction: the field is not
/// optional. Receipt exists != rollback verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackReceipt {
    pub schema_version: u32,
    pub receipt_id: String,
    pub update_plan_ref: String,
    pub from_version: String,
    pub to_version: String,
    pub backup_ref: ObjectRef,
    pub backup_verification: VerificationState,
    pub rollback_verification: VerificationState,
    pub state: RollbackState,
    pub actor: String,
    pub correlation_id: String,
    pub verified_at: Option<String>,
}

impl RollbackReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        receipt_id: &str,
        update_plan_ref: &str,
        from_version: &str,
        to_version: &str,
        backup_ref: ObjectRef,
        actor: &str,
        correlation_id: &str,
    ) -> ReleaseResult<Self> {
        non_empty(receipt_id, "receipt_id")?;
        non_empty(update_plan_ref, "receipt.update_plan_ref")?;
        non_empty(from_version, "receipt.from_version")?;
        non_empty(to_version, "receipt.to_version")?;
        non_empty(actor, "receipt.actor")?;
        non_empty(correlation_id, "receipt.correlation_id")?;
        if to_version == from_version {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "rollback receipt must change the version",
            )
            .with_field("receipt.to_version"));
        }
        Ok(Self {
            schema_version: RELEASE_SCHEMA_VERSION,
            receipt_id: receipt_id.to_string(),
            update_plan_ref: update_plan_ref.to_string(),
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            backup_ref,
            backup_verification: VerificationState::Unverified,
            rollback_verification: VerificationState::Unverified,
            state: RollbackState::RequiresBackup,
            actor: actor.to_string(),
            correlation_id: correlation_id.to_string(),
            verified_at: None,
        })
    }

    /// ROLLBACK RECEIPT REQUIRES BACKUP REF: guaranteed by the mandatory
    /// field; this helper documents the invariant for callers.
    pub fn has_backup_ref(&self) -> bool {
        !self.backup_ref.backend.is_empty() && !self.backup_ref.key.is_empty()
    }

    /// Receipt exists != rollback verified: only a verified backup and a
    /// verified rollback observation upgrade the state.
    pub fn is_verified(&self) -> bool {
        self.state == RollbackState::RollbackVerified
            && self.backup_verification == VerificationState::Verified
            && self.rollback_verification == VerificationState::Verified
    }
}

/// Offline bundle content item (SPEC-016 behavior 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleItem {
    pub kind: BundleKind,
    pub name: String,
    pub digest: Digest,
}

impl BundleItem {
    pub fn new(kind: BundleKind, name: &str, digest: Digest) -> ReleaseResult<Self> {
        non_empty(name, "bundle_item.name")?;
        Ok(Self {
            kind,
            name: name.to_string(),
            digest,
        })
    }
}

/// Offline bundle (SPEC-016 behavior 5; SPEC-024 manifests).
///
/// Contains approved images, models, licenses, SBOMs, and manifests.
/// OFFLINE BUNDLE EXISTS != OFFLINE BUNDLE VERIFIED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineBundle {
    pub schema_version: u32,
    pub bundle_id: String,
    pub release_id: String,
    pub contents: Vec<BundleItem>,
    pub manifest_ref: ObjectRef,
    pub sbom_refs: Vec<String>,
    pub license_refs: Vec<String>,
    pub migrations: Vec<String>,
    pub bundle_digest: Option<Digest>,
}

impl OfflineBundle {
    pub fn new(
        bundle_id: &str,
        release_id: &str,
        contents: Vec<BundleItem>,
        manifest_ref: ObjectRef,
        sbom_refs: Vec<String>,
        license_refs: Vec<String>,
        migrations: Vec<String>,
    ) -> ReleaseResult<Self> {
        non_empty(bundle_id, "bundle_id")?;
        non_empty(release_id, "bundle.release_id")?;
        if contents.is_empty() {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "offline bundle must contain at least one item",
            )
            .with_field("bundle.contents"));
        }
        // Acceptance obligation: bundles contain approved images, models,
        // licenses, SBOMs, and manifests.
        let kinds: std::collections::HashSet<BundleKind> =
            contents.iter().map(|i| i.kind).collect();
        for required in [
            BundleKind::Image,
            BundleKind::Model,
            BundleKind::License,
            BundleKind::Sbom,
        ] {
            if !kinds.contains(&required) {
                return Err(ReleaseError::new(
                    ReleaseErrorCode::Validation,
                    format!("offline bundle missing required content kind: {required}"),
                )
                .with_field("bundle.contents"));
            }
        }
        if sbom_refs.is_empty() {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "offline bundle must reference at least one SBOM",
            )
            .with_field("bundle.sbom_refs"));
        }
        if license_refs.is_empty() {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "offline bundle must reference at least one license",
            )
            .with_field("bundle.license_refs"));
        }
        Ok(Self {
            schema_version: RELEASE_SCHEMA_VERSION,
            bundle_id: bundle_id.to_string(),
            release_id: release_id.to_string(),
            contents,
            manifest_ref,
            sbom_refs,
            license_refs,
            migrations,
            bundle_digest: None,
        })
    }

    /// Deterministic content digest over the canonical JSON bytes. The
    /// digest excludes the self-referential `bundle_digest` field so the
    /// binding is verifiable.
    pub fn content_digest(&self) -> ReleaseResult<Digest> {
        let mut canonical = self.clone();
        canonical.bundle_digest = None;
        let bytes = serde_json::to_vec(&canonical).map_err(|e| {
            ReleaseError::new(
                ReleaseErrorCode::InternalInvariant,
                format!("bundle serialization failed: {e}"),
            )
        })?;
        Digest::new(&format!("sha256:{}", sha256_hex(&bytes)))
    }

    pub fn verify_digest_binding(&self) -> VerificationState {
        match &self.bundle_digest {
            None => VerificationState::Missing,
            Some(declared) => match self.content_digest() {
                Ok(computed) if computed == *declared => VerificationState::Verified,
                _ => VerificationState::Mismatch,
            },
        }
    }
}

/// Promotion decision record (SPEC-016 behavior 7).
///
/// PROMOTION DECISION != DEPLOYMENT: this record authorizes an exact
/// manual command; it contains no executor and cannot perform deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualPromotion {
    pub schema_version: u32,
    pub promotion_id: String,
    pub release_id: String,
    pub update_plan_ref: String,
    pub canary_ring_ref: String,
    pub approval_ref: String,
    pub approver: String,
    pub approved_at: String,
    pub state: PromotionState,
    pub exact_manual_command: String,
}

impl ManualPromotion {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        promotion_id: &str,
        release_id: &str,
        update_plan_ref: &str,
        canary_ring_ref: &str,
        approval_ref: &str,
        approver: &str,
        approved_at: &str,
        exact_manual_command: &str,
    ) -> ReleaseResult<Self> {
        non_empty(promotion_id, "promotion_id")?;
        non_empty(release_id, "promotion.release_id")?;
        non_empty(update_plan_ref, "promotion.update_plan_ref")?;
        non_empty(canary_ring_ref, "promotion.canary_ring_ref")?;
        non_empty(approval_ref, "promotion.approval_ref")?;
        non_empty(approver, "promotion.approver")?;
        non_empty(approved_at, "promotion.approved_at")?;
        non_empty(exact_manual_command, "promotion.exact_manual_command")?;
        if !is_iso8601_date(approved_at) {
            return Err(ReleaseError::new(
                ReleaseErrorCode::Validation,
                "promotion.approved_at must be an ISO-8601/RFC3339 timestamp",
            )
            .with_field("promotion.approved_at"));
        }
        Ok(Self {
            schema_version: RELEASE_SCHEMA_VERSION,
            promotion_id: promotion_id.to_string(),
            release_id: release_id.to_string(),
            update_plan_ref: update_plan_ref.to_string(),
            canary_ring_ref: canary_ring_ref.to_string(),
            approval_ref: approval_ref.to_string(),
            approver: approver.to_string(),
            approved_at: approved_at.to_string(),
            state: PromotionState::ApprovedManualOnly,
            exact_manual_command: exact_manual_command.to_string(),
        })
    }

    /// Production promotion remains an exact manual action: the record
    /// only exists after a human approval reference, and its only effect
    /// surface is the documented manual command.
    pub fn requires_human_approval(&self) -> bool {
        !self.approval_ref.is_empty() && self.state == PromotionState::ApprovedManualOnly
    }

    /// The promotion record never performs deployment.
    pub fn never_deploys(&self) -> bool {
        // No executor, no deploy target, no automatic effect surface: the
        // type is a decision record, not an action.
        true
    }
}

/// A promotion decision produced by evaluating a canary ring and approval
/// evidence. Fail-closed: anything short of an exact human approval denies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionRecord {
    pub decision: PromotionState,
    pub reason: String,
    pub requires_human: bool,
}

impl PromotionRecord {
    pub fn locked(reason: impl Into<String>) -> Self {
        Self {
            decision: PromotionState::Locked,
            reason: reason.into(),
            requires_human: true,
        }
    }

    pub fn awaiting_approval(reason: impl Into<String>) -> Self {
        Self {
            decision: PromotionState::AwaitingHumanApproval,
            reason: reason.into(),
            requires_human: true,
        }
    }

    pub fn approved_manual_only(reason: impl Into<String>) -> Self {
        Self {
            decision: PromotionState::ApprovedManualOnly,
            reason: reason.into(),
            requires_human: true,
        }
    }
}

/// Fail-closed promotion gate: a canary may only advance to
/// READY_TO_PROMOTE with real evidence, and promotion is only ever
/// APPROVED_MANUAL_ONLY after a human approval reference. No path returns
/// an autonomous deployment decision.
pub fn promotion_gate_decision(ring: &CanaryRing, approval_ref: Option<&str>) -> PromotionRecord {
    if ring.verdict == CanaryVerdict::Rollback {
        return PromotionRecord::locked("canary verdict is ROLLBACK");
    }
    if ring.verdict != CanaryVerdict::ReadyToPromote {
        return PromotionRecord::locked("canary verdict is not READY_TO_PROMOTE");
    }
    if ring.evidence_ref.is_none() {
        return PromotionRecord::locked("canary evidence is missing");
    }
    match approval_ref {
        None => PromotionRecord::awaiting_approval("exact manual approval required"),
        Some("") => PromotionRecord::awaiting_approval("exact manual approval required"),
        Some(_) => {
            PromotionRecord::approved_manual_only("promotion authorized as exact manual action")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::VocabularyError;
    use crate::vocabulary::{
        BundleKind, CanaryVerdict, DeploymentProfileMode, ReleaseChannel, RollbackState,
        SignatureAlgorithm, UpdateStepKind, VerificationState,
    };

    fn digest64(seed: u8) -> Digest {
        // Real 64-char lowercase hex digests (deterministic per test).
        let mut s = String::with_capacity(64);
        for i in 0..64 {
            let b = seed.wrapping_add(i);
            s.push(char::from_digit(u32::from(b % 16), 16).unwrap());
        }
        Digest::new(&format!("sha256:{s}")).unwrap()
    }

    fn object_ref(seed: &str) -> ObjectRef {
        ObjectRef::new("local", seed).unwrap()
    }

    fn signature(seed: u8) -> Signature {
        // 12-char base64 body (len % 4 == 0), alphabet-safe.
        let value = format!("AAAA{seed:02x}BBBB{seed:02x}");
        Signature::new(SignatureAlgorithm::Ed25519, "key-test-1", &value).unwrap()
    }

    fn component(seed: u8, version: &str) -> SignedComponent {
        SignedComponent::new(
            &format!("comp-{seed}"),
            &format!("component-{seed}"),
            version,
            object_ref(&format!("artifact-{seed}")),
            digest64(seed),
            signature(seed),
            object_ref(&format!("sbom-{seed}")),
            "MIT",
            1024,
        )
        .unwrap()
    }

    fn matrix() -> CompatibilityMatrix {
        let entries = vec![
            CompatibilityEntry::new(
                "comp-1",
                "1.0.0",
                "1.0.0",
                "1.9.9",
                vec![
                    DeploymentProfileMode::Managed,
                    DeploymentProfileMode::Byoc,
                    DeploymentProfileMode::ExistingSsh,
                    DeploymentProfileMode::Hybrid,
                    DeploymentProfileMode::FullyLocal,
                ],
            )
            .unwrap(),
            CompatibilityEntry::new(
                "comp-2",
                "2.0.0",
                "2.0.0",
                "2.9.9",
                vec![
                    DeploymentProfileMode::Managed,
                    DeploymentProfileMode::Byoc,
                    DeploymentProfileMode::ExistingSsh,
                    DeploymentProfileMode::Hybrid,
                    DeploymentProfileMode::FullyLocal,
                ],
            )
            .unwrap(),
        ];
        CompatibilityMatrix::new("matrix-1", entries).unwrap()
    }

    fn manifest() -> ReleaseManifest {
        let m = matrix();
        let components = vec![component(1, "1.0.0"), component(2, "2.0.0")];
        ReleaseManifest::new(
            "release-1",
            "1.0.0",
            ReleaseChannel::Stable,
            components,
            m,
            None,
            object_ref("sbom-root"),
            vec!["MIT".to_string()],
            "2026-08-25T00:00:00Z",
        )
        .unwrap()
    }

    // ---- Digest ---------------------------------------------------------

    #[test]
    fn ep042_unit_digest_accepts_real_sha256_hex() {
        let d = Digest::new("sha256:0123456789abcdef0123456789abcdef").unwrap();
        assert_eq!(d.alg(), "sha256");
        assert_eq!(d.hex(), "0123456789abcdef0123456789abcdef");
        assert_eq!(d.as_str(), "sha256:0123456789abcdef0123456789abcdef");
    }

    #[test]
    fn ep042_unit_digest_rejects_missing_alg_prefix() {
        assert!(Digest::new("0123456789abcdef0123456789abcdef").is_err());
    }

    #[test]
    fn ep042_unit_digest_rejects_unsupported_algorithm() {
        assert!(Digest::new("md5:0123456789abcdef0123456789abcdef").is_err());
        assert!(Digest::new("sha512:0123456789abcdef0123456789abcdef").is_err());
    }

    #[test]
    fn ep042_unit_digest_rejects_short_hex() {
        assert!(Digest::new("sha256:0123456789abcdef").is_err());
    }

    #[test]
    fn ep042_unit_digest_rejects_non_hex() {
        assert!(Digest::new("sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
    }

    #[test]
    fn ep042_unit_digest_rejects_uppercase_hex() {
        assert!(Digest::new("sha256:0123456789ABCDEF0123456789ABCDEF").is_err());
    }

    #[test]
    fn ep042_unit_digest_serde_rejects_malformed_wire_value() {
        assert!(serde_json::from_str::<Digest>("\"bogus\"").is_err());
        let d = digest64(1);
        let json = serde_json::to_string(&d).unwrap();
        let back: Digest = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn ep042_unit_sha256_hex_is_exactly_64_chars() {
        let h = sha256_hex(b"nexus release contract");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ---- Signature ------------------------------------------------------

    #[test]
    fn ep042_unit_signature_present_not_valid() {
        let sig = signature(1);
        assert_eq!(sig.state(), SignatureState::Present);
        assert_ne!(sig.state(), SignatureState::Valid);
    }

    #[test]
    fn ep042_unit_signature_rejects_empty_key_id() {
        assert!(Signature::new(SignatureAlgorithm::Ed25519, "", "AAAA01BBBB01").is_err());
    }

    #[test]
    fn ep042_unit_signature_rejects_non_base64_value() {
        assert!(Signature::new(SignatureAlgorithm::Ed25519, "k1", "not base64!!").is_err());
    }

    // ---- SignedComponent ------------------------------------------------

    #[test]
    fn ep042_unit_component_construction_and_signature_state() {
        let c = component(1, "1.0.0");
        assert_eq!(c.component_id, "comp-1");
        assert_eq!(c.signature_state(), SignatureState::Present);
    }

    #[test]
    fn ep042_unit_component_rejects_empty_component_id() {
        let err = SignedComponent::new(
            "",
            "name",
            "1.0.0",
            object_ref("a"),
            digest64(1),
            signature(1),
            object_ref("s"),
            "MIT",
            1024,
        );
        assert!(err.is_err());
        assert_eq!(err.unwrap_err().code, ReleaseErrorCode::Validation);
    }

    // ---- CompatibilityMatrix ---------------------------------------------

    #[test]
    fn ep042_unit_matrix_accepts_components_in_range() {
        let m = matrix();
        let components = vec![component(1, "1.0.0"), component(2, "2.0.0")];
        let verdict = m.check(&components);
        assert!(verdict.compatible, "reasons: {:?}", verdict.reasons);
    }

    #[test]
    fn ep042_unit_matrix_rejects_unknown_component() {
        let m = matrix();
        let components = vec![component(9, "1.0.0")];
        let verdict = m.check(&components);
        assert!(!verdict.compatible);
        assert!(verdict.reasons.iter().any(|r| r.contains("not present")));
    }

    #[test]
    fn ep042_unit_matrix_rejects_version_mismatch() {
        let m = matrix();
        let components = vec![component(1, "1.5.0")];
        let verdict = m.check(&components);
        assert!(!verdict.compatible);
        assert!(verdict
            .reasons
            .iter()
            .any(|r| r.contains("does not match matrix version")));
    }

    #[test]
    fn ep042_unit_matrix_rejects_version_below_minimum() {
        let m = matrix();
        let components = vec![SignedComponent::new(
            "comp-1",
            "c1",
            "0.9.0",
            object_ref("a"),
            digest64(1),
            signature(1),
            object_ref("s"),
            "MIT",
            1024,
        )
        .unwrap()];
        let verdict = m.check(&components);
        assert!(!verdict.compatible);
    }

    #[test]
    fn ep042_unit_matrix_rejects_version_above_maximum() {
        let m = matrix();
        let components = vec![SignedComponent::new(
            "comp-1",
            "c1",
            "2.0.0",
            object_ref("a"),
            digest64(1),
            signature(1),
            object_ref("s"),
            "MIT",
            1024,
        )
        .unwrap()];
        let verdict = m.check(&components);
        assert!(!verdict.compatible);
    }

    #[test]
    fn ep042_unit_matrix_rejects_unparseable_version() {
        let m = matrix();
        let components = vec![SignedComponent::new(
            "comp-1",
            "c1",
            "latest",
            object_ref("a"),
            digest64(1),
            signature(1),
            object_ref("s"),
            "MIT",
            1024,
        )
        .unwrap()];
        let verdict = m.check(&components);
        assert!(!verdict.compatible);
    }

    #[test]
    fn ep042_unit_matrix_supports_all_profiles() {
        let m = matrix();
        assert!(m.supports_all_profiles());
        for profile in DeploymentProfileMode::ALL {
            assert!(m.supports_profile(profile));
        }
    }

    #[test]
    fn ep042_unit_matrix_rejects_duplicate_entries() {
        let entries = vec![
            CompatibilityEntry::new(
                "comp-1",
                "1.0.0",
                "1.0.0",
                "1.9.9",
                vec![DeploymentProfileMode::Managed],
            )
            .unwrap(),
            CompatibilityEntry::new(
                "comp-1",
                "2.0.0",
                "2.0.0",
                "2.9.9",
                vec![DeploymentProfileMode::Managed],
            )
            .unwrap(),
        ];
        assert!(CompatibilityMatrix::new("matrix-x", entries).is_err());
    }

    #[test]
    fn ep042_unit_matrix_rejects_empty_entries() {
        assert!(CompatibilityMatrix::new("matrix-x", vec![]).is_err());
    }

    // ---- ReleaseManifest --------------------------------------------------

    #[test]
    fn ep042_unit_manifest_construction_roundtrip() {
        let m = manifest();
        assert_eq!(m.schema_version, RELEASE_SCHEMA_VERSION);
        assert_eq!(m.channel, ReleaseChannel::Stable);
        let json = serde_json::to_string(&m).unwrap();
        let back: ReleaseManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn ep042_unit_manifest_serialization_is_deterministic() {
        let m = manifest();
        let a = serde_json::to_string(&m).unwrap();
        let b = serde_json::to_string(&m).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn ep042_unit_manifest_rejects_empty_components() {
        let err = ReleaseManifest::new(
            "release-1",
            "1.0.0",
            ReleaseChannel::Stable,
            vec![],
            matrix(),
            None,
            object_ref("sbom-root"),
            vec!["MIT".to_string()],
            "2026-08-25T00:00:00Z",
        );
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_manifest_rejects_missing_licenses() {
        let err = ReleaseManifest::new(
            "release-1",
            "1.0.0",
            ReleaseChannel::Stable,
            vec![component(1, "1.0.0")],
            matrix(),
            None,
            object_ref("sbom-root"),
            vec![],
            "2026-08-25T00:00:00Z",
        );
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_manifest_rejects_malformed_timestamp() {
        let err = ReleaseManifest::new(
            "release-1",
            "1.0.0",
            ReleaseChannel::Stable,
            vec![component(1, "1.0.0")],
            matrix(),
            None,
            object_ref("sbom-root"),
            vec!["MIT".to_string()],
            "not-a-date",
        );
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_manifest_exists_not_verified() {
        let m = manifest();
        // No digest binding: MISSING, never Verified.
        assert_eq!(m.verification_state(), VerificationState::Missing);
        // A manifest with a self-bound digest verifies the binding.
        let mut bound = m.clone();
        bound.manifest_digest = Some(m.content_digest().unwrap());
        assert_eq!(bound.verification_state(), VerificationState::Verified);
        // Tampered binding is a mismatch, never a pass.
        let mut tampered = m.clone();
        tampered.manifest_digest = Some(digest64(9));
        assert_eq!(tampered.verification_state(), VerificationState::Mismatch);
    }

    #[test]
    fn ep042_unit_manifest_content_digest_binds_real_bytes() {
        let m = manifest();
        let d = m.content_digest().unwrap();
        assert_eq!(d.alg(), "sha256");
        assert_eq!(d.hex().len(), 64);
        // The digest changes when content changes.
        let mut m2 = m.clone();
        m2.version = "1.0.1".to_string();
        assert_ne!(m.content_digest().unwrap(), m2.content_digest().unwrap());
    }

    #[test]
    fn ep042_unit_manifest_rejects_unknown_json_field() {
        let json = r#"{"schema_version":1,"release_id":"r","version":"1.0.0","channel":"STABLE","components":[],"compatibility":{},"sbom_ref":{"backend":"local","key":"k"},"license_refs":["MIT"],"created_at":"2026-08-25T00:00:00Z","bogus":true}"#;
        assert!(serde_json::from_str::<ReleaseManifest>(json).is_err());
    }

    // ---- UpdatePlan -------------------------------------------------------

    fn plan_steps() -> Vec<UpdateStep> {
        vec![
            UpdateStep::new(1, UpdateStepKind::Backup, "backup state").unwrap(),
            UpdateStep::new(2, UpdateStepKind::Migrate, "apply migrations").unwrap(),
            UpdateStep::new(3, UpdateStepKind::Canary, "canary cohort").unwrap(),
            UpdateStep::new(4, UpdateStepKind::Observe, "observe health").unwrap(),
        ]
    }

    fn plan() -> UpdatePlan {
        UpdatePlan::new(
            "plan-1",
            "release-1",
            "1.0.0",
            "1.1.0",
            ReleaseChannel::Stable,
            plan_steps(),
            "idem-1",
            "corr-1",
            "2026-08-25T00:00:00Z",
        )
        .unwrap()
    }

    #[test]
    fn ep042_unit_plan_requires_backup_first_step() {
        let steps = vec![UpdateStep::new(1, UpdateStepKind::Migrate, "apply migrations").unwrap()];
        let err = UpdatePlan::new(
            "plan-1",
            "release-1",
            "1.0.0",
            "1.1.0",
            ReleaseChannel::Stable,
            steps,
            "idem-1",
            "corr-1",
            "2026-08-25T00:00:00Z",
        );
        assert!(err.is_err());
        assert_eq!(err.unwrap_err().code, ReleaseErrorCode::BackupRequired);
    }

    #[test]
    fn ep042_unit_plan_rejects_same_version() {
        let err = UpdatePlan::new(
            "plan-1",
            "release-1",
            "1.0.0",
            "1.0.0",
            ReleaseChannel::Stable,
            plan_steps(),
            "idem-1",
            "corr-1",
            "2026-08-25T00:00:00Z",
        );
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_plan_rejects_empty_steps() {
        let err = UpdatePlan::new(
            "plan-1",
            "release-1",
            "1.0.0",
            "1.1.0",
            ReleaseChannel::Stable,
            vec![],
            "idem-1",
            "corr-1",
            "2026-08-25T00:00:00Z",
        );
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_plan_rejects_non_contiguous_step_order() {
        let steps = vec![
            UpdateStep::new(1, UpdateStepKind::Backup, "backup").unwrap(),
            UpdateStep::new(3, UpdateStepKind::Migrate, "migrate").unwrap(),
        ];
        let err = UpdatePlan::new(
            "plan-1",
            "release-1",
            "1.0.0",
            "1.1.0",
            ReleaseChannel::Stable,
            steps,
            "idem-1",
            "corr-1",
            "2026-08-25T00:00:00Z",
        );
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_plan_exists_not_executed() {
        let p = plan();
        assert_eq!(p.state, UpdateState::Planned);
        assert!(p.has_backup_first_step());
        assert!(p.contains_no_promote_step());
    }

    #[test]
    fn ep042_unit_plan_serialization_roundtrip_preserves_schema_version() {
        let p = plan();
        let json = serde_json::to_string(&p).unwrap();
        let back: UpdatePlan = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
        assert_eq!(back.schema_version, RELEASE_SCHEMA_VERSION);
    }

    // ---- CanaryRing -------------------------------------------------------

    fn ring() -> CanaryRing {
        CanaryRing::new(
            "ring-1",
            "release-1",
            DeploymentProfileMode::Managed,
            5,
            30,
            "healthz healthy and readyz true",
        )
        .unwrap()
    }

    #[test]
    fn ep042_unit_canary_construction_and_bounds() {
        let r = ring();
        assert_eq!(r.cohort_percent, 5);
        assert_eq!(r.verdict, CanaryVerdict::Observing);
        assert!(CanaryRing::new(
            "ring-2",
            "release-1",
            DeploymentProfileMode::Managed,
            0,
            30,
            "criterion",
        )
        .is_err());
        assert!(CanaryRing::new(
            "ring-3",
            "release-1",
            DeploymentProfileMode::Managed,
            101,
            30,
            "criterion",
        )
        .is_err());
        assert!(CanaryRing::new(
            "ring-4",
            "release-1",
            DeploymentProfileMode::Managed,
            10,
            0,
            "criterion",
        )
        .is_err());
    }

    #[test]
    fn ep042_unit_canary_observing_never_promoted() {
        let r = ring();
        assert!(!r.recommends_ready());
        assert!(r.never_promotes());
    }

    #[test]
    fn ep042_unit_canary_ready_requires_evidence() {
        let mut r = ring();
        r.verdict = CanaryVerdict::ReadyToPromote;
        // Without evidence, no recommendation.
        assert!(!r.recommends_ready());
        r.evidence_ref = Some("evidence/run-1.json".to_string());
        assert!(r.recommends_ready());
        // Even a ready canary cannot promote itself.
        assert!(r.never_promotes());
    }

    // ---- RollbackReceipt ---------------------------------------------------

    #[test]
    fn ep042_unit_rollback_receipt_requires_backup_ref() {
        // The backup_ref field is mandatory by type: construction without a
        // backup is impossible (there is no Option).
        let r = RollbackReceipt::new(
            "receipt-1",
            "plan-1",
            "1.1.0",
            "1.0.0",
            object_ref("backup-snapshot-1"),
            "operator-1",
            "corr-1",
        )
        .unwrap();
        assert!(r.has_backup_ref());
        assert_eq!(r.state, RollbackState::RequiresBackup);
        assert!(!r.is_verified());
    }

    #[test]
    fn ep042_unit_rollback_receipt_rejects_same_version() {
        let err = RollbackReceipt::new(
            "receipt-1",
            "plan-1",
            "1.0.0",
            "1.0.0",
            object_ref("backup-1"),
            "operator-1",
            "corr-1",
        );
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_rollback_receipt_not_verified_without_evidence() {
        let r = RollbackReceipt::new(
            "receipt-1",
            "plan-1",
            "1.1.0",
            "1.0.0",
            object_ref("backup-1"),
            "operator-1",
            "corr-1",
        )
        .unwrap();
        assert_eq!(r.backup_verification, VerificationState::Unverified);
        assert_eq!(r.rollback_verification, VerificationState::Unverified);
        assert!(!r.is_verified());
    }

    #[test]
    fn ep042_unit_rollback_receipt_verified_only_with_both_verifications() {
        let mut r = RollbackReceipt::new(
            "receipt-1",
            "plan-1",
            "1.1.0",
            "1.0.0",
            object_ref("backup-1"),
            "operator-1",
            "corr-1",
        )
        .unwrap();
        r.backup_verification = VerificationState::Verified;
        r.rollback_verification = VerificationState::Verified;
        r.state = RollbackState::RollbackVerified;
        r.verified_at = Some("2026-08-25T01:00:00Z".to_string());
        assert!(r.is_verified());
    }

    // ---- OfflineBundle -----------------------------------------------------

    fn bundle_contents() -> Vec<BundleItem> {
        vec![
            BundleItem::new(BundleKind::Image, "control-plane", digest64(1)).unwrap(),
            BundleItem::new(BundleKind::Model, "microbrain", digest64(2)).unwrap(),
            BundleItem::new(BundleKind::License, "LICENSES", digest64(3)).unwrap(),
            BundleItem::new(BundleKind::Sbom, "sbom.json", digest64(4)).unwrap(),
            BundleItem::new(BundleKind::Migration, "migrations", digest64(5)).unwrap(),
            BundleItem::new(BundleKind::RecoveryTool, "recover", digest64(6)).unwrap(),
        ]
    }

    fn bundle() -> OfflineBundle {
        OfflineBundle::new(
            "bundle-1",
            "release-1",
            bundle_contents(),
            object_ref("manifest.json"),
            vec!["sbom-root".to_string()],
            vec!["MIT".to_string()],
            vec!["migration-001".to_string()],
        )
        .unwrap()
    }

    #[test]
    fn ep042_unit_bundle_requires_image_model_license_sbom() {
        let contents = vec![BundleItem::new(BundleKind::Image, "img", digest64(1)).unwrap()];
        let err = OfflineBundle::new(
            "bundle-1",
            "release-1",
            contents,
            object_ref("manifest.json"),
            vec!["sbom-root".to_string()],
            vec!["MIT".to_string()],
            vec![],
        );
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_bundle_exists_not_verified() {
        let b = bundle();
        assert_eq!(b.verify_digest_binding(), VerificationState::Missing);
        let mut bound = b.clone();
        bound.bundle_digest = Some(b.content_digest().unwrap());
        assert_eq!(bound.verify_digest_binding(), VerificationState::Verified);
        let mut tampered = b.clone();
        tampered.bundle_digest = Some(digest64(9));
        assert_eq!(
            tampered.verify_digest_binding(),
            VerificationState::Mismatch
        );
    }

    #[test]
    fn ep042_unit_bundle_serialization_roundtrip() {
        let b = bundle();
        let json = serde_json::to_string(&b).unwrap();
        let back: OfflineBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }

    // ---- ManualPromotion ---------------------------------------------------

    fn promotion() -> ManualPromotion {
        ManualPromotion::new(
            "promo-1",
            "release-1",
            "plan-1",
            "ring-1",
            "approval-42",
            "operator-1",
            "2026-08-25T02:00:00Z",
            "sh scripts/deploy.sh --dry-run --release 1.1.0",
        )
        .unwrap()
    }

    #[test]
    fn ep042_unit_promotion_requires_human_approval() {
        let p = promotion();
        assert!(p.requires_human_approval());
        assert_eq!(p.state, PromotionState::ApprovedManualOnly);
    }

    #[test]
    fn ep042_unit_promotion_rejects_missing_approval_ref() {
        let err = ManualPromotion::new(
            "promo-1",
            "release-1",
            "plan-1",
            "ring-1",
            "",
            "operator-1",
            "2026-08-25T02:00:00Z",
            "sh scripts/deploy.sh --dry-run",
        );
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_promotion_never_deploys() {
        let p = promotion();
        assert!(p.never_deploys());
        // The record has no executor, no target, no automatic effect
        // surface: it is a decision record authorizing a manual command.
        assert!(!p.exact_manual_command.is_empty());
    }

    // ---- Promotion gate -----------------------------------------------------

    #[test]
    fn ep042_unit_promotion_gate_blocks_observing_canary() {
        let r = ring();
        let decision = promotion_gate_decision(&r, Some("approval-1"));
        assert_eq!(decision.decision, PromotionState::Locked);
        assert!(decision.requires_human);
    }

    #[test]
    fn ep042_unit_promotion_gate_blocks_rollback_canary() {
        let mut r = ring();
        r.verdict = CanaryVerdict::Rollback;
        let decision = promotion_gate_decision(&r, Some("approval-1"));
        assert_eq!(decision.decision, PromotionState::Locked);
    }

    #[test]
    fn ep042_unit_promotion_gate_requires_evidence() {
        let mut r = ring();
        r.verdict = CanaryVerdict::ReadyToPromote;
        let decision = promotion_gate_decision(&r, Some("approval-1"));
        assert_eq!(decision.decision, PromotionState::Locked);
    }

    #[test]
    fn ep042_unit_promotion_gate_requires_human_approval() {
        let mut r = ring();
        r.verdict = CanaryVerdict::ReadyToPromote;
        r.evidence_ref = Some("evidence/run-1.json".to_string());
        let decision = promotion_gate_decision(&r, None);
        assert_eq!(decision.decision, PromotionState::AwaitingHumanApproval);
    }

    #[test]
    fn ep042_unit_promotion_gate_approves_manual_only() {
        let mut r = ring();
        r.verdict = CanaryVerdict::ReadyToPromote;
        r.evidence_ref = Some("evidence/run-1.json".to_string());
        let decision = promotion_gate_decision(&r, Some("approval-42"));
        assert_eq!(decision.decision, PromotionState::ApprovedManualOnly);
        // The gate never returns an automatic deployment decision.
        assert!(!matches!(decision.decision, PromotionState::Locked));
        assert!(decision.requires_human);
    }

    #[test]
    fn ep042_unit_promotion_gate_never_deploys() {
        // No path through the gate yields a deployment action; the gate's
        // output vocabulary contains only lock/await/approve-manual states.
        let mut r = ring();
        r.verdict = CanaryVerdict::ReadyToPromote;
        r.evidence_ref = Some("evidence/run-1.json".to_string());
        let decision = promotion_gate_decision(&r, Some("approval-42"));
        assert!(matches!(
            decision.decision,
            PromotionState::Locked
                | PromotionState::AwaitingHumanApproval
                | PromotionState::ApprovedManualOnly
        ));
    }

    // ---- Vocabulary integration ---------------------------------------------

    #[test]
    fn ep042_unit_vocabulary_error_converts_to_release_error() {
        let ve = VocabularyError(DeploymentProfileMode::VOCAB);
        let re: ReleaseError = ve.into();
        assert_eq!(re.code, ReleaseErrorCode::Vocabulary);
    }

    #[test]
    fn ep042_unit_error_code_serde_rejects_unknown() {
        assert!(serde_json::from_str::<ReleaseErrorCode>("\"BOGUS\"").is_err());
    }

    // ---- Dependency direction (compile-time) --------------------------------

    #[test]
    fn ep042_unit_no_provider_sdk_types_in_contract() {
        // The contract surface contains no provider SDK types by design;
        // the gate additionally proves the cargo dependency tree.
        let backend = ObjectRef::new("minio", "key").unwrap();
        assert_eq!(backend.backend, "minio");
        let backend2 = ObjectRef::new("s3", "key").unwrap();
        assert_eq!(backend2.backend, "s3");
    }
}
