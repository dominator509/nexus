//! EP-038 observability model types (SPEC-007).
//!
//! Provider-neutral contract layer: TelemetryContext (the canonical
//! metadata envelope), RedactionPolicy (mandatory, fail-closed,
//! applied before egress), MetricDefinition, TracePolicy (present !=
//! exported != safe), health observations, incidents, fleet health
//! (staleness-visible), and SLO evaluation (no events never equals
//! met). No Prometheus/Grafana/OpenTelemetry-SDK/Sentry/GlitchTip
//! dependency: providers are owned by later milestones.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nexus_domain::{CorrelationId, IncidentId, Privacy, TenantId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{ObservabilityError, ObservabilityResult};
use crate::port::{HealthAggregator, IncidentSink, MetricCatalog, SloEvaluator};
use crate::vocabulary::{
    CardinalityPolicy, HealthState, IncidentState, MetricKind, RedactionAction, Severity, SloState,
    StabilityLevel, TelemetrySignal,
};

// ------------------------------------------------------------- helpers

/// Canonical UTC RFC3339 timestamp for evidence/export surfaces.
pub fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// SHA-256 fingerprint (full hex). Never reversible; used for redaction
/// hashing and high-cardinality label fingerprinting. The `sha256:`
/// prefix keeps the fingerprint distinguishable from raw 64-hex
/// artifact keys so export gates never re-classify a safe hash as a
/// secret-shaped value.
pub fn sha256_fingerprint(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let hex: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("sha256:{hex}")
}

/// Truncated fingerprint for correlation-safe redaction (first 16 hex).
pub fn short_fingerprint(value: &str) -> String {
    let full = sha256_fingerprint(value);
    format!("fp:{}", &full[7..23])
}

/// True when the value carries a secret-shaped pattern. Used by the
/// redaction policy and trace export gate; never returns the value.
pub fn is_secret_shaped(value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() {
        return false;
    }
    if v.len() > 512 {
        // Unbounded payloads are never exportable raw.
        return true;
    }
    let lower = v.to_ascii_lowercase();
    // AWS access key id shape.
    if v.starts_with("AKIA") && v.len() >= 20 {
        return true;
    }
    // Private key blocks.
    if v.contains("BEGIN") && v.contains("PRIVATE KEY") {
        return true;
    }
    // Bearer tokens.
    if lower.starts_with("bearer ") && v.len() > 16 {
        return true;
    }
    // Assignment shapes.
    for marker in [
        "password=",
        "passwd=",
        "secret=",
        "token=",
        "api_key=",
        "apikey=",
        "access_key=",
        "authorization:",
        "x-api-key:",
        "client_secret=",
    ] {
        if lower.contains(marker) {
            return true;
        }
    }
    // Email addresses (high-cardinality personal identifiers).
    if v.contains('@') && v.contains('.') && !v.contains(' ') {
        let (local, _) = v.split_once('@').unwrap();
        if local.len() >= 3 {
            return true;
        }
    }
    // Phone-like shapes: 7+ consecutive digits with optional + prefix.
    let digits: usize = v.chars().filter(|c| c.is_ascii_digit()).count();
    if digits >= 11
        && (v.starts_with('+')
            || v.chars()
                .all(|c| c.is_ascii_digit() || c == '-' || c == ' '))
    {
        return true;
    }
    // 64-hex content-address keys (artifact keys are high cardinality).
    if v.len() == 64 && v.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    // IPv4 addresses.
    let octets: Vec<&str> = v.split('.').collect();
    if octets.len() == 4
        && octets
            .iter()
            .all(|o| !o.is_empty() && o.len() <= 3 && o.chars().all(|c| c.is_ascii_digit()))
    {
        return true;
    }
    false
}

/// True when the value *contains* a secret-shaped token anywhere
/// (e.g. an AWS key embedded in a sentence). Fail-closed: a metadata
/// field that carries an embedded secret is never allowed into the
/// context envelope or an exportable artifact.
pub fn contains_secret_shaped(value: &str) -> bool {
    if is_secret_shaped(value) {
        return true;
    }
    let v = value.trim();
    let lower = v.to_ascii_lowercase();
    // Embedded AWS access key id token (AKIA + >=20 chars).
    for token in v.split_whitespace() {
        if token.starts_with("AKIA") && token.len() >= 20 {
            return true;
        }
    }
    // Embedded private key blocks.
    if v.contains("BEGIN") && v.contains("PRIVATE KEY") {
        return true;
    }
    // Embedded bearer tokens.
    if lower.contains("bearer ") && v.len() > 16 {
        return true;
    }
    // Embedded assignment shapes.
    for marker in [
        "password=",
        "passwd=",
        "secret=",
        "token=",
        "api_key=",
        "apikey=",
        "access_key=",
        "authorization:",
        "x-api-key:",
        "client_secret=",
    ] {
        if lower.contains(marker) {
            return true;
        }
    }
    false
}

// ------------------------------------------------------ TelemetryContext

/// Canonical telemetry metadata envelope (SPEC-007 behavior 1).
///
/// Holds only safe operational identity: node, tenant, business
/// context, correlation, request, trace/span ids, component,
/// operation, severity, timestamp, environment, and source interface.
/// Free-form string fields reject empty and secret-shaped values at
/// construction so the envelope can never carry raw secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryContext {
    pub node: String,
    pub tenant: Option<TenantId>,
    pub business_context: Option<String>,
    pub correlation: Option<CorrelationId>,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub component: String,
    pub operation: String,
    pub severity: Severity,
    pub timestamp: u64,
    pub environment: Option<String>,
    pub source_interface: Option<String>,
}

fn validate_safe_field(name: &str, value: &str, required: bool) -> ObservabilityResult<()> {
    let v = value.trim();
    if v.is_empty() {
        if required {
            return Err(ObservabilityError::validation(format!(
                "telemetry {name} must not be empty"
            )));
        }
        return Ok(());
    }
    if v.len() > 256 {
        return Err(ObservabilityError::validation(format!(
            "telemetry {name} exceeds 256 chars"
        )));
    }
    if contains_secret_shaped(v) {
        return Err(ObservabilityError::redaction_denied(format!(
            "telemetry {name} carries secret-shaped content and cannot enter the context"
        )));
    }
    Ok(())
}

fn validate_trace_id(value: &str) -> ObservabilityResult<()> {
    if value.len() != 32 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ObservabilityError::validation(
            "trace_id must be 32 lowercase hex chars",
        ));
    }
    Ok(())
}

fn validate_span_id(value: &str) -> ObservabilityResult<()> {
    if value.len() != 16 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ObservabilityError::validation(
            "span_id must be 16 lowercase hex chars",
        ));
    }
    Ok(())
}

