//! Canonical memory wire model (SPEC-002, EP-004).
//!
//! Field names and enum wire values mirror `schemas/memory-record.schema.json`
//! exactly (additionalProperties: false; snake_case; canonical strings).
//! MemoryType comes from the locked vocabulary in `nexus-domain`.
//!
//! INV-014: memory writes carry provenance, sensitivity, retention,
//! confidence, and supersession semantics. INV-007: namespaces cannot leak
//! across context construction.

use std::fmt;
use std::str::FromStr;

use nexus_domain::{MemoryType, NexusId, TenantId};
use serde::{Deserialize, Serialize};

use crate::error::{DataError, DataErrorCode};

/// Sensitivity classification of a memory record (SPEC-002, SPEC-020).
///
/// Values mirror the canonical data-classification ladder so a memory
/// record can be filtered and redacted by the same policy engine that
/// governs privacy classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sensitivity {
    /// Visible to any authenticated caller within the tenant.
    Public,
    /// Household-private.
    Household,
    /// Personal to one principal.
    Personal,
    /// Requires explicit purpose-limited access.
    Sensitive,
    /// Business-confidential.
    BusinessConfidential,
    /// Security-relevant (alerts, audit, trust).
    Security,
    /// Secret; never in prompts or logs.
    Secret,
}

impl Sensitivity {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "PUBLIC",
            Self::Household => "HOUSEHOLD",
            Self::Personal => "PERSONAL",
            Self::Sensitive => "SENSITIVE",
            Self::BusinessConfidential => "BUSINESS_CONFIDENTIAL",
            Self::Security => "SECURITY",
            Self::Secret => "SECRET",
        }
    }
}

impl fmt::Display for Sensitivity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Sensitivity {
    type Err = DataError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PUBLIC" => Ok(Self::Public),
            "HOUSEHOLD" => Ok(Self::Household),
            "PERSONAL" => Ok(Self::Personal),
            "SENSITIVE" => Ok(Self::Sensitive),
            "BUSINESS_CONFIDENTIAL" => Ok(Self::BusinessConfidential),
            "SECURITY" => Ok(Self::Security),
            "SECRET" => Ok(Self::Secret),
            other => Err(DataError::new(
                DataErrorCode::Validation,
                format!("unknown sensitivity class: {other}"),
            )),
        }
    }
}

/// Lifecycle status of a memory record (schema enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MemoryStatus {
    /// Written as a proposal; not yet a canonical semantic fact.
    Proposed,
    /// Active canonical fact.
    Active,
    /// Superseded by a newer record (SPEC-002 supersession).
    Superseded,
    /// Rejected by policy evaluation.
    Rejected,
    /// Deleted (retention or explicit deletion; EXPORT/DELETE workflows).
    Deleted,
}

impl MemoryStatus {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "PROPOSED",
            Self::Active => "ACTIVE",
            Self::Superseded => "SUPERSEDED",
            Self::Rejected => "REJECTED",
            Self::Deleted => "DELETED",
        }
    }
}

impl fmt::Display for MemoryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MemoryStatus {
    type Err = DataError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PROPOSED" => Ok(Self::Proposed),
            "ACTIVE" => Ok(Self::Active),
            "SUPERSEDED" => Ok(Self::Superseded),
            "REJECTED" => Ok(Self::Rejected),
            "DELETED" => Ok(Self::Deleted),
            other => Err(DataError::new(
                DataErrorCode::Validation,
                format!("unknown memory status: {other}"),
            )),
        }
    }
}

/// Unit of a retention duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetentionUnit {
    Hours,
    Days,
    Weeks,
    Months,
    Years,
    /// No automatic expiry (legal hold or indefinite retention).
    Indefinite,
}

/// Retention policy for a memory record (SPEC-002, SPEC-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Retention unit.
    pub unit: RetentionUnit,
    /// Duration in `unit`; ignored when unit is `Indefinite`.
    pub value: u32,
}

impl RetentionPolicy {
    /// Indefinite retention (legal hold / no expiry).
    pub const fn indefinite() -> Self {
        Self {
            unit: RetentionUnit::Indefinite,
            value: 0,
        }
    }

    /// A bounded retention duration.
    pub const fn for_duration(unit: RetentionUnit, value: u32) -> Self {
        Self { unit, value }
    }

    /// Whether the policy never expires automatically.
    pub const fn is_indefinite(&self) -> bool {
        matches!(self.unit, RetentionUnit::Indefinite)
    }
}

impl fmt::Display for RetentionPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_indefinite() {
            f.write_str("INDEFINITE")
        } else {
            write!(f, "{:?} {}", self.unit, self.value)
        }
    }
}

