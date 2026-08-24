//! EP-040 testing/hardening/chaos vocabularies (SPEC-008 canonical terms;
//! TESTING.md test layers; node contract).
//!
//! Every public vocabulary is deny-unknown: arbitrary strings can never
//! silently become valid contract states. Each enum has a canonical
//! `as_str` form, a `FromStr` that rejects unknown values, and serde
//! serialization that fails closed on unknown wire values.

use std::fmt;
use std::str::FromStr;

/// Rejection reason for an unknown vocabulary value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocabularyError(pub &'static str);

impl fmt::Display for VocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown {} value", self.0)
    }
}

impl std::error::Error for VocabularyError {}

/// Canonical test layers (TESTING.md test layers 1-9).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TestLayer {
    /// Domain unit tests: pure invariants, parsers, policy tables.
    Unit,
    /// Property tests: idempotency, serialization, monotonicity.
    Property,
    /// Contract tests: JSON Schema, OpenAPI, AsyncAPI, SDK compatibility.
    Contract,
    /// Integration tests: real PostgreSQL, NATS, Temporal, Keycloak, etc.
    Integration,
    /// E2E tests: browser, Tauri, Flutter, CLI, control-plane entry points.
    E2e,
    /// Live-fire proofs against real services, providers, hardware.
    LiveFire,
    /// Provider certification: external account and observable effects.
    ProviderCertification,
    /// Hardware certification: physical model and firmware evidence.
    HardwareCertification,
    /// Performance, chaos, security, accessibility, privacy, and drills.
    Performance,
    /// Chaos and failure-injection behavior.
    Chaos,
    /// Security tests and adversarial coverage.
    Security,
    /// Accessibility audits against a declared standard.
    Accessibility,
    /// Privacy and data-classification tests.
    Privacy,
    /// Backup, restore, update, and rollback drills.
    Drill,
}

impl TestLayer {
    pub const VOCAB: &'static str = "test layer";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unit => "UNIT",
            Self::Property => "PROPERTY",
            Self::Contract => "CONTRACT",
            Self::Integration => "INTEGRATION",
            Self::E2e => "E2E",
            Self::LiveFire => "LIVE_FIRE",
            Self::ProviderCertification => "PROVIDER_CERTIFICATION",
            Self::HardwareCertification => "HARDWARE_CERTIFICATION",
            Self::Performance => "PERFORMANCE",
            Self::Chaos => "CHAOS",
            Self::Security => "SECURITY",
            Self::Accessibility => "ACCESSIBILITY",
            Self::Privacy => "PRIVACY",
            Self::Drill => "DRILL",
        }
    }
}

impl fmt::Display for TestLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TestLayer {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNIT" => Ok(Self::Unit),
            "PROPERTY" => Ok(Self::Property),
            "CONTRACT" => Ok(Self::Contract),
            "INTEGRATION" => Ok(Self::Integration),
            "E2E" => Ok(Self::E2e),
            "LIVE_FIRE" => Ok(Self::LiveFire),
            "PROVIDER_CERTIFICATION" => Ok(Self::ProviderCertification),
            "HARDWARE_CERTIFICATION" => Ok(Self::HardwareCertification),
            "PERFORMANCE" => Ok(Self::Performance),
            "CHAOS" => Ok(Self::Chaos),
            "SECURITY" => Ok(Self::Security),
            "ACCESSIBILITY" => Ok(Self::Accessibility),
            "PRIVACY" => Ok(Self::Privacy),
            "DRILL" => Ok(Self::Drill),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Canonical test outcome. SKIPPED TEST != PASSED TEST; IGNORED TEST !=
/// PASSED TEST; ZERO TESTS COLLECTED != GREEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TestOutcome {
    /// The test ran and its assertions held.
    Passed,
    /// The test ran and its assertions failed.
    Failed,
    /// The test was skipped; this is never a pass for a required test.
    Skipped,
    /// The test was ignored; this is never a pass for a required test.
    Ignored,
    /// Expected failure that did not occur; treated as a defect.
    XFailed,
    /// The test could not complete (timeout, environment, teardown).
    Blocked,
}

impl TestOutcome {
    pub const VOCAB: &'static str = "test outcome";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "PASSED",
            Self::Failed => "FAILED",
            Self::Skipped => "SKIPPED",
            Self::Ignored => "IGNORED",
            Self::XFailed => "XFAILED",
            Self::Blocked => "BLOCKED",
        }
    }

    /// A required test is only truly passed when it RAN and PASSED.
    pub fn is_required_pass(self) -> bool {
        matches!(self, Self::Passed)
    }
}

