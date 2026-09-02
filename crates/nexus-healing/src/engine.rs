//! EP-019 production incident engine (SPEC-018; ADR-026; AUD-017).
//!
//! The `IncidentEngine` port previously had NO production implementation:
//! live-fire tests hand-constructed `SandboxVerdict`, `SecurityVerdict`,
//! `RemediationApproval`, and canary objects, and approval-mismatch
//! checks were the caller's responsibility. This module is the real
//! engine: it owns the canonical lifecycle, deduplication, evidence
//! gating, patch-digest binding, and fail-closed verification.
//!
//! Invariants (all enforced by the engine, never by the caller):
//!
//! - Observe -> Incident -> Correlate -> Diagnose -> Reproduce ->
//!   PatchProposed -> SandboxValidation -> SecurityValidation ->
//!   Approval -> StagedDeployment -> PostDeployVerification -> Closed.
//!   The engine rejects skipped, backwards, and non-canonical
//!   transitions; terminal states never move.
//! - A diagnosis ALWAYS starts at `Hypothesis`; only a non-empty
//!   evidence reference may raise confidence, and only through the
//!   canonical escalation order. `Validated` is the only authoritative
//!   confidence.
//! - A sandbox/security verdict with `pass: false` fails closed into
//!   the explicit failure terminal; a passing verdict must carry
//!   evidence and may not be empty.
//! - An approval authorizes the EXACT patch digest recorded for the
//!   incident; a digest mismatch fails closed `Policy`. The engine
//!   checks the binding; the caller never does.
//! - Post-deploy verification closes an incident ONLY when `verified`
//!   is true; false closes nothing and fails closed `Verification`.
//! - Incidents deduplicate by canonical signature while OPEN. A
//!   terminal incident never resurrects.
//!
//! The engine is deterministic: IDs for derived objects (diagnosis,
//! patch, approval) are derived from the incident's correlation id, so
//! retrying the same signal reproduces the same IDs (retry-safe,
//! idempotent) without a random generator dependency.

use std::cell::RefCell;
use std::collections::BTreeMap;

use nexus_domain::{DiagnosisId, IncidentId, Risk};

use crate::contract::{
    DiagnosisTask, Incident, IncidentEngine, IncidentSignal, PatchProposal, RemediationApproval,
    SandboxVerdict, SecurityVerdict,
};
use crate::error::{HealingError, HealingErrorCode};
use crate::memory::{IncidentMemory, IncidentMemoryRecord};
use crate::vocabulary::{DiagnosisConfidence, IncidentState};

/// Derive a child typed id deterministically from a seed UUIDv7 by
/// mutating the final hex digit. The seed keeps version nibble '7' and
/// variant nibble '8'..'b' untouched; only the last char changes, so
/// the result is still a canonical UUIDv7 and distinct from the seed
/// and from every other salt value.
fn derive_id(seed: &str, salt: char) -> String {
    debug_assert!(seed.len() == 36, "seed must be a canonical UUIDv7");
    debug_assert!(
        salt.is_ascii_hexdigit() && !salt.is_ascii_uppercase(),
        "salt must be a lowercase hex digit"
    );
    let mut s = seed.to_string();
    s.pop();
    s.push(salt);
    s
}

/// The production incident engine.
///
/// Owns incident state, deduplication, diagnoses, patch proposals,
/// approvals, and the canonical lifecycle. The `IncidentEngine` port
/// takes `&self`, so the engine uses interior mutability; callers must
/// not hold a borrow across calls.
#[derive(Debug)]
pub struct StandardIncidentEngine<M: IncidentMemory> {
    memory: RefCell<M>,
    /// open incident id by canonical dedup key (deduplication while
    /// the incident is not terminal).
    open: RefCell<BTreeMap<String, IncidentId>>,
    /// incident by id.
    incidents: RefCell<BTreeMap<IncidentId, Incident>>,
    /// diagnosis by incident id (one active diagnosis per incident).
    diagnoses: RefCell<BTreeMap<IncidentId, DiagnosisTask>>,
    /// patch proposal by incident id (one active patch per incident).
    patches: RefCell<BTreeMap<IncidentId, PatchProposal>>,
    /// approval by incident id (bound to the exact patch digest).
    approvals: RefCell<BTreeMap<IncidentId, RemediationApproval>>,
}

impl<M: IncidentMemory> StandardIncidentEngine<M> {
    pub fn new(memory: M) -> Self {
        Self {
            memory: RefCell::new(memory),
            open: RefCell::new(BTreeMap::new()),
            incidents: RefCell::new(BTreeMap::new()),
            diagnoses: RefCell::new(BTreeMap::new()),
            patches: RefCell::new(BTreeMap::new()),
            approvals: RefCell::new(BTreeMap::new()),
        }
    }

