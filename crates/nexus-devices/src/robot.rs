//! EP-024 robot safety declarations and activation gating (SPEC-011
//! behavior 6).
//!
//! Robot capabilities declare physical workspace, speed, force, safety
//! interlocks, emergency stop, human presence, and approval class
//! BEFORE activation. A robot is never activated for a capability it
//! did not declare, and motion without an emergency-stop-capable
//! declaration is refused.

use serde::{Deserialize, Serialize};

use crate::error::{DevicesError, DevicesErrorCode};
use crate::vocabulary::RobotCapability;

/// Robot safety declaration (SPEC-011 behavior 6).
///
/// Every field is required and validated at construction. A robot may
/// be activated only for declared capabilities; it never receives
/// broader authority than declared (acceptance obligation 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RobotSafetyDeclaration {
    /// Physical workspace description (bounded string, never a free
    /// authority grant).
    pub workspace: String,
    /// Maximum declared speed in meters per second.
    pub max_speed_mps: f64,
    /// Maximum declared force in newtons.
    pub max_force_n: f64,
    /// Safety interlock names present on the robot (bounded list).
    pub safety_interlocks: Vec<String>,
    /// True only when an emergency stop is present and certified.
    pub emergency_stop: bool,
    /// True only when human presence detection is present and
    /// certified.
    pub human_presence_detection: bool,
    /// Approval class required before activation (EP-008 policy input).
    pub approval_class: crate::ApprovalClass,
    /// Declared capabilities. Activation for any other capability is
    /// refused.
    pub declared_capabilities: Vec<RobotCapability>,
}

impl RobotSafetyDeclaration {
    /// All eight fields are required parts of the safety declaration;
    /// a builder would allow constructing incomplete declarations.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace: impl Into<String>,
        max_speed_mps: f64,
        max_force_n: f64,
        safety_interlocks: Vec<String>,
        emergency_stop: bool,
        human_presence_detection: bool,
        approval_class: crate::ApprovalClass,
        declared_capabilities: Vec<RobotCapability>,
    ) -> Result<Self, DevicesError> {
        let workspace = workspace.into();
        if workspace.is_empty() || workspace.len() > 256 {
            return Err(DevicesError::new(
                DevicesErrorCode::Validation,
                "robot workspace must be 1..=256 characters",
                None,
                None,
            ));
        }
        if !max_speed_mps.is_finite() || max_speed_mps < 0.0 {
            return Err(DevicesError::new(
                DevicesErrorCode::Validation,
                "robot max speed must be a finite non-negative value",
                None,
                None,
            ));
        }
        if !max_force_n.is_finite() || max_force_n < 0.0 {
            return Err(DevicesError::new(
                DevicesErrorCode::Validation,
                "robot max force must be a finite non-negative value",
                None,
                None,
            ));
        }
        if safety_interlocks.len() > 64 {
            return Err(DevicesError::new(
                DevicesErrorCode::Validation,
                "robot safety interlock list is bounded to 64 entries",
                None,
                None,
            ));
        }
        if declared_capabilities.is_empty() {
            return Err(DevicesError::new(
                DevicesErrorCode::Validation,
                "robot must declare at least one capability before activation",
                None,
                None,
            ));
        }
        Ok(Self {
            workspace,
            max_speed_mps,
            max_force_n,
            safety_interlocks,
            emergency_stop,
            human_presence_detection,
            approval_class,
            declared_capabilities,
        })
    }

    /// True when the robot declares the given capability.
    pub fn declares(&self, capability: RobotCapability) -> bool {
        self.declared_capabilities.contains(&capability)
    }

    /// Refuse activation of a capability the robot did not declare.
    ///
    /// A future robot never receives broader authority than declared
    /// capabilities (acceptance obligation 4).
    pub fn ensure_declared(&self, capability: RobotCapability) -> Result<(), DevicesError> {
        if self.declares(capability) {
            Ok(())
        } else {
            Err(DevicesError::new(
                DevicesErrorCode::Policy,
                format!(
                    "robot activation refused: capability {} not declared",
                    capability.as_str()
                ),
                None,
                None,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::RobotCapability;

    #[test]
    fn robot_safety_declaration_requires_declared_capability() {
        let declaration = RobotSafetyDeclaration::new(
            "kitchen floor",
            0.5,
            5.0,
            vec!["bumper".into()],
            true,
            true,
            crate::ApprovalClass::Human,
            vec![RobotCapability::Navigation],
        )
        .expect("valid declaration");
        assert!(declaration.declares(RobotCapability::Navigation));
        assert!(declaration
            .ensure_declared(RobotCapability::Navigation)
            .is_ok());
        let error = declaration
            .ensure_declared(RobotCapability::Manipulation)
            .expect_err("undeclared refused");
        assert_eq!(error.code, DevicesErrorCode::Policy);
    }
}
