//! EP-009 trust vocabulary (SPEC-005, SPEC-020; ADR-013).
//!
//! These enums encode the vocabulary-locked classes owned by this node.
//! Every enum parses from its canonical string and rejects unknown values
//! (SPEC-005/SPEC-020 "Canonical terms"). Names are locked; a new synonym
//! requires an ADR and a schema update.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Error returned when a vocabulary string is not a known canonical class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustZoneError(pub String);

impl fmt::Display for TrustZoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown canonical trust class: {}", self.0)
    }
}

impl std::error::Error for TrustZoneError {}

macro_rules! trust_vocabulary_enum {
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
            type Err = TrustZoneError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(TrustZoneError(other.to_string())),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = TrustZoneError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

trust_vocabulary_enum! {
    /// Network trust zone for services, devices, and mesh nodes
    /// (SPEC-020; SPEC-005 behavior 7; ADR-013). Every service, device,
    /// and mesh node belongs to exactly one zone; zone boundaries
    /// determine mTLS policy, WireGuard segment membership, and secret
    /// exposure.
    TrustZone {
        Public = "PUBLIC",
        Guest = "GUEST",
        Local = "LOCAL",
        PrivateMesh = "PRIVATE_MESH",
    }
}

trust_vocabulary_enum! {
    /// Capability token lifecycle (SPEC-005 behavior 5; ADR-013). Tokens
    /// are short-lived, audience restricted, resource restricted, action
    /// restricted, and non-transferable. A token never outlives its
    /// expiry; `REVOKED` and `EXPIRED` are terminal.
    TokenState {
        Active = "ACTIVE",
        Revoked = "REVOKED",
        Expired = "EXPIRED",
    }
}

trust_vocabulary_enum! {
    /// Secret lifecycle (SPEC-005 behavior 6; ADR-013). Secrets are
    /// referenced by name and never enter model context. `ROTATING`
    /// means a new version is being installed; `REVOKED` means the
    /// reference no longer resolves.
    SecretState {
        Active = "ACTIVE",
        Rotating = "ROTATING",
        Revoked = "REVOKED",
    }
}

trust_vocabulary_enum! {
    /// mTLS certificate lifecycle (SPEC-005 behavior 7; ADR-013).
    /// Certificates are short-lived; `EXPIRED` is terminal after
    /// `not_after`, `REVOKED` is terminal before `not_after`.
    CertificateState {
        Active = "ACTIVE",
        Expired = "EXPIRED",
        Revoked = "REVOKED",
    }
}

trust_vocabulary_enum! {
    /// Service identity lifecycle (ADR-013). A service identity is the
    /// canonical service principal bound to an mTLS certificate;
    /// `SUSPENDED` stops new issuance without destroying the record,
    /// `REVOKED` terminates it.
    ServiceIdentityState {
        Active = "ACTIVE",
        Suspended = "SUSPENDED",
        Revoked = "REVOKED",
    }
}

trust_vocabulary_enum! {
    /// Mesh node lifecycle (ADR-013). `PENDING` means a node requested
    /// membership but is not yet registered; `REGISTERED` means it holds
    /// a WireGuard key pair and can connect; `ONLINE`/`OFFLINE` are
    /// operational observations; `REVOKED` is terminal.
    MeshNodeState {
        Pending = "PENDING",
        Registered = "REGISTERED",
        Online = "ONLINE",
        Offline = "OFFLINE",
        Revoked = "REVOKED",
    }
}