    /// Borrow the backing incident memory (test/observability access).
    pub fn memory(&self) -> std::cell::Ref<'_, M> {
        self.memory.borrow()
    }

    /// Canonical transition table: the only allowed forward edges.
    fn canonical_next(from: IncidentState) -> Option<IncidentState> {
        use IncidentState::*;
        match from {
            Observe => Some(Incident),
            Incident => Some(Correlate),
            Correlate => Some(Diagnose),
            Diagnose => Some(Reproduce),
            Reproduce => Some(PatchProposed),
            PatchProposed => Some(SandboxValidation),
            SandboxValidation => Some(SecurityValidation),
            SecurityValidation => Some(Approval),
            Approval => Some(StagedDeployment),
            StagedDeployment => Some(PostDeployVerification),
            PostDeployVerification => Some(Closed),
            // Terminal states never move; failure terminals are set by
            // the explicit verdict/verification paths, not by `to`.
            Closed | Rejected | Unreproducible | ValidationFailed | SecurityFailed | RolledBack
            | Blocked => None,
        }
    }

    /// Validate a canonical forward transition.
    fn check_transition(from: IncidentState, to: IncidentState) -> Result<(), HealingError> {
        if from.is_terminal() {
            return Err(HealingError::conflict(format!(
                "terminal incident state {from} never moves"
            )));
        }
        if Self::canonical_next(from) != Some(to) {
            return Err(HealingError::conflict(format!(
                "non-canonical transition {from} -> {to}"
            )));
        }
        Ok(())
    }

    /// Canonical dedup key for a signal (tenant-scoped; never merges
    /// across tenants, never uses raw error text alone).
    fn dedup_key(signal: &IncidentSignal) -> String {
        M::canonical_dedup_key(&signal.tenant_id, &signal.error_class, &signal.component)
    }

    /// The incident's risk, derived from the signal kind. Security
    /// events and deployment regressions are the highest-risk classes.
    fn risk_for(signal: &IncidentSignal) -> Risk {
        use crate::contract::IncidentSignalKind::*;
        match signal.kind {
            SecurityEvent | DeploymentRegression => Risk::R4,
            ProcessFailure | ResourceExhaustion => Risk::R3,
            HealthFailure | ConnectorFailure => Risk::R2,
            TestFailure | WorkflowFailure => Risk::R1,
        }
    }
}

impl<M: IncidentMemory> IncidentEngine for StandardIncidentEngine<M> {
    fn observe(&self, signal: IncidentSignal) -> Result<Incident, HealingError> {
        // Validate signal shape up front: canonical ids and non-empty
        // canonical text are required before anything is recorded.
        if signal.error_class.trim().is_empty() {
            return Err(HealingError::validation("error_class must not be empty"));
        }
        if signal.component.trim().is_empty() {
            return Err(HealingError::validation("component must not be empty"));
        }
        if signal.first_seen_epoch_ms > signal.last_seen_epoch_ms {
            return Err(HealingError::validation(
                "first_seen must precede last_seen",
            ));
        }

        let key = Self::dedup_key(&signal);

        // Deduplicate while open: an existing open incident with the
        // same canonical signature is returned, never duplicated.
        if let Some(existing_id) = self.open.borrow().get(&key) {
            if let Some(incident) = self.incidents.borrow().get(existing_id) {
                if !incident.state.is_terminal() {
                    return Ok(incident.clone());
                }
            }
        }

        // Deterministic id: the incident is identified by its
        // correlation id (UUIDv7 seed, retry-stable).
        let incident_id = IncidentId::new(signal.correlation_id.as_str())
            .map_err(|e| HealingError::validation(e.to_string()))?;

        let incident = Incident {
            incident_id,
            correlation_id: signal.correlation_id.clone(),
            tenant_id: signal.tenant_id.clone(),
            state: IncidentState::Incident,
            risk: Self::risk_for(&signal),
            components: vec![signal.component.clone()],
            first_seen_epoch_ms: signal.first_seen_epoch_ms,
            last_seen_epoch_ms: signal.last_seen_epoch_ms,
            dedup_key: key.clone(),
        };

        let record = IncidentMemoryRecord {
            incident_id: incident.incident_id.clone(),
            tenant_id: incident.tenant_id.clone(),
            dedup_key: key.clone(),
            error_class: signal.error_class.clone(),
            component: signal.component.clone(),
            final_state: None,
            skill_candidate_ref: None,
        };
        self.memory.borrow_mut().record(record)?;

        self.open
            .borrow_mut()
            .insert(key, incident.incident_id.clone());
        self.incidents
            .borrow_mut()
            .insert(incident.incident_id.clone(), incident.clone());
        Ok(incident)
    }

