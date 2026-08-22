//! EP-037 canonical artifact storage vocabulary (SPEC-024).
//!
//! SPEC-024 canonical terms (ArtifactStore, ObjectRef, ArtifactManifest,
//! BackupSet, RecoveryKey, RestorePlan, RPO, RTO, StorageMigration) are
//! vocabulary locked; this crate uses them without redefining them.
//!
//! EP-037 owns the storage vocabulary: backend kinds (local, NAS,
//! SeaweedFS, MinIO compatibility, Cloudflare R2, Backblaze B2, Amazon S3
//! behind one contract), data classes, retention classes, backup states,
//! and restore verification states. Truthfulness is structural:
//! MINIO is a compatibility-only backend (community repository archived),
//! a backend declaration is not a benchmark, an artifact written is not an
//! artifact verified, and a backup created is not a restore proven.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ArtifactError;

macro_rules! enum_vocab {
    ($(#[$doc:meta])* $name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $wire),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = ArtifactError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($wire => Ok($name::$variant),)+
                    other => Err(ArtifactError::vocabulary(format!(
                        concat!(stringify!($name), " has unsupported value '{}'"),
                        other
                    ))),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ArtifactError;
            fn try_from(s: &str) -> Result<Self, ArtifactError> {
                s.parse()
            }
        }
    };
}

enum_vocab! {
    /// Storage backend kind. All backends satisfy ONE provider-neutral
    /// contract (SPEC-024 requirement 1). MinIO is compatibility-only
    /// because the community repository is archived; the UI warns and
    /// recommends a maintained alternative. Unknown backends are rejected,
    /// never dynamically invented.
    StorageBackend {
        Local => "LOCAL",
        Nas => "NAS",
        SeaweedFs => "SEAWEEDFS",
        MinIo => "MINIO",
        R2 => "R2",
        B2 => "B2",
        S3 => "S3",
    }
}

impl StorageBackend {
    /// True for backends that egress the node (every backend except the
    /// local filesystem). Encrypted-at-rest is mandatory before any
    /// sensitive artifact leaves the node (SPEC-024 requirement 4).
    pub fn leaves_node(self) -> bool {
        !matches!(self, Self::Local)
    }

    /// MinIO is compatibility-only (community repository archived); its
    /// UI must warn and recommend a maintained alternative.
    pub fn is_compatibility_only(self) -> bool {
        matches!(self, Self::MinIo)
    }
}

enum_vocab! {
    /// Artifact data class (SPEC-020 privacy and retention; SPEC-024
    /// sensitive artifact handling). PUBLIC != HOUSEHOLD != PERSONAL !=
    /// SENSITIVE; the class is never inferred from the artifact name.
    DataClass {
        Public => "PUBLIC",
        Household => "HOUSEHOLD",
        Personal => "PERSONAL",
        Sensitive => "SENSITIVE",
        BusinessConfidential => "BUSINESS_CONFIDENTIAL",
        Security => "SECURITY",
    }
}

impl DataClass {
    /// Sensitive classes must be encrypted before leaving the node
    /// (SPEC-024 requirement 4; SPEC-020 data governance).
    pub fn requires_encryption_before_egress(self) -> bool {
        matches!(
            self,
            Self::Sensitive | Self::BusinessConfidential | Self::Security
        )
    }
}

enum_vocab! {
    /// Retention class (SPEC-020 retention and deletion; SPEC-024 backup
    /// retention). Retention is declared policy, not a storage backend
    /// promise: EXPIRING does not mean DELETED, and DELETED does not mean
    /// RESOURCE_ABSENT_VERIFIED until exact-target verification.
    RetentionClass {
        Immediate => "IMMEDIATE",
        ShortTerm => "SHORT_TERM",
        LongTerm => "LONG_TERM",
        Permanent => "PERMANENT",
    }
}

enum_vocab! {
    /// Backup state ladder. CREATED != VERIFIED != RESTORED: a backup set
    /// with a signed manifest and hashes is created; verification requires
    /// a readback/hash check; restore proof requires a fresh-target
    /// restore (SPEC-024 non-goal: backup without restore proof).
    BackupState {
        Declared => "DECLARED",
        Created => "CREATED",
        Verified => "VERIFIED",
        Restored => "RESTORED",
    }
}

enum_vocab! {
    /// Restore verification state. A restore plan is declared, then
    /// executed on a fresh target, then validated component-by-component
    /// (SPEC-024 requirement 7). VALIDATED != REATTACHED: edge nodes
    /// reconnect through controlled re-enrollment or preserved trust only
    /// after validation.
    RestoreVerificationState {
        Declared => "DECLARED",
        Executed => "EXECUTED",
        Validated => "VALIDATED",
        Reattached => "REATTACHED",
    }
}

enum_vocab! {
    /// Storage migration state. Migration copies first, verifies hashes
    /// and metadata on the target, then changes the canonical location,
    /// observes, and only after approval deletes old objects (SPEC-024
    /// requirement 8; non-goal: deleting the old provider first).
    MigrationState {
        Requested => "REQUESTED",
        Copying => "COPYING",
        Verified => "VERIFIED",
        CanonicalLocationChanged => "CANONICAL_LOCATION_CHANGED",
        OldDeleted => "OLD_DELETED",
    }
}
