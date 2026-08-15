//! EP-014 reflex vocabulary (SPEC-009; ADR-021).
//!
//! Vocabulary-locked canonical classes owned by the reflex plane.
//! Every enum parses from its canonical SCREAMING_SNAKE_CASE wire
//! string and rejects unknown values (fail closed at the boundary).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Vocabulary error for the reflex classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflexVocabularyError(pub String);

impl fmt::Display for ReflexVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ReflexVocabularyError {}

impl ReflexVocabularyError {
    pub fn unknown(class: &str, value: &str) -> Self {
        Self(format!("unknown {class} value: {value}"))
    }
}

macro_rules! vocabulary_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $wire:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = ReflexVocabularyError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($wire => Ok(Self::$variant),)+
                    other => Err(ReflexVocabularyError::unknown(stringify!($name), other)),
                }
            }
        }
    };
}

vocabulary_enum! {
    /// How a reflex decision was produced (SPEC-009; ADR-021).
    ///
    /// `DETERMINISTIC` means the model was bypassed: the task was
    /// resolved by deterministic rules only. `MODEL` means the decision
    /// came from a real provider and passed NexusControlObject
    /// validation before it was returned.
    ReflexDecisionClass {
        Deterministic = "DETERMINISTIC",
        Model = "MODEL",
    }
}

vocabulary_enum! {
    /// Effort selection policy class (SPEC-009 required behavior 2;
    /// ADR-021).
    ///
    /// `POLICY_SELECTED` means the tier was chosen by the deterministic
    /// `EffortPolicy` from request attributes. `EXPLICIT` means the
    /// caller supplied the tier directly. MAX is never the default for
    /// trivial work.
    EffortSelectionClass {
        PolicySelected = "POLICY_SELECTED",
        Explicit = "EXPLICIT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep014_unit_reflex_decision_class_round_trip() {
        for (wire, expected) in [
            ("DETERMINISTIC", ReflexDecisionClass::Deterministic),
            ("MODEL", ReflexDecisionClass::Model),
        ] {
            assert_eq!(wire.parse::<ReflexDecisionClass>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("UNKNOWN".parse::<ReflexDecisionClass>().is_err());
        assert!("".parse::<ReflexDecisionClass>().is_err());
    }

    #[test]
    fn ep014_unit_effort_selection_class_round_trip() {
        for (wire, expected) in [
            ("POLICY_SELECTED", EffortSelectionClass::PolicySelected),
            ("EXPLICIT", EffortSelectionClass::Explicit),
        ] {
            assert_eq!(wire.parse::<EffortSelectionClass>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("AUTO".parse::<EffortSelectionClass>().is_err());
    }
}