    fn transition(&self, incident: &mut Incident, to: IncidentState) -> Result<(), HealingError> {
        Self::check_transition(incident.state, to)?;
        let id = incident.incident_id.clone();
        incident.state = to;
        self.incidents.borrow_mut().insert(id, incident.clone());
        Ok(())
    }

    fn create_diagnosis(
        &self,
        incident: &Incident,
        hypothesis: String,
    ) -> Result<DiagnosisTask, HealingError> {
        if incident.state != IncidentState::Diagnose {
            return Err(HealingError::conflict(format!(
                "diagnosis requires DIAGNOSE incident, found {}",
                incident.state
            )));
        }
        if hypothesis.trim().is_empty() {
            return Err(HealingError::validation("hypothesis must not be empty"));
        }
        if self.diagnoses.borrow().contains_key(&incident.incident_id) {
            return Err(HealingError::conflict("one active diagnosis per incident"));
        }
        let diagnosis_id = DiagnosisId::new(derive_id(incident.correlation_id.as_str(), '1'))
            .map_err(|e| HealingError::validation(e.to_string()))?;
        let diagnosis = DiagnosisTask {
            diagnosis_id,
            incident_id: incident.incident_id.clone(),
            tenant_id: incident.tenant_id.clone(),
            correlation_id: incident.correlation_id.clone(),
            hypothesis,
            confidence: DiagnosisConfidence::Hypothesis,
            evidence_refs: vec![],
            attempts: 1,
        };
        self.diagnoses
            .borrow_mut()
            .insert(incident.incident_id.clone(), diagnosis.clone());
        Ok(diagnosis)
    }

    fn update_diagnosis_confidence(
        &self,
        diagnosis: &mut DiagnosisTask,
        confidence: DiagnosisConfidence,
        evidence_ref: String,
    ) -> Result<(), HealingError> {
        // Only real evidence may raise confidence; the canonical
        // escalation order is enforced (never skip to VALIDATED).
        if evidence_ref.trim().is_empty() {
            return Err(HealingError::verification(
                "confidence escalation requires an evidence reference",
            ));
        }
        let order = [
            DiagnosisConfidence::Hypothesis,
            DiagnosisConfidence::Supported,
            DiagnosisConfidence::Reproduced,
            DiagnosisConfidence::Validated,
        ];
        let from = order
            .iter()
            .position(|c| *c == diagnosis.confidence)
            .ok_or_else(|| HealingError::internal("unknown diagnosis confidence"))?;
        let to = order
            .iter()
            .position(|c| *c == confidence)
            .ok_or_else(|| HealingError::internal("unknown target confidence"))?;
        // Escalation is stepwise: confidence may advance exactly one
        // canonical rung per evidence-bearing call. A jump straight to
        // VALIDATED (skipping SUPPORTED/REPRODUCED) would let a model
        // declare its own fix authoritative without the intermediate
        // evidence chain.
        if to != from + 1 {
            return Err(HealingError::conflict(
                "confidence must escalate one canonical rung at a time",
            ));
        }
        diagnosis.confidence = confidence;
        diagnosis.evidence_refs.push(evidence_ref);
        Ok(())
    }

    fn propose_patch(
        &self,
        incident: &Incident,
        proposal: PatchProposal,
    ) -> Result<(), HealingError> {
        if incident.state != IncidentState::Reproduce {
            return Err(HealingError::conflict(format!(
                "patch proposal requires REPRODUCE incident, found {}",
                incident.state
            )));
        }
        if proposal.files_changed.is_empty() {
            return Err(HealingError::validation("patch must declare changed files"));
        }
        if proposal.diff.trim().is_empty() {
            return Err(HealingError::validation("patch diff must not be empty"));
        }
        if proposal.patch_digest.trim().is_empty() {
            return Err(HealingError::validation("patch digest must not be empty"));
        }
        if self.patches.borrow().contains_key(&incident.incident_id) {
            return Err(HealingError::conflict("one active patch per incident"));
        }
        if proposal.incident_id != incident.incident_id || proposal.tenant_id != incident.tenant_id
        {
            return Err(HealingError::authorization(
                "patch proposal must match the incident's tenant and id",
            ));
        }
        self.patches
            .borrow_mut()
            .insert(incident.incident_id.clone(), proposal);
        Ok(())
    }

