//! EP-024 robotics connector (SPEC-011 behavior 6).
//!
//! The robotics connector owns the RobotProvider boundary for future
//! robots. No robot hardware is present on this host, so the host
//! truthfully reports an empty inventory and UNAVAILABLE capabilities:
//! a robot is never advertised, activated, or moved without real
//! certification evidence (Reality rule; acceptance obligation 4: a
//! future robot receives no broader authority than declared
//! capabilities).
//!
//! The safety declaration gates activation: an operator must first
//! supply a `RobotSafetyDeclaration` (workspace, speed, force, safety
//! interlocks, emergency stop, human presence, approval class) and the
//! requested capability must be declared before any activation is
//! permitted. Without real hardware the host can only refuse; this is
//! the honest fail-closed behavior until certification.

#![forbid(unsafe_code)]

use nexus_devices::error::{DevicesError, DevicesErrorCode};
use nexus_devices::robot::RobotSafetyDeclaration;
use nexus_devices::vocabulary::{DeviceAvailability, RobotCapability, RobotId};
use nexus_devices::RobotProvider;

/// Fail-closed robot provider host.
///
/// Inventory is empty and capabilities are UNAVAILABLE until real
/// hardware certification evidence exists. The host still validates
/// safety declarations and capability gating so the activation path is
/// real and fails safely even before any robot is bound.
#[derive(Debug, Clone, Copy, Default)]
pub struct RobotProviderHost;

impl RobotProviderHost {
    /// Validate that an activation request is within the declared
    /// capabilities and the declaration's approval class.
    ///
    /// This is the pure gating rule; with no hardware bound the host
    /// still refuses any activation with Unavailable, but the rule is
    /// proven here so a future bound robot cannot bypass it.
    pub fn validate_activation(
        &self,
        declaration: &RobotSafetyDeclaration,
        capability: RobotCapability,
    ) -> Result<(), DevicesError> {
        declaration.ensure_declared(capability)
    }

    /// Refuse activation: no robot hardware is bound.
    pub fn activate(
        &self,
        _robot: &RobotId,
        _capability: RobotCapability,
    ) -> Result<(), DevicesError> {
        Err(DevicesError::new(
            DevicesErrorCode::Unavailable,
            "robot provider has no hardware bound; activation refused",
            None,
            None,
        ))
    }
}

impl RobotProvider for RobotProviderHost {
    fn list_robots(&self) -> Result<Vec<RobotId>, DevicesError> {
        // No hardware certified on this host; never fabricate an
        // inventory entry (Reality rule).
        Ok(Vec::new())
    }

    fn declared_capabilities(&self, robot: &RobotId) -> Result<Vec<RobotCapability>, DevicesError> {
        let _ = robot;
        Err(DevicesError::new(
            DevicesErrorCode::Unavailable,
            "robot provider has no hardware bound",
            None,
            None,
        ))
    }

    fn availability(&self, robot: &RobotId) -> Result<DeviceAvailability, DevicesError> {
        let _ = robot;
        Ok(DeviceAvailability::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_devices::ApprovalClass;

    #[test]
    fn robotics_host_inventory_never_fabricated() {
        let host = RobotProviderHost;
        assert!(host.list_robots().expect("empty inventory").is_empty());
    }

    #[test]
    fn robotics_host_availability_unavailable() {
        let host = RobotProviderHost;
        let robot = RobotId::new("robo-1").expect("id");
        assert_eq!(
            host.availability(&robot).expect("availability"),
            DeviceAvailability::Unavailable
        );
    }

    #[test]
    fn robotics_host_declared_capabilities_unavailable() {
        let host = RobotProviderHost;
        let robot = RobotId::new("robo-1").expect("id");
        assert_eq!(
            host.declared_capabilities(&robot)
                .expect_err("no hardware bound")
                .code,
            DevicesErrorCode::Unavailable
        );
    }

    #[test]
    fn robotics_host_activation_refused_without_hardware() {
        let host = RobotProviderHost;
        let robot = RobotId::new("robo-1").expect("id");
        assert_eq!(
            host.activate(&robot, RobotCapability::Navigation)
                .expect_err("activation refused")
                .code,
            DevicesErrorCode::Unavailable
        );
    }

    #[test]
    fn robotics_host_safety_gate_rejects_undeclared() {
        let host = RobotProviderHost;
        let declaration = RobotSafetyDeclaration::new(
            "workshop",
            0.5,
            5.0,
            vec!["bumper".to_string()],
            true,
            true,
            ApprovalClass::Human,
            vec![RobotCapability::Navigation],
        )
        .expect("valid declaration");
        assert!(host
            .validate_activation(&declaration, RobotCapability::Navigation)
            .is_ok());
        assert_eq!(
            host.validate_activation(&declaration, RobotCapability::Manipulation)
                .expect_err("undeclared refused")
                .code,
            DevicesErrorCode::Policy
        );
    }

    #[test]
    fn robotics_host_safety_gate_requires_declaration() {
        let host = RobotProviderHost;
        let declaration = RobotSafetyDeclaration::new(
            "workshop",
            0.5,
            5.0,
            vec![],
            true,
            true,
            ApprovalClass::Human,
            vec![],
        )
        .expect_err("no capabilities rejected");
        assert_eq!(declaration.code, DevicesErrorCode::Validation);
        let _ = host;
    }
}
