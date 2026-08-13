//! Risk classification (SPEC-005/SPEC-006; R0..R4).
//!
//! SPEC-005 behavior 4: R3 and R4 actions require a cryptographic
//! step-up or explicit preauthorization; R4 never accepts model
//! approval. The `RiskClassifier` port maps action descriptors to a
//! risk class. This crate defines the deterministic classification
//! function; providers may extend it but cannot lower a class below
//! the deterministic floor.

use std::fmt;

use nexus_auth::AuthenticationStrength;
use nexus_domain::{CapabilityClass, Reversal, Risk};
use serde::{Deserialize, Serialize};

/// Re-export the canonical risk class (SPEC-006 `R0`..`R4`).
pub use nexus_domain::Risk as RiskClass;

/// Deterministic risk ordering (R0 < R1 < ... < R4).
///
/// The locked domain `Risk` enum does not derive `Ord`; this is the
/// canonical rank function owned by EP-008 so the gateway can compare
/// classes without widening the domain vocabulary.
pub const fn risk_rank(risk: Risk) -> u8 {
    match risk {
        Risk::R0 => 0,
        Risk::R1 => 1,
        Risk::R2 => 2,
        Risk::R3 => 3,
        Risk::R4 => 4,
    }
}

/// Whether `a` is at or above `threshold` in the R0..R4 ladder.
pub const fn risk_at_least(risk: Risk, threshold: Risk) -> bool {
    risk_rank(risk) >= risk_rank(threshold)
}

/// Inputs for risk classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskAssessmentInput {
    /// Capability class of the requested action.
    pub capability: CapabilityClass,
    /// Reversal class: irreversible actions are never below R3.
    pub reversal: Reversal,
    /// Whether the action touches secret or security-class data.
    pub touches_secret: bool,
    /// Current authentication strength (used to reject step-up gaps).
    pub strength: AuthenticationStrength,
}

impl RiskAssessmentInput {
    /// Construct a risk assessment input.
    pub fn new(
        capability: CapabilityClass,
        reversal: Reversal,
        touches_secret: bool,
        strength: AuthenticationStrength,
    ) -> Self {
        Self {
            capability,
            reversal,
            touches_secret,
            strength,
        }
    }
}

/// Risk-classification errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskClassifierError {
    /// Classification produced an invalid class (never happens with the
    /// deterministic floor; reserved for provider overrides).
    InvalidClass,
}

impl fmt::Display for RiskClassifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid risk class produced by classifier")
    }
}

impl std::error::Error for RiskClassifierError {}

/// Provider-neutral risk classification port.
pub trait RiskClassifier {
    /// Classify the action into R0..R4.
    fn classify(&self, input: &RiskAssessmentInput) -> Result<Risk, RiskClassifierError>;
}

/// Deterministic risk floor (SPEC-005/SPEC-006).
///
/// - `QUERY` -> R0, unless it touches secret data -> R2.
/// - `STREAM` -> R1, unless secret -> R2.
/// - `COMMAND` -> R2 by default; irreversible (Reversal::Irreversible)
///   raises to R3; secret-touching raises to R3.
/// - `WORKFLOW` -> R2; irreversible or secret -> R3.
/// - `ADMINISTRATIVE` -> R3 by default; irreversible -> R4.
///
/// The floor never lowers a class below these bounds; provider
/// classifiers may only raise.
pub fn deterministic_risk_floor(input: &RiskAssessmentInput) -> Result<Risk, RiskClassifierError> {
    let base = match input.capability {
        CapabilityClass::Query => {
            if input.touches_secret {
                Risk::R2
            } else {
                Risk::R0
            }
        }
        CapabilityClass::Stream => {
            if input.touches_secret {
                Risk::R2
            } else {
                Risk::R1
            }
        }
        CapabilityClass::Command | CapabilityClass::Workflow => {
            if input.touches_secret {
                Risk::R3
            } else {
                Risk::R2
            }
        }
        CapabilityClass::Administrative => Risk::R3,
    };
    let raised = match input.reversal {
        Reversal::None => base,
        Reversal::Compensating => base,
        Reversal::Snapshot => base,
        Reversal::Irreversible => match base {
            Risk::R0 | Risk::R1 | Risk::R2 => Risk::R3,
            Risk::R3 => Risk::R4,
            Risk::R4 => Risk::R4,
        },
    };
    Ok(raised)
}

/// A pure, deterministic classifier using the floor.
pub struct DeterministicRiskClassifier;

impl RiskClassifier for DeterministicRiskClassifier {
    fn classify(&self, input: &RiskAssessmentInput) -> Result<Risk, RiskClassifierError> {
        deterministic_risk_floor(input)
    }
}