impl TelemetryContext {
    /// Construct a validated telemetry envelope.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node: impl Into<String>,
        tenant: Option<TenantId>,
        business_context: Option<String>,
        correlation: Option<CorrelationId>,
        request_id: Option<String>,
        trace_id: Option<String>,
        span_id: Option<String>,
        component: impl Into<String>,
        operation: impl Into<String>,
        severity: Severity,
        environment: Option<String>,
        source_interface: Option<String>,
    ) -> ObservabilityResult<Self> {
        let node = node.into();
        let component = component.into();
        let operation = operation.into();
        validate_safe_field("node", &node, true)?;
        validate_safe_field("component", &component, true)?;
        validate_safe_field("operation", &operation, true)?;
        if let Some(v) = &business_context {
            validate_safe_field("business_context", v, false)?;
        }
        if let Some(v) = &request_id {
            validate_safe_field("request_id", v, false)?;
        }
        if let Some(v) = &environment {
            validate_safe_field("environment", v, false)?;
        }
        if let Some(v) = &source_interface {
            validate_safe_field("source_interface", v, false)?;
        }
        if let Some(v) = &trace_id {
            validate_trace_id(v)?;
        }
        if let Some(v) = &span_id {
            validate_span_id(v)?;
        }
        if trace_id.is_none() && span_id.is_some() {
            return Err(ObservabilityError::validation(
                "span_id requires a trace_id",
            ));
        }
        Ok(Self {
            node,
            tenant,
            business_context,
            correlation,
            request_id,
            trace_id,
            span_id,
            component,
            operation,
            severity,
            timestamp: now_epoch_secs(),
            environment,
            source_interface,
        })
    }

    /// The signal kind this envelope describes.
    pub fn signal(&self) -> TelemetrySignal {
        TelemetrySignal::Trace
    }

    /// Serialize to canonical JSON (safe by construction).
    pub fn to_json(&self) -> ObservabilityResult<String> {
        serde_json::to_string(self).map_err(|e| {
            ObservabilityError::internal(format!("telemetry context serialization failed: {e}"))
        })
    }
}

// -------------------------------------------------------- RedactedEnvelope

/// The only form telemetry may take when leaving the local component
/// boundary (SPEC-007 behavior 2; fail-closed redaction before egress).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedEnvelope {
    pub signal: TelemetrySignal,
    pub context: TelemetryContext,
    /// Safe key/value pairs after redaction. Secret-shaped values never
    /// appear here.
    pub fields: BTreeMap<String, String>,
    /// Field names that were redacted or dropped in this pass.
    pub redacted_fields: Vec<String>,
    /// True when redaction policy actually ran.
    pub policy_applied: bool,
}

impl RedactedEnvelope {
    pub fn new(
        signal: TelemetrySignal,
        context: TelemetryContext,
        fields: BTreeMap<String, String>,
        redacted_fields: Vec<String>,
    ) -> Self {
        Self {
            signal,
            context,
            fields,
            redacted_fields,
            policy_applied: true,
        }
    }

    /// Exportable telemetry must never carry a secret-shaped value.
    pub fn assert_exportable(&self) -> ObservabilityResult<()> {
        for (k, v) in &self.fields {
            if is_secret_shaped(v) {
                return Err(ObservabilityError::redaction_denied(format!(
                    "exportable field {k} carries secret-shaped content"
                )));
            }
        }
        Ok(())
    }
}

// ------------------------------------------------------- RedactionPolicy

/// Mandatory, fail-closed redaction policy (SPEC-007 behavior 2).
///
/// Observed raw events are never exportable; every value passes through
/// `apply` first. Explicitly allowed fields are kept only when they do
/// not carry secret-shaped content; sensitive fields are redacted;
/// unclassified values fail closed to a redaction marker; secret-shaped
/// values are hashed or dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionPolicy {
    /// Field names explicitly allowed to leave as-is (when safe).
    pub allowed_fields: Vec<String>,
    /// Field names always redacted regardless of content.
    pub sensitive_fields: Vec<String>,
    /// Action applied to secret-shaped values.
    pub secret_action: RedactionAction,
    /// Action applied to unclassified values not in either list.
    pub unclassified_action: RedactionAction,
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        Self {
            allowed_fields: vec![
                "component".into(),
                "operation".into(),
                "node".into(),
                "severity".into(),
                "unit".into(),
                "metric".into(),
                "state".into(),
            ],
            sensitive_fields: vec![
                "payload".into(),
                "body".into(),
                "prompt".into(),
                "request".into(),
                "response".into(),
                "secret".into(),
                "token".into(),
                "password".into(),
                "api_key".into(),
                "authorization".into(),
                "connector".into(),
                "audio".into(),
                "image".into(),
            ],
            secret_action: RedactionAction::Hash,
            unclassified_action: RedactionAction::MarkRedacted,
        }
    }
}

impl RedactionPolicy {
    pub fn new(
        allowed_fields: Vec<String>,
        sensitive_fields: Vec<String>,
        secret_action: RedactionAction,
        unclassified_action: RedactionAction,
    ) -> Self {
        Self {
            allowed_fields,
            sensitive_fields,
            secret_action,
            unclassified_action,
        }
    }

    /// Classify a single field value. Never returns the raw value for a
    /// redacted field.
    fn apply_value(&self, field: &str, value: &str) -> (Option<String>, bool) {
        let is_sensitive = self.sensitive_fields.iter().any(|f| f == field);
        let is_allowed = self.allowed_fields.iter().any(|f| f == field);
        if is_secret_shaped(value) {
            let out = match self.secret_action {
                RedactionAction::Keep => None, // never: secret-shaped never kept
                RedactionAction::Drop => None,
                RedactionAction::Hash => Some(sha256_fingerprint(value)),
                RedactionAction::Fingerprint => Some(short_fingerprint(value)),
                RedactionAction::MarkRedacted => Some("[REDACTED:secret]".to_string()),
            };
            return (out, true);
        }
        if is_sensitive {
            return (Some("[REDACTED]".to_string()), true);
        }
        if is_allowed {
            return (Some(value.to_string()), false);
        }
        match self.unclassified_action {
            RedactionAction::Keep => (Some(value.to_string()), false),
            RedactionAction::Drop => (None, true),
            RedactionAction::Hash => (Some(sha256_fingerprint(value)), true),
            RedactionAction::Fingerprint => (Some(short_fingerprint(value)), true),
            RedactionAction::MarkRedacted => (Some("[REDACTED]".to_string()), true),
        }
    }