    fn record_sandbox_validation(
        &self,
        incident: &mut Incident,
        verdict: &SandboxVerdict,
    ) -> Result<(), HealingError> {
        if incident.state != IncidentState::PatchProposed {
            return Err(HealingError::conflict(format!(
                "sandbox validation requires PATCH_PROPOSED incident, found {}",
                incident.state
            )));
        }
        if !verdict.pass {
            incident.state = IncidentState::ValidationFailed;
            self.incidents
                .borrow_mut()
                .insert(incident.incident_id.clone(), incident.clone());
            return Err(HealingError::verification(
                "sandbox validation failed; incident is VALIDATION_FAILED",
            ));
        }
        if verdict.checks.is_empty() || verdict.evidence_ref.trim().is_empty() {
            return Err(HealingError::verification(
                "passing sandbox verdict must carry checks and evidence",
            ));
        }
        let id = incident.incident_id.clone();
        incident.state = IncidentState::SandboxValidation;
        self.incidents.borrow_mut().insert(id, incident.clone());
        Ok(())
    }

    fn record_security_validation(
        &self,
        incident: &mut Incident,
        verdict: &SecurityVerdict,
    ) -> Result<(), HealingError> {
        if incident.state != IncidentState::SandboxValidation {
            return Err(HealingError::conflict(format!(
                "security validation requires SANDBOX_VALIDATION incident, found {}",
                incident.state
            )));
        }
        if !verdict.pass {
            incident.state = IncidentState::SecurityFailed;
            self.incidents
                .borrow_mut()
                .insert(incident.incident_id.clone(), incident.clone());
            return Err(HealingError::verification(
                "security validation failed; incident is SECURITY_FAILED",
            ));
        }
        if verdict.checks.is_empty() || verdict.evidence_ref.trim().is_empty() {
            return Err(HealingError::verification(
                "passing security verdict must carry checks and evidence",
            ));
        }
        let id = incident.incident_id.clone();
        incident.state = IncidentState::SecurityValidation;
        self.incidents.borrow_mut().insert(id, incident.clone());
        Ok(())
    }

    fn record_approval(
        &self,
        incident: &mut Incident,
        approval: &RemediationApproval,
    ) -> Result<(), HealingError> {
        if incident.state != IncidentState::SecurityValidation {
            return Err(HealingError::conflict(format!(
                "approval requires SECURITY_VALIDATION incident, found {}",
                incident.state
            )));
        }
        // The engine owns the binding: an approval authorizes the
        // EXACT patch digest recorded for the incident. A digest
        // mismatch fails closed Policy; the caller never checks this.
        let recorded = self
            .patches
            .borrow()
            .get(&incident.incident_id)
            .ok_or_else(|| HealingError::not_found("no patch recorded for incident"))?
            .patch_digest
            .clone();
        if approval.patch_digest != recorded {
            return Err(HealingError::policy(
                "approval digest does not match the recorded patch digest",
            ));
        }
        if approval.incident_id != incident.incident_id
            || approval.tenant_id != incident.tenant_id
            || approval.correlation_id != incident.correlation_id
        {
            return Err(HealingError::authorization(
                "approval must match the incident identity",
            ));
        }
        if approval.approver.trim().is_empty() {
            return Err(HealingError::validation("approver must not be empty"));
        }
        let id = incident.incident_id.clone();
        self.approvals
            .borrow_mut()
            .insert(id.clone(), approval.clone());
        incident.state = IncidentState::Approval;
        self.incidents.borrow_mut().insert(id, incident.clone());
        Ok(())
    }

    fn record_post_deploy_verification(
        &self,
        incident: &mut Incident,
        verified: bool,
    ) -> Result<(), HealingError> {
        if incident.state != IncidentState::PostDeployVerification {
            return Err(HealingError::conflict(format!(
                "post-deploy verification requires POST_DEPLOY_VERIFICATION incident, found {}",
                incident.state
            )));
        }
        if !verified {
            return Err(HealingError::verification(
                "post-deploy verification failed; incident stays open",
            ));
        }
        let id = incident.incident_id.clone();
        incident.state = IncidentState::Closed;
        self.incidents
            .borrow_mut()
            .insert(id.clone(), incident.clone());
        self.open.borrow_mut().remove(&incident.dedup_key);
        let record = IncidentMemoryRecord {
            incident_id: incident.incident_id.clone(),
            tenant_id: incident.tenant_id.clone(),
            dedup_key: incident.dedup_key.clone(),
            error_class: incident.components.first().cloned().unwrap_or_default(),
            component: incident.components.first().cloned().unwrap_or_default(),
            final_state: Some(IncidentState::Closed.as_str().to_string()),
            skill_candidate_ref: None,
        };
        self.memory.borrow_mut().record_final(record)?;
        Ok(())
    }

    fn code(&self) -> HealingErrorCode {
        HealingErrorCode::Internal
    }
}
