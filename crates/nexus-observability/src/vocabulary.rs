//! EP-038 observability vocabularies (SPEC-007 canonical terms).
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

/// Canonical telemetry signal kinds (SPEC-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TelemetrySignal {
    Trace,
    Metric,
    Log,
    Incident,
    Health,
    Fleet,
    Slo,
}

impl TelemetrySignal {
    pub const VOCAB: &'static str = "telemetry signal";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Metric => "METRIC",
            Self::Log => "LOG",
            Self::Incident => "INCIDENT",
            Self::Health => "HEALTH",
            Self::Fleet => "FLEET",
            Self::Slo => "SLO",
        }
    }
}

impl FromStr for TelemetrySignal {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TRACE" => Ok(Self::Trace),
            "METRIC" => Ok(Self::Metric),
            "LOG" => Ok(Self::Log),
            "INCIDENT" => Ok(Self::Incident),
            "HEALTH" => Ok(Self::Health),
            "FLEET" => Ok(Self::Fleet),
            "SLO" => Ok(Self::Slo),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for TelemetrySignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical severity ladder (SPEC-007 alert/incident severity).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl Severity {
    pub const VOCAB: &'static str = "severity";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
        }
    }
}

impl FromStr for Severity {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DEBUG" => Ok(Self::Debug),
            "INFO" => Ok(Self::Info),
            "WARNING" => Ok(Self::Warning),
            "ERROR" => Ok(Self::Error),
            "CRITICAL" => Ok(Self::Critical),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical health state ladder (SPEC-007 behavior 4; never a boolean).
///
/// `CONFIGURED != REACHABLE != RESPONDING != READY`. Degraded means some
/// mandatory dependencies are unavailable; Unhealthy means liveness or
/// readiness failed; Unknown means no trustworthy observation exists.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HealthState {
    /// Declared or configured only; no observation yet.
    Configured,
    /// The endpoint/component is reachable on the network.
    Reachable,
    /// The component responded to a request.
    Responding,
    /// All mandatory dependencies verified; ready to serve.
    Ready,
    /// Serving but with degraded mandatory dependencies.
    Degraded,
    /// Liveness/readiness failed.
    Unhealthy,
    /// No trustworthy observation within the freshness window.
    Unknown,
}

impl HealthState {
    pub const VOCAB: &'static str = "health state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "CONFIGURED",
            Self::Reachable => "REACHABLE",
            Self::Responding => "RESPONDING",
            Self::Ready => "READY",
            Self::Degraded => "DEGRADED",
            Self::Unhealthy => "UNHEALTHY",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// A state counts as a positive readiness signal only at Ready.
    pub fn is_ready(self) -> bool {
        self == Self::Ready
    }

    /// A state that can never be claimed healthy.
    pub fn is_unhealthy_or_unknown(self) -> bool {
        matches!(self, Self::Unhealthy | Self::Unknown)
    }
}

impl FromStr for HealthState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CONFIGURED" => Ok(Self::Configured),
            "REACHABLE" => Ok(Self::Reachable),
            "RESPONDING" => Ok(Self::Responding),
            "READY" => Ok(Self::Ready),
            "DEGRADED" => Ok(Self::Degraded),
            "UNHEALTHY" => Ok(Self::Unhealthy),
            "UNKNOWN" => Ok(Self::Unknown),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for HealthState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical metric kinds (SPEC-007 metric catalog).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
    Distribution,
}

impl MetricKind {
    pub const VOCAB: &'static str = "metric kind";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Counter => "COUNTER",
            Self::Gauge => "GAUGE",
            Self::Histogram => "HISTOGRAM",
            Self::Distribution => "DISTRIBUTION",
        }
    }
}

impl FromStr for MetricKind {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "COUNTER" => Ok(Self::Counter),
            "GAUGE" => Ok(Self::Gauge),
            "HISTOGRAM" => Ok(Self::Histogram),
            "DISTRIBUTION" => Ok(Self::Distribution),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for MetricKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical incident state machine (SPEC-007 incidents).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IncidentState {
    Open,
    Acknowledged,
    Resolved,
    Suppressed,
}

impl IncidentState {
    pub const VOCAB: &'static str = "incident state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "OPEN",
            Self::Acknowledged => "ACKNOWLEDGED",
            Self::Resolved => "RESOLVED",
            Self::Suppressed => "SUPPRESSED",
        }
    }
}