    /// Apply redaction to observed raw fields before egress.
    ///
    /// Raw payload fields are denied by default (the default sensitive
    /// list includes `payload`/`body`/`prompt`/`request`/`response`).
    pub fn apply(
        &self,
        signal: TelemetrySignal,
        context: TelemetryContext,
        observed: Vec<(String, String)>,
    ) -> RedactedEnvelope {
        let mut fields = BTreeMap::new();
        let mut redacted = Vec::new();
        for (field, value) in observed {
            match self.apply_value(&field, &value) {
                (Some(safe), true) => {
                    fields.insert(field.clone(), safe);
                    redacted.push(field);
                }
                (Some(safe), false) => {
                    fields.insert(field, safe);
                }
                (None, true) => redacted.push(field),
                (None, false) => {}
            }
        }
        RedactedEnvelope::new(signal, context, fields, redacted)
    }

    /// Fail-closed check: is a raw value exportable under this policy?
    pub fn is_exportable(&self, field: &str, value: &str) -> bool {
        if is_secret_shaped(value) {
            return false;
        }
        if self.sensitive_fields.iter().any(|f| f == field) {
            return false;
        }
        if !self.allowed_fields.iter().any(|f| f == field) {
            return matches!(self.unclassified_action, RedactionAction::Keep);
        }
        true
    }
}

// -------------------------------------------------------- MetricDefinition

/// Canonical metric definition (SPEC-007 metric catalog).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricDefinition {
    /// Stable dotted identifier, e.g. `nexus.requests.total`.
    pub id: String,
    pub description: String,
    pub unit: String,
    pub kind: MetricKind,
    pub labels: Vec<String>,
    pub cardinality: CardinalityPolicy,
    pub privacy: Privacy,
    pub owner: String,
    pub stability: StabilityLevel,
    pub aggregation: String,
}

fn validate_metric_id(id: &str) -> ObservabilityResult<()> {
    if id.is_empty() || id.len() > 128 {
        return Err(ObservabilityError::validation(
            "metric id must be 1..=128 chars",
        ));
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-');
    if !ok {
        return Err(ObservabilityError::validation(
            "metric id must be lowercase dotted identifier",
        ));
    }
    Ok(())
}

fn validate_label_name(label: &str) -> ObservabilityResult<()> {
    if label.is_empty() || label.len() > 64 {
        return Err(ObservabilityError::validation("label name 1..=64 chars"));
    }
    let ok = label
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.');
    if !ok {
        return Err(ObservabilityError::validation(
            "label name must be lowercase identifier",
        ));
    }
    Ok(())
}

impl MetricDefinition {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        unit: impl Into<String>,
        kind: MetricKind,
        labels: Vec<String>,
        cardinality: CardinalityPolicy,
        privacy: Privacy,
        owner: impl Into<String>,
        stability: StabilityLevel,
        aggregation: impl Into<String>,
    ) -> ObservabilityResult<Self> {
        let id = id.into();
        validate_metric_id(&id)?;
        let description = description.into();
        if description.is_empty() {
            return Err(ObservabilityError::validation(
                "metric description required",
            ));
        }
        let owner = owner.into();
        if owner.trim().is_empty() {
            return Err(ObservabilityError::validation("metric owner required"));
        }
        for l in &labels {
            validate_label_name(l)?;
        }
        Ok(Self {
            id,
            description,
            unit: unit.into(),
            kind,
            labels,
            cardinality,
            privacy,
            owner,
            stability,
            aggregation: aggregation.into(),
        })
    }
}

// ------------------------------------------------------------- TracePolicy

/// Trace context (SPEC-007 traces). Presence of a trace id never implies
/// exportability or safety.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub sampled: bool,
}

impl TraceContext {
    pub fn new(
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        sampled: bool,
    ) -> ObservabilityResult<Self> {
        let trace_id = trace_id.into();
        let span_id = span_id.into();
        validate_trace_id(&trace_id)?;
        validate_span_id(&span_id)?;
        Ok(Self {
            trace_id,
            span_id,
            sampled,
        })
    }
}

/// Export decision for a trace (SPEC-007 trace privacy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceExportDecision {
    /// Sampled and safe after redaction; carries only redacted attrs.
    Exportable {
        redacted_attributes: BTreeMap<String, String>,
    },
    /// Trace exists internally but is not sampled; never exported.
    Unsampled,
    /// Trace is sampled but policy denies export (privacy).
    Denied { reason: String },
}

/// Canonical trace policy: span attribute redaction + export gating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracePolicy {
    /// Attribute keys that are always denied in spans.
    pub denied_attribute_keys: Vec<String>,
    /// Redaction policy applied to span attributes.
    pub redaction: RedactionPolicy,
}

impl Default for TracePolicy {
    fn default() -> Self {
        Self {
            denied_attribute_keys: vec![
                "prompt".into(),
                "payload".into(),
                "body".into(),
                "request".into(),
                "response".into(),
                "secret".into(),
                "token".into(),
                "password".into(),
                "connector".into(),
                "audio".into(),
                "image".into(),
            ],
            redaction: RedactionPolicy::default(),
        }
    }
}

impl TracePolicy {
    pub fn new(denied_attribute_keys: Vec<String>, redaction: RedactionPolicy) -> Self {
        Self {
            denied_attribute_keys,
            redaction,
        }
    }

    /// Decide whether a trace may be exported. The trace can exist
    /// (trace id present) while still being unsampled or denied.
    pub fn check_export(
        &self,
        trace: &TraceContext,
        attributes: &[(String, String)],
    ) -> TraceExportDecision {
        if !trace.sampled {
            return TraceExportDecision::Unsampled;
        }
        let mut redacted_attributes = BTreeMap::new();
        let mut denied_reason: Option<String> = None;
        for (key, value) in attributes {
            if self.denied_attribute_keys.iter().any(|k| k == key) {
                // A denied attribute key is never exportable. The trace
                // may exist internally, but carrying a denied key means
                // it is not safe to export (fail-closed).
                denied_reason = Some(format!("attribute {key} denied (policy)"));
                continue;
            }
            if let (Some(safe), _) = self.redaction.apply_value(key, value) {
                redacted_attributes.insert(key.clone(), safe);
            }
        }
        if let Some(reason) = denied_reason {
            return TraceExportDecision::Denied { reason };
        }
        // Safety: no redacted attribute may still carry secret content.
        for (k, v) in &redacted_attributes {
            if is_secret_shaped(v) {
                return TraceExportDecision::Denied {
                    reason: format!("attribute {k} still secret-shaped after redaction"),
                };
            }
        }
        TraceExportDecision::Exportable {
            redacted_attributes,
        }
    }
}

// -------------------------------------------------------- Health building

/// One observed component health check (SPEC-007 behavior 4). Health is
/// derived from observations with timestamps, never from configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub component: String,
    pub state: HealthState,
    pub last_seen: u64,
    pub detail: Option<String>,
}