impl fmt::Display for TestOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TestOutcome {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PASSED" => Ok(Self::Passed),
            "FAILED" => Ok(Self::Failed),
            "SKIPPED" => Ok(Self::Skipped),
            "IGNORED" => Ok(Self::Ignored),
            "XFAILED" => Ok(Self::XFailed),
            "BLOCKED" => Ok(Self::Blocked),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Canonical flake classification (TESTING.md flaky policy; EP-040 fence).
/// A retry may classify a flake but must never erase it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FlakeClassification {
    /// Transient environment/ordering noise; root cause still required.
    Transient,
    /// A fixture left state behind that contaminated the next run.
    FixtureStateLeak,
    /// The run exhausted CPU, memory, disk, ports, or processes.
    ResourceExhaustion,
    /// Runtime ordering/timing between parallel tests.
    RuntimeOrdering,
    /// A foreign node's test or fixture interfered.
    ForeignNode,
    /// The global verify wrapper has a defect.
    GlobalVerifyDefect,
    /// The owning node's code regressed.
    OwnerCodeRegression,
    /// Environment configuration changed between runs.
    Environment,
    /// Credentials/auth blocked the operation.
    AuthBlocked,
}

impl FlakeClassification {
    pub const VOCAB: &'static str = "flake classification";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transient => "TRANSIENT",
            Self::FixtureStateLeak => "FIXTURE_STATE_LEAK",
            Self::ResourceExhaustion => "RESOURCE_EXHAUSTION",
            Self::RuntimeOrdering => "RUNTIME_ORDERING",
            Self::ForeignNode => "FOREIGN_NODE",
            Self::GlobalVerifyDefect => "GLOBAL_VERIFY_DEFECT",
            Self::OwnerCodeRegression => "OWNER_CODE_REGRESSION",
            Self::Environment => "ENVIRONMENT",
            Self::AuthBlocked => "AUTH_BLOCKED",
        }
    }
}

impl fmt::Display for FlakeClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FlakeClassification {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TRANSIENT" => Ok(Self::Transient),
            "FIXTURE_STATE_LEAK" => Ok(Self::FixtureStateLeak),
            "RESOURCE_EXHAUSTION" => Ok(Self::ResourceExhaustion),
            "RUNTIME_ORDERING" => Ok(Self::RuntimeOrdering),
            "FOREIGN_NODE" => Ok(Self::ForeignNode),
            "GLOBAL_VERIFY_DEFECT" => Ok(Self::GlobalVerifyDefect),
            "OWNER_CODE_REGRESSION" => Ok(Self::OwnerCodeRegression),
            "ENVIRONMENT" => Ok(Self::Environment),
            "AUTH_BLOCKED" => Ok(Self::AuthBlocked),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Canonical failure-injection kind for a chaos scenario (EP-040 fence:
/// unavailable dependency, timeout, malformed input, duplicate request,
/// denied permission, cancelled work, partial side effect, corrupted
/// message, exhausted budget, revoked token).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureInjectionKind {
    /// Terminate a container, process, or connection.
    Terminate,
    /// Exceed a timeout budget.
    Timeout,
    /// Send malformed input through a real boundary.
    MalformedInput,
    /// Replay a duplicate request with the same idempotency key.
    DuplicateRequest,
    /// Deny a permission the happy path relies on.
    DeniedPermission,
    /// Cancel in-flight work mid-transaction.
    CancelledWork,
    /// Fail after a partial side effect (never claim success).
    PartialSideEffect,
    /// Corrupt a controlled message at a real boundary.
    CorruptMessage,
    /// Exhaust a declared budget (retries, memory, quota).
    ExhaustBudget,
    /// Revoke a token mid-operation.
    RevokedToken,
    /// Make a real dependency unavailable.
    UnavailableDependency,
}

impl FailureInjectionKind {
    pub const VOCAB: &'static str = "failure injection kind";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Terminate => "TERMINATE",
            Self::Timeout => "TIMEOUT",
            Self::MalformedInput => "MALFORMED_INPUT",
            Self::DuplicateRequest => "DUPLICATE_REQUEST",
            Self::DeniedPermission => "DENIED_PERMISSION",
            Self::CancelledWork => "CANCELLED_WORK",
            Self::PartialSideEffect => "PARTIAL_SIDE_EFFECT",
            Self::CorruptMessage => "CORRUPT_MESSAGE",
            Self::ExhaustBudget => "EXHAUST_BUDGET",
            Self::RevokedToken => "REVOKED_TOKEN",
            Self::UnavailableDependency => "UNAVAILABLE_DEPENDENCY",
        }
    }
}

