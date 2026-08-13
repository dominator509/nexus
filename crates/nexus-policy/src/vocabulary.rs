//! EP-008 authorization policy vocabulary (SPEC-005, SPEC-006; ADR-012).
//!
//! These enums encode the vocabulary-locked classes owned by this node.
//! Every enum parses from its canonical string and rejects unknown values
//! (SPEC-005/SPEC-006 "Canonical terms"). Names are locked; a new synonym
//! requires an ADR and a schema update.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Error returned when a vocabulary string is not a known canonical class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyVocabularyError(pub String);

impl fmt::Display for PolicyVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown canonical policy class: {}", self.0)
    }
}

impl std::error::Error for PolicyVocabularyError {}

macro_rules! policy_vocabulary_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            /// Canonical wire string for this class.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = PolicyVocabularyError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(PolicyVocabularyError(other.to_string())),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = PolicyVocabularyError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

policy_vocabulary_enum! {
    /// Action lifecycle state (SPEC-006 behavior 4). Every consequential
    /// action moves through this deterministic lifecycle; the Action
    /// Gateway and receipts reference the state at each boundary.
    ActionLifecycleState {
        Requested = "REQUESTED",
        Evaluated = "EVALUATED",
        AwaitingApproval = "AWAITING_APPROVAL",
        Approved = "APPROVED",
        Executing = "EXECUTING",
        Verifying = "VERIFYING",
        Succeeded = "SUCCEEDED",
        Failed = "FAILED",
        Compensating = "COMPENSATING",
        Compensated = "COMPENSATED",
        Rejected = "REJECTED",
    }
}