impl ComponentHealth {
    pub fn new(
        component: impl Into<String>,
        state: HealthState,
        last_seen: u64,
        detail: Option<String>,
    ) -> Self {
        Self {
            component: component.into(),
            state,
            last_seen,
            detail,
        }
    }

    pub fn is_stale(&self, now: u64, window_secs: u64) -> bool {
        now.saturating_sub(self.last_seen) > window_secs
    }
}

/// A composed health report for one node/component set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthReport {
    pub subject: String,
    pub state: HealthState,
    pub components: Vec<ComponentHealth>,
    pub composed_at: u64,
}

// ------------------------------------------------------------ Incidents

/// Delivery/recording result of an incident report (provider-neutral).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncidentDeliveryResult {
    /// Recorded/queued for the sink (never claims external delivery).
    Recorded,
    /// Same dedupe key already open at equal/lower severity.
    Deduplicated,
    /// Explicitly suppressed by policy.
    Suppressed,
    /// The sink failed; the incident is retained for retry.
    Failed { reason: String },
}

/// Canonical incident record (SPEC-007 incidents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incident {
    pub incident_id: IncidentId,
    pub dedupe_key: String,
    pub severity: Severity,
    pub classification: String,
    pub source: String,
    pub correlation: Option<CorrelationId>,
    pub state: IncidentState,
    pub redacted_context: RedactedEnvelope,
    pub opened_at: u64,
    pub acknowledged_at: Option<u64>,
    pub resolved_at: Option<u64>,
    /// True when this incident is a severity escalation of a prior one.
    pub escalated: bool,
}

impl Incident {
    /// Canonical dedupe key from stable operational identity only.
    pub fn dedupe_key_for(
        source: &str,
        classification: &str,
        correlation: Option<&CorrelationId>,
    ) -> String {
        match correlation {
            Some(c) => format!("{source}:{classification}:{}", c.as_str()),
            None => format!("{source}:{classification}"),
        }
    }
}

// ------------------------------------------------------------- FleetHealth

/// Per-node fleet observation (SPEC-007 fleet health).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeHealthReport {
    pub node: String,
    pub state: HealthState,
    pub components: Vec<ComponentHealth>,
    pub last_observed: u64,
}

impl NodeHealthReport {
    pub fn new(
        node: impl Into<String>,
        components: Vec<ComponentHealth>,
        last_observed: u64,
    ) -> Self {
        let state = Self::compose(&components);
        Self {
            node: node.into(),
            state,
            components,
            last_observed,
        }
    }

    fn compose(components: &[ComponentHealth]) -> HealthState {
        if components.is_empty() {
            return HealthState::Unknown;
        }
        if components.iter().any(|c| c.state == HealthState::Unhealthy) {
            return HealthState::Unhealthy;
        }
        if components.iter().any(|c| c.state == HealthState::Unknown) {
            return HealthState::Degraded;
        }
        if components.iter().all(|c| c.state == HealthState::Ready) {
            return HealthState::Ready;
        }
        // Partial readiness: some ready, some below -> degraded.
        if components.iter().any(|c| c.state == HealthState::Ready)
            && components.iter().any(|c| c.state != HealthState::Ready)
        {
            return HealthState::Degraded;
        }
        if components
            .iter()
            .any(|c| c.state == HealthState::Responding)
        {
            return HealthState::Responding;
        }
        if components.iter().any(|c| c.state == HealthState::Reachable) {
            return HealthState::Reachable;
        }
        HealthState::Configured
    }

    pub fn is_stale(&self, now: u64, window_secs: u64) -> bool {
        now.saturating_sub(self.last_observed) > window_secs
    }
}

/// Fleet-wide summary (SPEC-007 fleet health). One healthy node never
/// makes an unknown/unhealthy fleet healthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetSummary {
    pub total: u64,
    pub ready: u64,
    pub degraded: u64,
    pub unhealthy: u64,
    pub unknown: u64,
    pub stale: u64,
}

/// Canonical aggregate fleet view with explicit staleness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetHealth {
    pub nodes: Vec<NodeHealthReport>,
    pub freshness_window_secs: u64,
    pub composed_at: u64,
}

impl FleetHealth {
    pub fn new(nodes: Vec<NodeHealthReport>, freshness_window_secs: u64, composed_at: u64) -> Self {
        Self {
            nodes,
            freshness_window_secs,
            composed_at,
        }
    }

    /// Summarize at `now`. Stale nodes are classified unknown; the
    /// summary is always staleness-visible.
    pub fn summary(&self, now: u64) -> FleetSummary {
        let mut s = FleetSummary {
            total: self.nodes.len() as u64,
            ready: 0,
            degraded: 0,
            unhealthy: 0,
            unknown: 0,
            stale: 0,
        };
        for n in &self.nodes {
            if n.is_stale(now, self.freshness_window_secs) {
                s.stale += 1;
                s.unknown += 1;
                continue;
            }
            match n.state {
                HealthState::Ready => s.ready += 1,
                HealthState::Degraded => s.degraded += 1,
                HealthState::Unhealthy => s.unhealthy += 1,
                _ => s.unknown += 1,
            }
        }
        s
    }

    /// The fleet is healthy only when every node is fresh and Ready.
    /// Last-known-healthy is never currently-healthy.
    pub fn is_healthy(&self, now: u64) -> bool {
        !self.nodes.is_empty()
            && self.nodes.iter().all(|n| {
                !n.is_stale(now, self.freshness_window_secs) && n.state == HealthState::Ready
            })
    }

    /// True when any critical node is missing, stale, unknown, degraded,
    /// or unhealthy. Used to block claims that need fleet confidence.
    pub fn unsafe_to_claim(&self, now: u64, critical: &[&str]) -> bool {
        critical.iter().any(|name| {
            match self.nodes.iter().find(|n| n.node == *name) {
                None => true, // critical node absent
                Some(n) => {
                    n.is_stale(now, self.freshness_window_secs) || n.state != HealthState::Ready
                }
            }
        })
    }
}

// ------------------------------------------------------------------ SLOs

/// Canonical SLO definition (SPEC-007 SLOs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SloDefinition {
    pub id: String,
    /// Target ratio in (0, 1), e.g. 0.99.
    pub target: f64,
    pub window: Duration,
    pub event_source: String,
    /// Minimum total events before Met/Violated may be claimed.
    pub min_evidence: u64,
}

