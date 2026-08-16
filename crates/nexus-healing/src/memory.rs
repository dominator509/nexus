//! EP-019 incident memory (SPEC-018 canonical term `IncidentMemory`).
//!
//! Durable, provider-neutral recall of past incidents: deduplication by
//! canonical signature, correlation of repeated incidents, and evidence
//! preservation. A successful remediation may become a candidate
//! reusable skill ONLY through the EP-018 skill proposal/evaluation/
//! signing/trust/install process -- the self-healing system can never
//! directly install its own generated skills.

use crate::contract::Incident;
use crate::error::HealingError;
use nexus_domain::{IncidentId, TenantId};

/// An incident memory record (redacted metadata only; no secrets,
/// credentials, private source context, or full model prompts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidentMemoryRecord {
    pub incident_id: IncidentId,
    pub tenant_id: TenantId,
    /// Canonical dedup key (deterministic signature).
    pub dedup_key: String,
    /// Error class of the incident.
    pub error_class: String,
    /// Affected component.
    pub component: String,
    /// Final state once the incident closes.
    pub final_state: Option<String>,
    /// Skill candidate reference (only after EP-018 eval + approval).
    pub skill_candidate_ref: Option<String>,
}

/// Incident memory port: deduplicated, correlated incident history.
pub trait IncidentMemory {
    /// Record an incident in memory (idempotent by incident id).
    fn record(&mut self, record: IncidentMemoryRecord) -> Result<(), HealingError>;

    /// Look up prior incidents by canonical dedup key (deduplication
    /// and repeated-incident handling).
    fn find_by_dedup_key(&self, dedup_key: &str) -> Vec<IncidentMemoryRecord>;

    /// Look up an incident by id.
    fn get(&self, incident_id: &IncidentId) -> Option<IncidentMemoryRecord>;

    /// Build the canonical dedup key for an incident (deterministic:
    /// tenant + error class + component + correlation, NOT raw error
    /// text alone and NEVER merged across tenant boundaries).
    fn canonical_dedup_key(tenant_id: &TenantId, error_class: &str, component: &str) -> String {
        format!("{}|{}|{}", tenant_id.as_str(), error_class, component)
    }
}

/// In-memory incident memory implementation (deterministic; M1 contract
/// boundary). Durable store integration is owned by later boundaries.
#[derive(Debug, Default)]
pub struct InMemoryIncidentMemory {
    records: Vec<IncidentMemoryRecord>,
}

impl InMemoryIncidentMemory {
    pub fn new() -> Self {
        Self::default()
    }
}

impl IncidentMemory for InMemoryIncidentMemory {
    fn record(&mut self, record: IncidentMemoryRecord) -> Result<(), HealingError> {
        if self
            .records
            .iter()
            .any(|r| r.incident_id == record.incident_id)
        {
            return Err(HealingError::conflict(
                "incident already recorded (idempotency)",
            ));
        }
        self.records.push(record);
        Ok(())
    }

    fn find_by_dedup_key(&self, dedup_key: &str) -> Vec<IncidentMemoryRecord> {
        let mut out: Vec<IncidentMemoryRecord> = self
            .records
            .iter()
            .filter(|r| r.dedup_key == dedup_key)
            .cloned()
            .collect();
        out.sort_by(|a, b| a.incident_id.as_str().cmp(b.incident_id.as_str()));
        out
    }

    fn get(&self, incident_id: &IncidentId) -> Option<IncidentMemoryRecord> {
        self.records
            .iter()
            .find(|r| &r.incident_id == incident_id)
            .cloned()
    }
}

/// Helper to build a record from a live incident (redacted).
impl From<&Incident> for IncidentMemoryRecord {
    fn from(incident: &Incident) -> Self {
        Self {
            incident_id: incident.incident_id.clone(),
            tenant_id: incident.tenant_id.clone(),
            dedup_key: incident.dedup_key.clone(),
            error_class: String::new(),
            component: incident.components.first().cloned().unwrap_or_default(),
            final_state: Some(incident.state.as_str().to_string()),
            skill_candidate_ref: None,
        }
    }
}