impl FromStr for IncidentState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OPEN" => Ok(Self::Open),
            "ACKNOWLEDGED" => Ok(Self::Acknowledged),
            "RESOLVED" => Ok(Self::Resolved),
            "SUPPRESSED" => Ok(Self::Suppressed),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for IncidentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical SLO evaluation states (SPEC-007 SLOs).
///
/// `NoData` and `InsufficientEvidence` are explicitly NOT met: no events
/// never equals SLO met.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SloState {
    Met,
    Violated,
    NoData,
    InsufficientEvidence,
    Unknown,
}

impl SloState {
    pub const VOCAB: &'static str = "SLO state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Met => "MET",
            Self::Violated => "VIOLATED",
            Self::NoData => "NO_DATA",
            Self::InsufficientEvidence => "INSUFFICIENT_EVIDENCE",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// A green claim requires Met with a non-zero denominator.
    pub fn is_green(self) -> bool {
        self == Self::Met
    }
}

impl FromStr for SloState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MET" => Ok(Self::Met),
            "VIOLATED" => Ok(Self::Violated),
            "NO_DATA" => Ok(Self::NoData),
            "INSUFFICIENT_EVIDENCE" => Ok(Self::InsufficientEvidence),
            "UNKNOWN" => Ok(Self::Unknown),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for SloState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical redaction actions (SPEC-007 redaction; fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RedactionAction {
    /// Field is explicitly allowed to leave the component as-is.
    Keep,
    /// Field is dropped from exportable telemetry entirely.
    Drop,
    /// Field is replaced by a SHA-256 fingerprint (cannot be reversed).
    Hash,
    /// Field is replaced by a short truncated fingerprint for correlation.
    Fingerprint,
    /// Field is replaced by a fixed redaction marker.
    MarkRedacted,
}

impl RedactionAction {
    pub const VOCAB: &'static str = "redaction action";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "KEEP",
            Self::Drop => "DROP",
            Self::Hash => "HASH",
            Self::Fingerprint => "FINGERPRINT",
            Self::MarkRedacted => "MARK_REDACTED",
        }
    }
}

impl FromStr for RedactionAction {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "KEEP" => Ok(Self::Keep),
            "DROP" => Ok(Self::Drop),
            "HASH" => Ok(Self::Hash),
            "FINGERPRINT" => Ok(Self::Fingerprint),
            "MARK_REDACTED" => Ok(Self::MarkRedacted),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for RedactionAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical stability levels for contract artifacts (SPEC-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StabilityLevel {
    Stable,
    Beta,
    Alpha,
    Internal,
}

impl StabilityLevel {
    pub const VOCAB: &'static str = "stability level";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "STABLE",
            Self::Beta => "BETA",
            Self::Alpha => "ALPHA",
            Self::Internal => "INTERNAL",
        }
    }
}

impl FromStr for StabilityLevel {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "STABLE" => Ok(Self::Stable),
            "BETA" => Ok(Self::Beta),
            "ALPHA" => Ok(Self::Alpha),
            "INTERNAL" => Ok(Self::Internal),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for StabilityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical cardinality policies for metric labels (SPEC-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CardinalityPolicy {
    /// A small fixed label set; the exact names are part of the catalog.
    Fixed,
    /// Bounded enumerable values per label.
    Bounded,
    /// The label carries user-controlled values; high cardinality is
    /// denied unless the policy permits a redacted/fingerprinted form.
    DenyHighCardinality,
}

impl CardinalityPolicy {
    pub const VOCAB: &'static str = "cardinality policy";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "FIXED",
            Self::Bounded => "BOUNDED",
            Self::DenyHighCardinality => "DENY_HIGH_CARDINALITY",
        }
    }
}

