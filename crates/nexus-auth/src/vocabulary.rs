//! EP-007 authentication vocabulary (SPEC-005; ADR-011).
//!
//! These enums encode the vocabulary-locked classes owned by this node.
//! Every enum parses from its canonical string and rejects unknown values
//! (SPEC-005 "Canonical terms"). Names are locked; a new synonym requires
//! an ADR and a schema update.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Error returned when a vocabulary string is not a known canonical class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthVocabularyError(pub String);

impl fmt::Display for AuthVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown canonical auth class: {}", self.0)
    }
}

impl std::error::Error for AuthVocabularyError {}

macro_rules! auth_vocabulary_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
            type Err = AuthVocabularyError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(AuthVocabularyError(other.to_string())),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = AuthVocabularyError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

auth_vocabulary_enum! {
    /// Authentication strength of a completed sign-in (SPEC-005; ADR-010).
    ///
    /// `NONE` means no authentication; `SINGLE_FACTOR` is one possession
    /// or knowledge factor; `MULTI_FACTOR` is two or more independent
    /// factors; `STEP_UP` is a cryptographic step-up (passkey/WebAuthn,
    /// hardware key, or explicit re-authentication). SPEC-005 behavior 4:
    /// R3 and R4 actions require STEP_UP (or explicit preauthorization);
    /// R4 never accepts model approval.
    AuthenticationStrength {
        None = "NONE",
        SingleFactor = "SINGLE_FACTOR",
        MultiFactor = "MULTI_FACTOR",
        StepUp = "STEP_UP",
    }
}

auth_vocabulary_enum! {
    /// Token class issued by the OIDC provider (SPEC-005).
    ///
    /// `ACCESS` tokens are short-lived bearer tokens scoped to a resource;
    /// `REFRESH` tokens are rotation-only credentials (never used as
    /// bearer); `ID` tokens carry identity claims.
    TokenClass {
        Access = "ACCESS",
        Refresh = "REFRESH",
        Id = "ID",
    }
}

auth_vocabulary_enum! {
    /// Passkey enrollment lifecycle state (SPEC-005; WebAuthn).
    PasskeyState {
        PendingChallenge = "PENDING_CHALLENGE",
        Registered = "REGISTERED",
        Revoked = "REVOKED",
    }
}

auth_vocabulary_enum! {
    /// Device enrollment lifecycle state (SPEC-005).
    DeviceEnrollmentState {
        PendingVerification = "PENDING_VERIFICATION",
        Enrolled = "ENROLLED",
        Rejected = "REJECTED",
        Revoked = "REVOKED",
    }
}

auth_vocabulary_enum! {
    /// Step-up challenge lifecycle state (SPEC-005 behavior 4).
    StepUpState {
        Pending = "PENDING",
        Satisfied = "SATISFIED",
        Expired = "EXPIRED",
        Cancelled = "CANCELLED",
    }
}

auth_vocabulary_enum! {
    /// Recovery kit material kind (SPEC-005 behavior 6; offline recovery).
    RecoveryMaterialKind {
        SealedEnvelope = "SEALED_ENVELOPE",
        SplitShares = "SPLIT_SHARES",
        RecoveryCode = "RECOVERY_CODE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep007_unit_auth_vocabulary_parses_all_strengths() {
        for (text, expected) in [
            ("NONE", AuthenticationStrength::None),
            ("SINGLE_FACTOR", AuthenticationStrength::SingleFactor),
            ("MULTI_FACTOR", AuthenticationStrength::MultiFactor),
            ("STEP_UP", AuthenticationStrength::StepUp),
        ] {
            assert_eq!(text.parse::<AuthenticationStrength>().unwrap(), expected);
            assert_eq!(expected.as_str(), text);
        }
    }

    #[test]
    fn ep007_unit_auth_vocabulary_rejects_unknown_strength() {
        assert_eq!(
            "BIO".parse::<AuthenticationStrength>(),
            Err(AuthVocabularyError("BIO".to_string()))
        );
        assert_eq!(
            "".parse::<AuthenticationStrength>(),
            Err(AuthVocabularyError("".to_string()))
        );
    }

    #[test]
    fn ep007_unit_auth_vocabulary_roundtrips_serde() {
        let strength = AuthenticationStrength::StepUp;
        let json = serde_json::to_string(&strength).unwrap();
        assert_eq!(json, "\"STEP_UP\"");
        let back: AuthenticationStrength = serde_json::from_str(&json).unwrap();
        assert_eq!(back, strength);
    }

    #[test]
    fn ep007_unit_auth_vocabulary_serde_rejects_unknown() {
        let res: Result<AuthenticationStrength, _> = serde_json::from_str("\"BIOMETRIC\"");
        assert!(res.is_err());
    }

    #[test]
    fn ep007_unit_auth_vocabulary_covers_all_classes() {
        assert_eq!(TokenClass::Refresh.as_str(), "REFRESH");
        assert_eq!(PasskeyState::Registered.as_str(), "REGISTERED");
        assert_eq!(DeviceEnrollmentState::Enrolled.as_str(), "ENROLLED");
        assert_eq!(StepUpState::Satisfied.as_str(), "SATISFIED");
        assert_eq!(RecoveryMaterialKind::SplitShares.as_str(), "SPLIT_SHARES");
    }

    #[test]
    fn ep007_unit_auth_vocabulary_no_vendor_brand_leaks() {
        let all = [
            AuthenticationStrength::StepUp.as_str(),
            TokenClass::Id.as_str(),
            PasskeyState::Registered.as_str(),
            DeviceEnrollmentState::Enrolled.as_str(),
            StepUpState::Pending.as_str(),
            RecoveryMaterialKind::RecoveryCode.as_str(),
        ];
        for value in all {
            let lower = value.to_ascii_lowercase();
            for brand in [
                "keycloak",
                "auth0",
                "okta",
                "cognito",
                "apple",
                "google",
                "microsoft",
                "fido",
            ] {
                assert!(
                    !lower.contains(brand),
                    "canonical class {value} leaks provider brand {brand}"
                );
            }
        }
    }
}