impl SloDefinition {
    pub fn new(
        id: impl Into<String>,
        target: f64,
        window: Duration,
        event_source: impl Into<String>,
        min_evidence: u64,
    ) -> ObservabilityResult<Self> {
        let id = id.into();
        validate_metric_id(&id)?;
        if !(target > 0.0 && target < 1.0) {
            return Err(ObservabilityError::validation(
                "SLO target must be strictly between 0 and 1",
            ));
        }
        if window.is_zero() {
            return Err(ObservabilityError::validation(
                "SLO window must be non-zero",
            ));
        }
        Ok(Self {
            id,
            target,
            window,
            event_source: event_source.into(),
            min_evidence,
        })
    }
}

/// Canonical SLO evaluation (SPEC-007 SLOs). No events never equals met.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SloEvaluation {
    pub slo_id: String,
    pub target: f64,
    pub window_secs: u64,
    pub good: u64,
    pub total: u64,
    pub error_budget_remaining: f64,
    pub burn_rate: f64,
    pub status: SloState,
    pub evidence_basis: String,
}

// ------------------------------------------------- canonical implementations

/// Real catalog registry: deny-unknown metric ids, bounded labels,
/// high-cardinality value rejection.
#[derive(Debug, Clone, Default)]
pub struct MetricRegistry {
    definitions: BTreeMap<String, MetricDefinition>,
}

impl MetricRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &MetricDefinition> {
        self.definitions.values()
    }
}

impl MetricCatalog for MetricRegistry {
    fn register(&mut self, definition: MetricDefinition) -> ObservabilityResult<()> {
        if self.definitions.contains_key(&definition.id) {
            return Err(ObservabilityError::conflict(format!(
                "metric {} already registered",
                definition.id
            )));
        }
        self.definitions.insert(definition.id.clone(), definition);
        Ok(())
    }

    fn lookup(&self, id: &str) -> Option<&MetricDefinition> {
        self.definitions.get(id)
    }

    fn validate_label(&self, metric: &str, label: &str, value: &str) -> ObservabilityResult<()> {
        let def = self.definitions.get(metric).ok_or_else(|| {
            ObservabilityError::unsupported_signal(format!("unknown metric {metric}"))
        })?;
        if !def.labels.iter().any(|l| l == label) {
            return Err(ObservabilityError::validation(format!(
                "label {label} not declared on metric {metric}"
            )));
        }
        if is_secret_shaped(value) {
            return Err(ObservabilityError::redaction_denied(format!(
                "label {label} value is secret-shaped and cannot enter the metric"
            )));
        }
        if def.cardinality == CardinalityPolicy::DenyHighCardinality {
            // High-cardinality values are denied unless they are already
            // a fingerprint/hash form.
            if value.len() > 64 {
                return Err(ObservabilityError::policy(format!(
                    "label {label} value is high cardinality on {metric}"
                )));
            }
            if value.len() >= 16 && value.starts_with("fp:") {
                return Ok(());
            }
            if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()) {
                return Ok(());
            }
            return Err(ObservabilityError::policy(format!(
                "label {label} value must be fingerprinted for {metric}"
            )));
        }
        Ok(())
    }
}

/// Real health composition from observed checks (never config).
#[derive(Debug, Clone, Default)]
pub struct CompositeHealthAggregator {
    checks: BTreeMap<String, ComponentHealth>,
}

impl CompositeHealthAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn component_count(&self) -> usize {
        self.checks.len()
    }
}

impl HealthAggregator for CompositeHealthAggregator {
    fn ingest(&mut self, check: ComponentHealth) {
        self.checks.insert(check.component.clone(), check);
    }

    fn compose(&self, now: u64, window_secs: u64) -> HealthState {
        if self.checks.is_empty() {
            return HealthState::Unknown;
        }
        let components: Vec<&ComponentHealth> = self.checks.values().collect();
        // Staleness: any stale mandatory observation is Unknown, never
        // healthy.
        let mut any_stale = false;
        for c in &components {
            if c.is_stale(now, window_secs) {
                any_stale = true;
                break;
            }
        }
        if any_stale {
            return HealthState::Unknown;
        }
        if components.iter().any(|c| c.state == HealthState::Unhealthy) {
            return HealthState::Unhealthy;
        }
        if components.iter().all(|c| c.state == HealthState::Ready) {
            return HealthState::Ready;
        }
        // Partial mandatory dependencies -> degraded, never blind
        // healthy.
        if components.iter().any(|c| c.state == HealthState::Ready)
            && components.iter().any(|c| c.state != HealthState::Ready)
        {
            return HealthState::Degraded;
        }
        if components
            .iter()
            .any(|c| c.state == HealthState::Responding)
        {
            return HealthState::Responding;
        }
        if components.iter().any(|c| c.state == HealthState::Reachable) {
            return HealthState::Reachable;
        }
        if components
            .iter()
            .any(|c| c.state == HealthState::Configured)
        {
            return HealthState::Configured;
        }
        HealthState::Unknown
    }

    fn report(&self, subject: &str, now: u64) -> HealthReport {
        HealthReport {
            subject: subject.to_string(),
            state: self.compose(now, 0),
            components: self.checks.values().cloned().collect(),
            composed_at: now,
        }
    }
}

/// Real incident recorder with dedupe and severity escalation.
/// Provider-neutral: this is the contract boundary; external delivery
/// (GlitchTip/Slack/PagerDuty) is owned by later milestones.
#[derive(Debug, Clone, Default)]
pub struct RecordingIncidentSink {
    incidents: BTreeMap<String, Incident>,
    /// incident id string -> dedupe key string.
    by_id: BTreeMap<String, String>,
}

impl RecordingIncidentSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.incidents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.incidents.is_empty()
    }

    pub fn get(&self, incident_id: &IncidentId) -> Option<&Incident> {
        self.by_id
            .get(incident_id.as_str())
            .and_then(|key| self.incidents.get(key))
    }

    pub fn open_incidents(&self) -> Vec<&Incident> {
        self.incidents
            .values()
            .filter(|i| matches!(i.state, IncidentState::Open | IncidentState::Acknowledged))
            .collect()
    }
}