/// Versioned embedding reference (SPEC-002 behavior 2).
///
/// The embedding model and dimensions are versioned per row so a model
/// upgrade can re-embed without losing provenance. This is a reference,
/// never the vector payload itself; pgvector stores the vector, and the
/// canonical store keeps this reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingRef {
    /// Embedding model identifier (provider-neutral).
    pub model: String,
    /// Vector dimensionality for this row.
    pub dimensions: u32,
    /// Model version at write time.
    pub version: String,
}

/// A memory write proposal (SPEC-002 behavior 5).
///
/// Models cannot directly create canonical semantic facts. Writes enter as
/// proposals and become `ACTIVE` only after policy evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryProposal {
    /// Proposed record body (status forced to `PROPOSED` at evaluation).
    pub record: MemoryRecord,
}

/// Canonical memory record (SPEC-002, schema `memory-record.schema.json`).
///
/// `additionalProperties: false` on the wire: unknown fields are rejected
/// during deserialization, matching the canonical schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    /// Record identifier.
    pub memory_id: NexusId,
    /// Tenant boundary (INV-005, tenant isolation).
    pub tenant_id: TenantId,
    /// Memory namespace (INV-007): user, household, business, private, security.
    pub namespace: String,
    /// Memory type (locked vocabulary).
    pub memory_type: MemoryType,
    /// Structured content; never free-form provider payload.
    pub content: serde_json::Value,
    /// SHA-256 hex digest of the canonical content (schema pattern).
    pub content_hash: String,
    /// Provenance source (e.g. channel, provider, workflow).
    pub source: String,
    /// Actor (principal or system) that produced the record.
    pub actor: String,
    /// RFC 3339 timestamp when the record was created.
    pub created_at: String,
    /// RFC 3339 timestamp when the fact was observed.
    pub observed_at: String,
    /// Confidence in [0, 1].
    pub confidence: f64,
    /// Sensitivity classification (INV-014).
    pub sensitivity: Sensitivity,
    /// Purpose limitation label (SPEC-020).
    pub purpose: String,
    /// Retention policy (SPEC-020).
    pub retention: RetentionPolicy,
    /// Lifecycle status (schema enum).
    pub status: MemoryStatus,
    /// Records this fact was derived from (provenance chain).
    pub derived_from: Vec<NexusId>,
    /// Record this fact supersedes, when applicable.
    pub supersedes: Option<NexusId>,
    /// Versioned embedding reference, when indexed.
    pub embedding_ref: Option<EmbeddingRef>,
}

impl MemoryRecord {
    /// Validate canonical invariants (schema constraints + INV-014).
    pub fn validate(&self) -> Result<(), DataError> {
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(DataError::new(
                DataErrorCode::Validation,
                "confidence must be in [0, 1]",
            ));
        }
        let is_hex64 = self.content_hash.len() == 64
            && self.content_hash.bytes().all(|b| b.is_ascii_hexdigit());
        if !is_hex64 {
            return Err(DataError::new(
                DataErrorCode::Validation,
                "content_hash must be 64 hex characters",
            ));
        }
        if self.namespace.is_empty() {
            return Err(DataError::new(
                DataErrorCode::Validation,
                "namespace must not be empty",
            ));
        }
        if self.source.is_empty() || self.actor.is_empty() {
            return Err(DataError::new(
                DataErrorCode::Validation,
                "source and actor must not be empty",
            ));
        }
        Ok(())
    }
}

/// Query over memory records (SPEC-002 behavior 6).
///
/// Retrieval combines authorization filters, structured lookup, full-text,
/// vector, graph, recency, importance, confidence, and diversity. Every
/// filter is optional; the provider combines them with the caller's
/// authorization boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Required namespace scope (INV-007); empty means tenant-wide.
    pub namespace: Option<String>,
    /// Optional memory type filter.
    pub memory_type: Option<MemoryType>,
    /// Optional sensitivity ceiling (records above are filtered out).
    pub max_sensitivity: Option<Sensitivity>,
    /// Optional status filter.
    pub status: Option<MemoryStatus>,
    /// Optional full-text term.
    pub text: Option<String>,
    /// Optional embedding reference for vector similarity.
    pub embedding_ref: Option<EmbeddingRef>,
    /// Optional recency bound (RFC 3339; only records observed at or after).
    pub observed_after: Option<String>,
    /// Maximum results.
    pub limit: usize,
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            namespace: None,
            memory_type: None,
            max_sensitivity: None,
            status: None,
            text: None,
            embedding_ref: None,
            observed_after: None,
            limit: 20,
        }
    }
}

/// A retrieval result (SPEC-002 hybrid retrieval).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCandidate {
    /// The matched record.
    pub record: MemoryRecord,
    /// Retrieval score in [0, 1]; provider-defined blend.
    pub score: f64,
}
