//! Connector/capability dispatch table (directive G).
//!
//! The sidecar holds an explicit table of connector -> capabilities
//! with their canonical classes. Dispatch fails closed on:
//!
//! - connector A + a capability belonging to connector B (no fallback
//!   search across connectors);
//! - correct connector + wrong class (typed class mismatch);
//! - correct connector + unknown capability (typed NOT_FOUND).
//!
//! No fuzzy matching and no vendor alias resolution happen at
//! execution time.

use std::collections::BTreeMap;

use nexus_domain::vocabulary::CapabilityClass;

use crate::error::{SidecarError, SidecarErrorKind};

/// One capability entry in the dispatch table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityEntry {
    /// Canonical capability class (EP-010 vocabulary).
    pub class: CapabilityClass,
}

/// A connector's capability table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectorTable {
    connector_id: String,
    capabilities: BTreeMap<String, CapabilityEntry>,
}

impl ConnectorTable {
    /// Construct an empty table for a connector.
    pub fn new(connector_id: impl Into<String>) -> Self {
        Self {
            connector_id: connector_id.into(),
            capabilities: BTreeMap::new(),
        }
    }

    /// The connector id this table belongs to.
    pub fn connector(&self) -> &str {
        &self.connector_id
    }

    /// Register a capability with its canonical class.
    pub fn register(&mut self, capability_id: impl Into<String>, class: CapabilityClass) {
        self.capabilities
            .insert(capability_id.into(), CapabilityEntry { class });
    }

    /// Capability ids in canonical (sorted) order.
    pub fn capability_ids(&self) -> Vec<String> {
        self.capabilities.keys().cloned().collect()
    }

    /// The class of a capability known to this connector.
    pub fn class_of(&self, capability_id: &str) -> Option<CapabilityClass> {
        self.capabilities.get(capability_id).map(|e| e.class)
    }

    /// Dispatch check (directive G).
    ///
    /// Returns the canonical class when the capability belongs to
    /// this connector; otherwise a typed NOT_FOUND.
    pub fn resolve(&self, capability_id: &str) -> Result<CapabilityClass, SidecarError> {
        self.class_of(capability_id).ok_or_else(|| {
            SidecarError::new(
                SidecarErrorKind::Unavailable,
                "capability not found for connector",
                None,
                None,
                Some(capability_id.to_string()),
            )
        })
    }
}

/// Table of connector tables (directive G: no cross-connector search).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityClassTable {
    connectors: BTreeMap<String, ConnectorTable>,
}

impl CapabilityClassTable {
    /// Construct an empty table.
    pub fn new() -> Self {
        Self {
            connectors: BTreeMap::new(),
        }
    }

    /// Add/register a connector table.
    pub fn insert(&mut self, table: ConnectorTable) {
        self.connectors.insert(table.connector_id.clone(), table);
    }

    /// Connector ids in canonical order (the sidecar is
    /// single-connector; used for fingerprinting/telemetry).
    pub fn connectors(&self) -> Vec<String> {
        self.connectors.keys().cloned().collect()
    }

    /// Look up a connector's table.
    pub fn connector(&self, connector_id: &str) -> Option<&ConnectorTable> {
        self.connectors.get(connector_id)
    }

    /// Enforce connector/capability consistency (directive G).
    ///
    /// The connector must be known; the capability must belong to
    /// that connector (no fallback to other connectors); the declared
    /// class must match the table's canonical class.
    pub fn enforce(
        &self,
        connector_id: &str,
        capability_id: &str,
        declared_class: CapabilityClass,
        correlation_id: Option<&str>,
    ) -> Result<(), SidecarError> {
        let table = self.connector(connector_id).ok_or_else(|| {
            SidecarError::new(
                SidecarErrorKind::Unavailable,
                "unknown connector",
                correlation_id.map(str::to_string),
                None,
                Some(connector_id.to_string()),
            )
        })?;
        let canonical = table.resolve(capability_id)?;
        if canonical != declared_class {
            return Err(SidecarError::new(
                SidecarErrorKind::Validation,
                format!(
                    "capability class mismatch: declared {declared_class:?}, canonical {canonical:?}"
                ),
                correlation_id.map(str::to_string),
                None,
                Some(capability_id.to_string()),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> CapabilityClassTable {
        let mut t = CapabilityClassTable::new();
        let mut c = ConnectorTable::new("fixture-connector");
        c.register("fixture.contacts.query", CapabilityClass::Query);
        c.register("fixture.contacts.command", CapabilityClass::Command);
        t.insert(c);
        t
    }

    #[test]
    fn ep011_unit_sidecar_dispatch_accepts_correct_pair() {
        let t = table();
        assert!(
            t.enforce(
                "fixture-connector",
                "fixture.contacts.query",
                CapabilityClass::Query,
                None
            )
            .is_ok()
        );
    }

    #[test]
    fn ep011_unit_sidecar_dispatch_denies_wrong_class() {
        let t = table();
        let err = t
            .enforce(
                "fixture-connector",
                "fixture.contacts.query",
                CapabilityClass::Command,
                None,
            )
            .unwrap_err();
        assert_eq!(err.kind, SidecarErrorKind::Validation);
        assert!(err.message.contains("class mismatch"));
    }

    #[test]
    fn ep011_unit_sidecar_dispatch_denies_cross_connector_capability() {
        let t = table();
        // fixture-connector does not own other-connector's capability.
        let err = t
            .enforce(
                "fixture-connector",
                "other-connector.invoice.read",
                CapabilityClass::Query,
                None,
            )
            .unwrap_err();
        assert!(err.message.contains("capability not found"));
    }

    #[test]
    fn ep011_unit_sidecar_dispatch_denies_unknown_connector() {
        let t = table();
        let err = t
            .enforce(
                "other-connector",
                "fixture.contacts.query",
                CapabilityClass::Query,
                None,
            )
            .unwrap_err();
        assert!(err.message.contains("unknown connector"));
    }
}