impl IncidentSink for RecordingIncidentSink {
    fn report(
        &mut self,
        incident_id: IncidentId,
        dedupe_key: String,
        severity: Severity,
        classification: &str,
        source: &str,
        correlation: Option<CorrelationId>,
        redacted_context: RedactedEnvelope,
    ) -> IncidentDeliveryResult {
        let now = now_epoch_secs();
        if let Some(existing) = self.incidents.get(&dedupe_key) {
            if matches!(
                existing.state,
                IncidentState::Open | IncidentState::Acknowledged
            ) {
                if severity > existing.severity {
                    // Escalation: never hidden by dedupe. A new record
                    // is created at the higher severity.
                    let escalated = Incident {
                        incident_id: incident_id.clone(),
                        dedupe_key: dedupe_key.clone(),
                        severity,
                        classification: classification.to_string(),
                        source: source.to_string(),
                        correlation: correlation.clone(),
                        state: IncidentState::Open,
                        redacted_context,
                        opened_at: now,
                        acknowledged_at: None,
                        resolved_at: None,
                        escalated: true,
                    };
                    self.by_id
                        .insert(incident_id.as_str().to_string(), dedupe_key.clone());
                    self.incidents.insert(dedupe_key.clone(), escalated);
                    return IncidentDeliveryResult::Recorded;
                }
                return IncidentDeliveryResult::Deduplicated;
            }
            if existing.state == IncidentState::Suppressed {
                return IncidentDeliveryResult::Suppressed;
            }
            // Resolved incidents with the same key: allow a fresh report.
        }
        let incident = Incident {
            incident_id: incident_id.clone(),
            dedupe_key: dedupe_key.clone(),
            severity,
            classification: classification.to_string(),
            source: source.to_string(),
            correlation,
            state: IncidentState::Open,
            redacted_context,
            opened_at: now,
            acknowledged_at: None,
            resolved_at: None,
            escalated: false,
        };
        self.by_id
            .insert(incident_id.as_str().to_string(), dedupe_key.clone());
        self.incidents.insert(dedupe_key, incident);
        IncidentDeliveryResult::Recorded
    }

    fn acknowledge(&mut self, incident_id: &IncidentId) -> ObservabilityResult<()> {
        let key = self
            .by_id
            .get(incident_id.as_str())
            .ok_or_else(|| ObservabilityError::not_found("incident not found"))?
            .clone();
        let incident = self
            .incidents
            .get_mut(key.as_str())
            .ok_or_else(|| ObservabilityError::not_found("incident not found"))?;
        if incident.state == IncidentState::Resolved {
            return Err(ObservabilityError::conflict(
                "cannot acknowledge a resolved incident",
            ));
        }
        incident.state = IncidentState::Acknowledged;
        incident.acknowledged_at = Some(now_epoch_secs());
        Ok(())
    }

    fn resolve(&mut self, incident_id: &IncidentId) -> ObservabilityResult<()> {
        let key = self
            .by_id
            .get(incident_id.as_str())
            .ok_or_else(|| ObservabilityError::not_found("incident not found"))?
            .clone();
        let incident = self
            .incidents
            .get_mut(key.as_str())
            .ok_or_else(|| ObservabilityError::not_found("incident not found"))?;
        if incident.state == IncidentState::Resolved {
            return Err(ObservabilityError::conflict("incident already resolved"));
        }
        incident.state = IncidentState::Resolved;
        incident.resolved_at = Some(now_epoch_secs());
        Ok(())
    }
}

/// Real windowed SLO evaluator: zero denominator is NoData; below the
/// evidence threshold is InsufficientEvidence; never green without data.
#[derive(Debug, Clone, Default)]
pub struct WindowedSloEvaluator;

