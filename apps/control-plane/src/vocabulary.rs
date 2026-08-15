//! EP-044 runtime vocabulary (ADR-019).
//!
//! Canonical state classes for the Control Plane Runtime. Values parse
//! from canonical SCREAMING_SNAKE_CASE strings and reject unknowns, so
//! wire and storage forms stay stable (SPEC-005 vocabulary law).

use serde::{Deserialize, Serialize};

/// Vocabulary parse/rejection error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVocabularyError(pub String);

impl std::fmt::Display for RuntimeVocabularyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid runtime vocabulary: {}", self.0)
    }
}

impl std::error::Error for RuntimeVocabularyError {}

macro_rules! runtime_vocabulary_enum {
    (
        $(#[$meta:meta])*
        $name:ident {
            $($variant:ident = $wire:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            /// Canonical wire form.
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }

            /// Parse the canonical wire form; unknown values are rejected.
            pub fn parse(value: &str) -> Result<Self, RuntimeVocabularyError> {
                match value {
                    $($wire => Ok(Self::$variant),)+
                    other => Err(RuntimeVocabularyError(format!(
                        "unknown {}: {}",
                        stringify!($name),
                        other
                    ))),
                }
            }
        }

        impl std::str::FromStr for $name {
            type Err = RuntimeVocabularyError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

runtime_vocabulary_enum! {
    /// Runtime lifecycle state (ADR-019).
    RuntimeState {
        Starting = "STARTING",
        Ready = "READY",
        Degraded = "DEGRADED",
        Stopping = "STOPPING",
        Stopped = "STOPPED",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep044_unit_vocabulary_round_trip() {
        assert_eq!(RuntimeState::Ready.as_str(), "READY");
        assert_eq!(RuntimeState::parse("READY"), Ok(RuntimeState::Ready));
        assert_eq!("READY".parse::<RuntimeState>(), Ok(RuntimeState::Ready));
        assert_eq!(RuntimeState::Starting.to_string(), "STARTING");
    }

    #[test]
    fn ep044_unit_vocabulary_rejects_unknown() {
        let err = RuntimeState::parse("HALTED").unwrap_err();
        assert!(err.0.contains("unknown RuntimeState"));
    }

    #[test]
    fn ep044_unit_vocabulary_serde_screaming_snake() {
        let wire = serde_json::to_string(&RuntimeState::Degraded).unwrap();
        assert_eq!(wire, "\"DEGRADED\"");
        let parsed: RuntimeState = serde_json::from_str("\"STOPPING\"").unwrap();
        assert_eq!(parsed, RuntimeState::Stopping);
    }
}
