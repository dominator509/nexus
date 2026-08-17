//! EP-024 deterministic device capability mapper (SPEC-011).
//!
//! Provider domain names are normalized at the infrastructure boundary
//! and never become domain contracts. The mapper deterministically maps
//! a canonical capability key to the EP-010 capability taxonomy
//! (CapabilityClass / Risk / ApprovalClass / Idempotency), reusing the
//! nexus-home `DeviceCapability` shape so the whole device plane speaks
//! one vocabulary. Unknown keys are rejected; nothing is invented.

use nexus_domain::{ApprovalClass, CapabilityClass, Idempotency, Risk};
use serde::{Deserialize, Serialize};

use crate::error::{DevicesError, DevicesErrorCode};

/// Canonical device capability (SPEC-011 canonical term
/// DeviceCapability), reusing the EP-010 capability taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapability {
    /// Canonical capability key (`^[a-z][a-z0-9_.-]+$`).
    pub capability_id: String,
    /// Capability class from the EP-010 taxonomy.
    pub class: CapabilityClass,
    /// Risk class of the capability's actions (EP-008 policy input).
    pub risk: Risk,
    /// Approval class required before execution.
    pub approval: ApprovalClass,
    /// Idempotency contract for retryable commands (SPEC-006).
    pub idempotency: Idempotency,
}

/// Deterministic capability mapper.
///
/// The mapping table is closed: every canonical key the platform knows
/// maps to the same taxonomy every time. An unknown key is an error,
/// never a silently fabricated capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeviceCapabilityMapper;

impl DeviceCapabilityMapper {
    /// Map a canonical capability key to the EP-010 taxonomy.
    pub fn map(&self, capability_id: &str) -> Result<DeviceCapability, DevicesError> {
        let (class, risk, approval, idempotency) = match capability_id {
            "media.playback" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "media.volume" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "media.source" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "media.power" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "appliance.power" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "appliance.mode" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "appliance.status" => (
                CapabilityClass::Query,
                Risk::R0,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "irrigation.zone" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "irrigation.schedule" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "irrigation.moisture" => (
                CapabilityClass::Query,
                Risk::R0,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "vacuum.dock" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "vacuum.clean" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "vacuum.pause" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "vacuum.home" => (
                CapabilityClass::Command,
                Risk::R1,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "vacuum.map" => (
                CapabilityClass::Query,
                Risk::R0,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "robot.navigation" => (
                CapabilityClass::Command,
                Risk::R3,
                ApprovalClass::Human,
                Idempotency::Required,
            ),
            "robot.manipulation" => (
                CapabilityClass::Command,
                Risk::R3,
                ApprovalClass::Human,
                Idempotency::Required,
            ),
            "robot.sensing" => (
                CapabilityClass::Query,
                Risk::R0,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "robot.safety_interlock" => (
                CapabilityClass::Query,
                Risk::R0,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "robot.emergency_stop" => (
                CapabilityClass::Query,
                Risk::R0,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            "robot.human_presence" => (
                CapabilityClass::Query,
                Risk::R0,
                ApprovalClass::None,
                Idempotency::Required,
            ),
            _ => {
                return Err(DevicesError::new(
                    DevicesErrorCode::Vocabulary,
                    format!("unknown canonical capability key {capability_id:?}"),
                    None,
                    None,
                ))
            }
        };
        Ok(DeviceCapability {
            capability_id: capability_id.to_string(),
            class,
            risk,
            approval,
            idempotency,
        })
    }
}