impl fmt::Display for FailureInjectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FailureInjectionKind {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TERMINATE" => Ok(Self::Terminate),
            "TIMEOUT" => Ok(Self::Timeout),
            "MALFORMED_INPUT" => Ok(Self::MalformedInput),
            "DUPLICATE_REQUEST" => Ok(Self::DuplicateRequest),
            "DENIED_PERMISSION" => Ok(Self::DeniedPermission),
            "CANCELLED_WORK" => Ok(Self::CancelledWork),
            "PARTIAL_SIDE_EFFECT" => Ok(Self::PartialSideEffect),
            "CORRUPT_MESSAGE" => Ok(Self::CorruptMessage),
            "EXHAUST_BUDGET" => Ok(Self::ExhaustBudget),
            "REVOKED_TOKEN" => Ok(Self::RevokedToken),
            "UNAVAILABLE_DEPENDENCY" => Ok(Self::UnavailableDependency),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Resource kinds owned by a test fixture or scenario (resource hygiene
/// model; EP-040 fence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceKind {
    Container,
    Network,
    Volume,
    TempRoot,
    CredentialFile,
    ChildProcess,
}

impl ResourceKind {
    pub const VOCAB: &'static str = "resource kind";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Container => "CONTAINER",
            Self::Network => "NETWORK",
            Self::Volume => "VOLUME",
            Self::TempRoot => "TEMP_ROOT",
            Self::CredentialFile => "CREDENTIAL_FILE",
            Self::ChildProcess => "CHILD_PROCESS",
        }
    }
}

impl fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ResourceKind {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CONTAINER" => Ok(Self::Container),
            "NETWORK" => Ok(Self::Network),
            "VOLUME" => Ok(Self::Volume),
            "TEMP_ROOT" => Ok(Self::TempRoot),
            "CREDENTIAL_FILE" => Ok(Self::CredentialFile),
            "CHILD_PROCESS" => Ok(Self::ChildProcess),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Hardening control state ladder: CONTROL DEFINED != CONTROL APPLIED !=
/// CONTROL VERIFIED != CONTROL REGRESSED. A written control is not proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HardeningControlState {
    /// The control exists as a written definition.
    Defined,
    /// The control is applied to the target.
    Applied,
    /// The control is verified by real evidence.
    Verified,
    /// A regression moved the control out of verified state.
    Regressed,
}

impl HardeningControlState {
    pub const VOCAB: &'static str = "hardening control state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Defined => "DEFINED",
            Self::Applied => "APPLIED",
            Self::Verified => "VERIFIED",
            Self::Regressed => "REGRESSED",
        }
    }
}

impl fmt::Display for HardeningControlState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HardeningControlState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DEFINED" => Ok(Self::Defined),
            "APPLIED" => Ok(Self::Applied),
            "VERIFIED" => Ok(Self::Verified),
            "REGRESSED" => Ok(Self::Regressed),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Blast radius scope for a chaos scenario. No scenario is valid without
/// a bounded blast radius and cleanup policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlastRadius {
    /// Confined to one test or one fixture.
    Single,
    /// Confined to the owning node's resources.
    Node,
    /// Confined to the workspace test surface.
    Workspace,
    /// Global scope is prohibited unless a later milestone explicitly owns it.
    Global,
}

impl BlastRadius {
    pub const VOCAB: &'static str = "blast radius";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Single => "SINGLE",
            Self::Node => "NODE",
            Self::Workspace => "WORKSPACE",
            Self::Global => "GLOBAL",
        }
    }
}

impl fmt::Display for BlastRadius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BlastRadius {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SINGLE" => Ok(Self::Single),
            "NODE" => Ok(Self::Node),
            "WORKSPACE" => Ok(Self::Workspace),
            "GLOBAL" => Ok(Self::Global),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Provider/hardware certification status. NotAsserted is the honest
/// default until real controlled-dependency evidence exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CertificationStatus {
    /// No certification evidence has been produced.
    NotAsserted,
    /// Evidence collection is in progress.
    InProgress,
    /// Certified against real controlled dependencies.
    Certified,
    /// Certification failed or evidence was rejected.
    Failed,
}

impl CertificationStatus {
    pub const VOCAB: &'static str = "certification status";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotAsserted => "NOT_ASSERTED",
            Self::InProgress => "IN_PROGRESS",
            Self::Certified => "CERTIFIED",
            Self::Failed => "FAILED",
        }
    }
}

impl fmt::Display for CertificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CertificationStatus {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NOT_ASSERTED" => Ok(Self::NotAsserted),
            "IN_PROGRESS" => Ok(Self::InProgress),
            "CERTIFIED" => Ok(Self::Certified),
            "FAILED" => Ok(Self::Failed),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}