impl FromStr for CardinalityPolicy {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FIXED" => Ok(Self::Fixed),
            "BOUNDED" => Ok(Self::Bounded),
            "DENY_HIGH_CARDINALITY" => Ok(Self::DenyHighCardinality),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for CardinalityPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_deny_unknown<T: FromStr<Err = VocabularyError>>(s: &str) {
        assert!(s.parse::<T>().is_err(), "expected {s} to be denied");
    }

    #[test]
    fn ep038_unit_vocabulary_deny_unknown_severity() {
        assert_eq!("ERROR".parse::<Severity>().unwrap(), Severity::Error);
        assert_deny_unknown::<Severity>("URGENT");
        assert_deny_unknown::<Severity>("");
        assert_deny_unknown::<Severity>("error"); // lowercase denied
    }

    #[test]
    fn ep038_unit_vocabulary_deny_unknown_health_state() {
        assert_eq!(
            "DEGRADED".parse::<HealthState>().unwrap(),
            HealthState::Degraded
        );
        assert_deny_unknown::<HealthState>("UP");
        assert_deny_unknown::<HealthState>("down");
        assert_deny_unknown::<HealthState>("PARTIAL");
        assert!(!HealthState::Configured.is_ready());
        assert!(!HealthState::Unhealthy.is_ready());
        assert!(HealthState::Ready.is_ready());
        assert!(HealthState::Unknown.is_unhealthy_or_unknown());
    }

    #[test]
    fn ep038_unit_vocabulary_deny_unknown_metric_kind() {
        assert_eq!(
            "COUNTER".parse::<MetricKind>().unwrap(),
            MetricKind::Counter
        );
        assert_deny_unknown::<MetricKind>("COUNT");
        assert_deny_unknown::<MetricKind>("Summary");
    }

    #[test]
    fn ep038_unit_vocabulary_deny_unknown_incident_state() {
        assert_eq!(
            "OPEN".parse::<IncidentState>().unwrap(),
            IncidentState::Open
        );
        assert_deny_unknown::<IncidentState>("NEW");
        assert_deny_unknown::<IncidentState>("closed");
    }

    #[test]
    fn ep038_unit_vocabulary_deny_unknown_slo_state() {
        assert_eq!("NO_DATA".parse::<SloState>().unwrap(), SloState::NoData);
        assert!(!SloState::NoData.is_green());
        assert!(!SloState::InsufficientEvidence.is_green());
        assert!(!SloState::Violated.is_green());
        assert!(SloState::Met.is_green());
        assert_deny_unknown::<SloState>("GREEN");
    }

    #[test]
    fn ep038_unit_vocabulary_deny_unknown_redaction_action() {
        assert_eq!(
            "HASH".parse::<RedactionAction>().unwrap(),
            RedactionAction::Hash
        );
        assert_deny_unknown::<RedactionAction>("ENCRYPT");
    }

    #[test]
    fn ep038_unit_vocabulary_deny_unknown_signal_and_stability() {
        assert_eq!(
            "TRACE".parse::<TelemetrySignal>().unwrap(),
            TelemetrySignal::Trace
        );
        assert_deny_unknown::<TelemetrySignal>("SPAN");
        assert_eq!(
            "STABLE".parse::<StabilityLevel>().unwrap(),
            StabilityLevel::Stable
        );
        assert_deny_unknown::<StabilityLevel>("GA");
        assert_eq!(
            "DENY_HIGH_CARDINALITY"
                .parse::<CardinalityPolicy>()
                .unwrap(),
            CardinalityPolicy::DenyHighCardinality
        );
        assert_deny_unknown::<CardinalityPolicy>("UNBOUNDED");
    }

    #[test]
    fn ep038_unit_vocabulary_serde_rejects_unknown_wire_value() {
        let raw = serde_json::json!({"severity": "CRITICAL", "state": "READY"});
        let sev: Severity = serde_json::from_value(raw.get("severity").unwrap().clone()).unwrap();
        assert_eq!(sev, Severity::Critical);
        let unknown = serde_json::json!("EMERGENCY");
        assert!(serde_json::from_value::<Severity>(unknown).is_err());
        assert!(serde_json::from_value::<HealthState>(serde_json::json!("GOOD")).is_err());
        assert!(serde_json::from_value::<SloState>(serde_json::json!("PASS")).is_err());
    }
}