impl SloEvaluator for WindowedSloEvaluator {
    fn evaluate(&self, slo: &SloDefinition, good: u64, total: u64) -> SloEvaluation {
        let window_secs = slo.window.as_secs();
        if total == 0 {
            return SloEvaluation {
                slo_id: slo.id.clone(),
                target: slo.target,
                window_secs,
                good,
                total,
                error_budget_remaining: 0.0,
                burn_rate: 0.0,
                status: SloState::NoData,
                evidence_basis: "total=0".to_string(),
            };
        }
        if total < slo.min_evidence {
            return SloEvaluation {
                slo_id: slo.id.clone(),
                target: slo.target,
                window_secs,
                good,
                total,
                error_budget_remaining: 0.0,
                burn_rate: 0.0,
                status: SloState::InsufficientEvidence,
                evidence_basis: format!("total={total} < min_evidence={}", slo.min_evidence),
            };
        }
        let actual = good as f64 / total as f64;
        let budget_remaining = actual - slo.target;
        let burn_rate = if (1.0 - slo.target) > 0.0 {
            ((1.0 - actual) / (1.0 - slo.target)).max(0.0)
        } else {
            0.0
        };
        let status = if budget_remaining >= 0.0 {
            SloState::Met
        } else {
            SloState::Violated
        };
        SloEvaluation {
            slo_id: slo.id.clone(),
            target: slo.target,
            window_secs,
            good,
            total,
            error_budget_remaining: budget_remaining,
            burn_rate,
            status,
            evidence_basis: format!("good={good} total={total}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::{CardinalityPolicy, MetricKind, Severity, StabilityLevel};

    fn tenant() -> TenantId {
        "01970000-0000-7000-8000-000000000001".parse().unwrap()
    }

    fn correlation() -> CorrelationId {
        "01970000-0000-7000-8000-000000000011".parse().unwrap()
    }

    fn context(component: &str) -> TelemetryContext {
        TelemetryContext::new(
            "node-a",
            Some(tenant()),
            None,
            Some(correlation()),
            None,
            Some("0123456789abcdef0123456789abcdef".to_string()),
            Some("0123456789abcdef".to_string()),
            component,
            "op",
            Severity::Info,
            Some("test".to_string()),
            Some("ep038".to_string()),
        )
        .unwrap()
    }

    #[test]
    fn ep038_unit_telemetry_context_rejects_secret_shaped_field() {
        let err = TelemetryContext::new(
            "node-a",
            None,
            None,
            None,
            None,
            None,
            None,
            "component",
            "op AKIAIOSFODNN7EXAMPLE",
            Severity::Info,
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(
            err.code,
            crate::error::ObservabilityErrorCode::RedactionDenied
        );
    }

    #[test]
    fn ep038_unit_telemetry_context_requires_trace_for_span() {
        let err = TelemetryContext::new(
            "node-a",
            None,
            None,
            None,
            None,
            None,
            Some("0123456789abcdef".to_string()),
            "component",
            "op",
            Severity::Info,
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(err.code, crate::error::ObservabilityErrorCode::Validation);
    }

    #[test]
    fn ep038_unit_telemetry_context_serializes_safely() {
        let ctx = context("storage");
        let json = ctx.to_json().unwrap();
        assert!(json.contains("node-a"));
        assert!(!json.contains("AKIA"));
    }

    #[test]
    fn ep038_unit_redaction_hashes_secret_shaped_values() {
        let policy = RedactionPolicy::default();
        let envelope = policy.apply(
            TelemetrySignal::Metric,
            context("storage"),
            vec![
                ("component".to_string(), "storage".to_string()),
                ("aws_key".to_string(), "AKIAIOSFODNN7EXAMPLE".to_string()),
                ("payload".to_string(), "raw body".to_string()),
                ("metric".to_string(), "nexus.requests.total".to_string()),
            ],
        );
        envelope.assert_exportable().unwrap();
        assert_eq!(envelope.fields.get("component").unwrap(), "storage");
        assert_eq!(
            envelope.fields.get("metric").unwrap(),
            "nexus.requests.total"
        );
        // Secret-shaped value hashed, never kept raw. The hash is
        // prefixed so it can never be re-classified as a raw artifact key.
        let hashed = envelope.fields.get("aws_key").unwrap();
        assert_ne!(hashed, "AKIAIO...MPLE");
        assert!(hashed.starts_with("sha256:"));
        assert_eq!(hashed.len(), 71);
        // Sensitive payload redacted to marker.
        assert_eq!(envelope.fields.get("payload").unwrap(), "[REDACTED]");
        assert!(envelope.redacted_fields.contains(&"aws_key".to_string()));
    }

    #[test]
    fn ep038_unit_redaction_raw_payload_denied_by_default() {
        let policy = RedactionPolicy::default();
        assert!(!policy.is_exportable("payload", "raw body"));
        assert!(!policy.is_exportable("prompt", "write a poem"));
        assert!(!policy.is_exportable("component", "AKIAIOSFODNN7EXAMPLE"));
        assert!(policy.is_exportable("component", "storage"));
    }

    #[test]
    fn ep038_unit_redaction_unclassified_fails_closed() {
        let policy = RedactionPolicy::default();
        let envelope = policy.apply(
            TelemetrySignal::Log,
            context("app"),
            vec![(
                "mystery_field".to_string(),
                "unclassified value".to_string(),
            )],
        );
        // Unclassified never kept raw by default: marked redacted.
        assert_eq!(envelope.fields.get("mystery_field").unwrap(), "[REDACTED]");
    }

    #[test]
    fn ep038_unit_metric_catalog_deny_unknown_and_cardinality() {
        let mut registry = MetricRegistry::new();
        let def = MetricDefinition::new(
            "nexus.requests.total",
            "total requests",
            "1",
            MetricKind::Counter,
            vec!["tenant_hash".to_string()],
            CardinalityPolicy::DenyHighCardinality,
            Privacy::Public,
            "core",
            StabilityLevel::Stable,
            "SUM",
        )
        .unwrap();
        registry.register(def).unwrap();
        assert!(registry.lookup("nexus.requests.total").is_some());
        // Unknown metric denied.
        let err = registry.validate_label("nexus.nonexistent", "tenant_hash", "abc");
        assert!(err.is_err());
        // Undeclared label denied.
        let err = registry.validate_label("nexus.requests.total", "user_id", "abc");
        assert!(err.is_err());
        // Raw high-cardinality value denied.
        let err = registry.validate_label(
            "nexus.requests.total",
            "tenant_hash",
            "01970000-0000-7000-8000-000000000001",
        );
        assert!(err.is_err());
        // Fingerprinted value allowed.
        registry
            .validate_label(
                "nexus.requests.total",
                "tenant_hash",
                &short_fingerprint("01970000-0000-7000-8000-000000000001"),
            )
            .unwrap();
        // Duplicate registration denied.
        let dup = MetricDefinition::new(
            "nexus.requests.total",
            "dup",
            "1",
            MetricKind::Counter,
            vec![],
            CardinalityPolicy::Fixed,
            Privacy::Public,
            "core",
            StabilityLevel::Stable,
            "SUM",
        )
        .unwrap();
        assert!(registry.register(dup).is_err());
    }

    #[test]
    fn ep038_unit_metric_catalog_rejects_unsafe_label_values() {
        let mut registry = MetricRegistry::new();
        let def = MetricDefinition::new(
            "nexus.workflow.duration",
            "workflow duration",
            "ms",
            MetricKind::Histogram,
            vec!["workflow".to_string()],
            CardinalityPolicy::DenyHighCardinality,
            Privacy::Public,
            "core",
            StabilityLevel::Beta,
            "HISTOGRAM",
        )
        .unwrap();
        registry.register(def).unwrap();
        for bad in [
            "user@example.com",
            "+15551234567",
            "AKIAIOSFODNN7EXAMPLE",
            "01970000-0000-7000-8000-000000000001",
            "192.168.1.1",
            "some very long prompt text that exceeds the cardinality budget",
        ] {
            let err = registry.validate_label("nexus.workflow.duration", "workflow", bad);
            assert!(err.is_err(), "expected {bad} to be rejected");
        }
    }

    #[test]
    fn ep038_unit_trace_present_not_exported_not_safe() {
        let policy = TracePolicy::default();
        let sampled =
            TraceContext::new("0123456789abcdef0123456789abcdef", "0123456789abcdef", true)
                .unwrap();
        let unsampled = TraceContext::new(
            "0123456789abcdef0123456789abcdef",
            "0123456789abcdef",
            false,
        )
        .unwrap();
        // Unsampled: trace id present but never exported.
        assert_eq!(
            policy.check_export(&unsampled, &[]),
            TraceExportDecision::Unsampled
        );
        // Sampled with denied sensitive attribute -> denied export.
        let denied = policy.check_export(
            &sampled,
            &[("payload".to_string(), "secret content".to_string())],
        );
        assert!(matches!(denied, TraceExportDecision::Denied { .. }));
        // Sampled with safe attributes -> exportable, redacted.
        let ok = policy.check_export(
            &sampled,
            &[
                ("component".to_string(), "storage".to_string()),
                ("aws_key".to_string(), "AKIAIOSFODNN7EXAMPLE".to_string()),
            ],
        );
        match ok {
            TraceExportDecision::Exportable {
                redacted_attributes,
            } => {
                assert_eq!(redacted_attributes.get("component").unwrap(), "storage");
                let v = redacted_attributes.get("aws_key").unwrap();
                assert_ne!(v, "AKIAIOSFODNN7EXAMPLE");
            }
            other => panic!("expected exportable, got {other:?}"),
        }
    }

    #[test]
    fn ep038_unit_health_configured_not_ready_and_stale_not_healthy() {
        let mut agg = CompositeHealthAggregator::new();
        let now = 1_000_000;
        // No observations: unknown, never ready.
        assert_eq!(agg.compose(now, 60), HealthState::Unknown);
        // Config-like observation only (Configured): never ready.
        agg.ingest(ComponentHealth::new(
            "db",
            HealthState::Configured,
            now,
            None,
        ));
        assert_eq!(agg.compose(now, 60), HealthState::Configured);
        // Fresh Ready observation -> Ready.
        agg.ingest(ComponentHealth::new("db", HealthState::Ready, now, None));
        assert_eq!(agg.compose(now, 60), HealthState::Ready);
        // Stale Ready observation -> Unknown, not healthy.
        let stale_now = now + 120;
        assert_eq!(agg.compose(stale_now, 60), HealthState::Unknown);
    }

    #[test]
    fn ep038_unit_health_partial_dependencies_degraded() {
        let mut agg = CompositeHealthAggregator::new();
        let now = 1_000_000;
        agg.ingest(ComponentHealth::new("db", HealthState::Ready, now, None));
        agg.ingest(ComponentHealth::new(
            "queue",
            HealthState::Reachable,
            now,
            None,
        ));
        // Partial mandatory dependency -> degraded, not blindly healthy.
        assert_eq!(agg.compose(now, 60), HealthState::Degraded);
    }

    #[test]
    fn ep038_unit_incident_dedupe_and_escalation() {
        let mut sink = RecordingIncidentSink::new();
        let id1: IncidentId = "01970000-0000-7000-8000-000000000021".parse().unwrap();
        let id2: IncidentId = "01970000-0000-7000-8000-000000000022".parse().unwrap();
        let id3: IncidentId = "01970000-0000-7000-8000-000000000023".parse().unwrap();
        let key = Incident::dedupe_key_for("storage", "operations", Some(&correlation()));
        let envelope = RedactionPolicy::default().apply(
            TelemetrySignal::Incident,
            context("storage"),
            vec![("detail".to_string(), "AKIAIOSFODNN7EXAMPLE".to_string())],
        );
        let r1 = sink.report(
            id1.clone(),
            key.clone(),
            Severity::Warning,
            "operations",
            "storage",
            Some(correlation()),
            envelope.clone(),
        );
        assert_eq!(r1, IncidentDeliveryResult::Recorded);
        // Same key, same severity: deduplicated.
        let r2 = sink.report(
            id2.clone(),
            key.clone(),
            Severity::Warning,
            "operations",
            "storage",
            Some(correlation()),
            envelope.clone(),
        );
        assert_eq!(r2, IncidentDeliveryResult::Deduplicated);
        // Higher severity: escalation recorded, never hidden.
        let r3 = sink.report(
            id3.clone(),
            key.clone(),
            Severity::Critical,
            "operations",
            "storage",
            Some(correlation()),
            envelope.clone(),
        );
        assert_eq!(r3, IncidentDeliveryResult::Recorded);
        let escalated = sink.get(&id3).unwrap();
        assert!(escalated.escalated);
        assert_eq!(escalated.severity, Severity::Critical);
        // Incident context never carries the raw secret.
        let json = serde_json::to_string(escalated).unwrap();
        assert!(!json.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn ep038_unit_incident_redacted_body_and_state_transitions() {
        let mut sink = RecordingIncidentSink::new();
        let id: IncidentId = "01970000-0000-7000-8000-000000000024".parse().unwrap();
        let key = "storage:security:unique-1".to_string();
        let envelope = RedactionPolicy::default().apply(
            TelemetrySignal::Incident,
            context("storage"),
            vec![("prompt".to_string(), "private prompt text".to_string())],
        );
        sink.report(
            id.clone(),
            key,
            Severity::Error,
            "security",
            "storage",
            None,
            envelope,
        );
        sink.acknowledge(&id).unwrap();
        let incident = sink.get(&id).unwrap().clone();
        assert_eq!(incident.state, IncidentState::Acknowledged);
        sink.resolve(&id).unwrap();
        let incident = sink.get(&id).unwrap().clone();
        assert_eq!(incident.state, IncidentState::Resolved);
        assert!(incident.resolved_at.is_some());
        // Unknown id rejected.
        let missing: IncidentId = "01970000-0000-7000-8000-000000000025".parse().unwrap();
        assert!(sink.resolve(&missing).is_err());
        // Redacted context: prompt never raw.
        let json = serde_json::to_string(&incident).unwrap();
        assert!(!json.contains("private prompt text"));
    }

    #[test]
    fn ep038_unit_fleet_stale_node_not_healthy() {
        let now = 1_000_000;
        let fresh = NodeHealthReport::new(
            "node-a",
            vec![ComponentHealth::new("core", HealthState::Ready, now, None)],
            now,
        );
        let stale = NodeHealthReport::new(
            "node-b",
            vec![ComponentHealth::new(
                "core",
                HealthState::Ready,
                now - 500,
                None,
            )],
            now - 500,
        );
        let fleet = FleetHealth::new(vec![fresh, stale], 60, now);
        assert!(!fleet.is_healthy(now));
        let s = fleet.summary(now);
        assert_eq!(s.stale, 1);
        assert_eq!(s.unknown, 1);
        assert_eq!(s.ready, 1);
    }

    #[test]
    fn ep038_unit_fleet_unknown_critical_unsafe_to_claim() {
        let now = 1_000_000;
        let healthy = NodeHealthReport::new(
            "node-a",
            vec![ComponentHealth::new("core", HealthState::Ready, now, None)],
            now,
        );
        // One healthy node must not make the fleet claimable when a
        // critical node is absent.
        let fleet = FleetHealth::new(vec![healthy], 60, now);
        assert!(fleet.unsafe_to_claim(now, &["node-a", "node-critical"]));
        assert!(!fleet.unsafe_to_claim(now, &["node-a"]));
    }

    #[test]
    fn ep038_unit_slo_no_events_never_met() {
        let evaluator = WindowedSloEvaluator;
        let slo = SloDefinition::new(
            "nexus.api.availability",
            0.99,
            Duration::from_secs(3600),
            "api",
            10,
        )
        .unwrap();
        let zero = evaluator.evaluate(&slo, 0, 0);
        assert_eq!(zero.status, SloState::NoData);
        assert!(!zero.status.is_green());
        let insufficient = evaluator.evaluate(&slo, 5, 5);
        assert_eq!(insufficient.status, SloState::InsufficientEvidence);
        assert!(!insufficient.status.is_green());
        let met = evaluator.evaluate(&slo, 999, 1000);
        assert_eq!(met.status, SloState::Met);
        assert!(met.status.is_green());
        assert!(met.error_budget_remaining >= 0.0);
        let violated = evaluator.evaluate(&slo, 950, 1000);
        assert_eq!(violated.status, SloState::Violated);
        assert!(violated.error_budget_remaining < 0.0);
        assert!(violated.burn_rate > 0.0);
    }

    #[test]
    fn ep038_unit_slo_target_validation() {
        assert!(SloDefinition::new("slo.one", 0.0, Duration::from_secs(60), "src", 1).is_err());
        assert!(SloDefinition::new("slo.two", 1.0, Duration::from_secs(60), "src", 1).is_err());
        assert!(SloDefinition::new("slo.three", 0.99, Duration::ZERO, "src", 1).is_err());
        assert!(SloDefinition::new("slo.four", 0.99, Duration::from_secs(60), "src", 1).is_ok());
    }
}
