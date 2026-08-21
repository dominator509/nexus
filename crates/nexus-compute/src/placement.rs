//! EP-036 placement policy (SPEC-016 Compute Fabric).
//!
//! Placement is constraint-based, never provider-name-based. Eligibility
//! requires every explicit constraint: minimum CPU/RAM/disk, required
//! architecture, required accelerator, locality, privacy boundary, tenant
//! boundary, allowed classes, allowed regions, and cost ceiling. If no
//! eligible target exists the placement FAILS CLOSED; it never silently
//! downgrades a constraint (e.g. LOCAL_ONLY workload moved to public
//! cloud) to place the workload.

use crate::error::{ComputeError, ComputeResult};
use crate::model::{
    ComputeNode, PlacementConstraint, PlacementDecision, ProvisioningRequestId, WorkloadManifestId,
};
use crate::vocabulary::PlacementFailureClass;

/// Result of evaluating a constraint against a set of nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementEvaluation {
    pub eligible: Vec<ComputeNode>,
    pub selected: Option<ComputeNode>,
    pub failure_class: Option<PlacementFailureClass>,
}

impl PlacementEvaluation {
    pub fn is_assigned(&self) -> bool {
        self.selected.is_some()
    }
}

/// Constraint-based placement decision. Returns a decision; when no
/// eligible target exists the decision records the failure class and
/// fails closed (no silent constraint downgrade).
pub fn placement_decision(
    request_id: ProvisioningRequestId,
    manifest_id: WorkloadManifestId,
    constraint: &PlacementConstraint,
    nodes: &[ComputeNode],
) -> ComputeResult<PlacementDecision> {
    let mut eligible: Vec<ComputeNode> = Vec::new();
    let mut first_failure: Option<PlacementFailureClass> = None;

    for node in nodes {
        match constraint.evaluate(node) {
            Ok(()) => eligible.push(node.clone()),
            Err(err) => {
                if first_failure.is_none() {
                    first_failure = Some(classify_failure(err));
                }
            }
        }
    }

    if eligible.is_empty() {
        return Ok(PlacementDecision::rejected(
            request_id,
            manifest_id,
            first_failure.unwrap_or(PlacementFailureClass::NoEligibleTarget),
        ));
    }

    // Deterministic choice: prefer the least-powerful eligible node
    // (smallest cpu_cores, then memory) so the fabric does not waste
    // capacity and does not escalate to GPU-class nodes unless required.
    eligible.sort_by(|a, b| {
        let ac = (
            a.declared_capacity.cpu_cores,
            a.declared_capacity.memory_gib,
        );
        let bc = (
            b.declared_capacity.cpu_cores,
            b.declared_capacity.memory_gib,
        );
        ac.cmp(&bc)
    });

    let selected = eligible[0].clone();
    Ok(PlacementDecision::assigned(
        request_id,
        manifest_id,
        selected.node_id,
    ))
}

fn classify_failure(err: ComputeError) -> PlacementFailureClass {
    use crate::error::ComputeErrorCode;
    match err.code {
        ComputeErrorCode::Policy => {
            let msg = err.message.to_ascii_lowercase();
            if msg.contains("tenant") {
                PlacementFailureClass::TenantBoundaryViolation
            } else if msg.contains("privacy") {
                PlacementFailureClass::PrivacyBoundaryViolation
            } else if msg.contains("gpu") || msg.contains("architecture") {
                PlacementFailureClass::ConstraintUnsatisfiable
            } else if msg.contains("budget") {
                PlacementFailureClass::BudgetExceeded
            } else {
                PlacementFailureClass::NoEligibleTarget
            }
        }
        _ => PlacementFailureClass::ConstraintUnsatisfiable,
    }
}
