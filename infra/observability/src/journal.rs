//! Durable quarantine + audit journal (SPEC-007 behavior 8; AUD-057).
//!
//! The runtime's fallback recording sink and audit trail were
//! process-local: a provider outage followed by process loss dropped
//! the quarantined incident instead of preserving it for later
//! synchronization. This journal persists both surfaces as JSON-lines
//! files under a runtime-owned state directory:
//!
//! - `quarantine.jsonl` - one serialized `Incident` per line (the
//!   quarantined fallback records that have not reached the provider);
//! - `audit.jsonl` - one serialized `AuditRecord` per line.
//!
//! The journal is append-and-rewrite (bounded, low volume), opens with
//! mode 0600 files under a mode 0700 directory, and every write is
//! fsynced before returning so a process loss after a successful write
//! cannot lose the record. Records are `serde` types that were already
//! redacted at construction; nothing secret-shaped can enter.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use nexus_domain::IncidentId;
use nexus_observability::{Incident, ObservabilityError, ObservabilityResult};

use crate::audit::AuditRecord;

/// Durable JSON-lines journal for quarantined incidents and audit
/// records under one state directory.
#[derive(Debug, Clone)]
pub struct DurableJournal {
    quarantine_path: PathBuf,
    audit_path: PathBuf,
}

impl DurableJournal {
    /// Open (or create) a journal under `dir`. The directory is created
    /// with restrictive permissions when missing; journal files are
    /// created lazily on first write and opened with mode 0600.
    pub fn open(dir: impl AsRef<Path>) -> ObservabilityResult<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|e| {
            ObservabilityError::internal(format!("durable journal: create state dir failed: {e}"))
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        Ok(Self {
            quarantine_path: dir.join("quarantine.jsonl"),
            audit_path: dir.join("audit.jsonl"),
        })
    }

    /// Load every quarantined incident previously persisted. A corrupt
    /// line fails closed (never silently dropped), because a quarantine
    /// record that cannot be parsed must not disappear without a trace.
    pub fn load_incidents(&self) -> ObservabilityResult<Vec<Incident>> {
        self.load::<Incident>(&self.quarantine_path)
    }

    /// Load every audit record previously persisted. Corrupt lines fail
    /// closed for the same reason as incidents.
    pub fn load_audit(&self) -> ObservabilityResult<Vec<AuditRecord>> {
        self.load::<AuditRecord>(&self.audit_path)
    }

    /// Persist one incident, replacing any previous record with the same
    /// incident id (the canonical identity for one quarantined incident).
    pub fn store_incident(&self, incident: &Incident) -> ObservabilityResult<()> {
        self.replace_line(
            &self.quarantine_path,
            incident,
            incident.incident_id.as_str(),
        )
    }

    /// Remove one incident from the durable quarantine (called when the
    /// provider accepts it and no later synchronization is needed).
    pub fn remove_incident(&self, incident_id: &IncidentId) -> ObservabilityResult<()> {
        self.filter_incidents(&self.quarantine_path, |i| i.incident_id != *incident_id)
    }

    /// Append one audit record (audit is append-only).
    pub fn append_audit(&self, record: &AuditRecord) -> ObservabilityResult<()> {
        self.append_line(&self.audit_path, record)
    }

    /// True when a quarantine journal file currently holds zero
    /// incidents (used by tests and diagnostics; never a secret).
    pub fn quarantine_empty(&self) -> ObservabilityResult<bool> {
        Ok(self.load_incidents()?.is_empty())
    }

    // ----------------------------------------------------------- helpers

    fn load<T: serde::de::DeserializeOwned>(&self, path: &Path) -> ObservabilityResult<Vec<T>> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(ObservabilityError::internal(format!(
                    "durable journal: read {} failed: {e}",
                    path.display()
                )))
            }
        };
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for (idx, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| {
                ObservabilityError::internal(format!(
                    "durable journal: read {} line {} failed: {e}",
                    path.display(),
                    idx + 1
                ))
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let parsed = serde_json::from_str::<T>(&line).map_err(|e| {
                ObservabilityError::internal(format!(
                    "durable journal: corrupt record in {} line {}: {e}",
                    path.display(),
                    idx + 1
                ))
            })?;
            out.push(parsed);
        }
        Ok(out)
    }

    fn open_append(path: &Path) -> ObservabilityResult<File> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| {
                ObservabilityError::internal(format!(
                    "durable journal: open {} failed: {e}",
                    path.display()
                ))
            })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(file)
    }

    fn open_truncate(path: &Path) -> ObservabilityResult<File> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|e| {
                ObservabilityError::internal(format!(
                    "durable journal: open {} failed: {e}",
                    path.display()
                ))
            })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(file)
    }

    fn sync_rename(tmp: &Path, path: &Path) -> ObservabilityResult<()> {
        std::fs::rename(tmp, path).map_err(|e| {
            ObservabilityError::internal(format!(
                "durable journal: rename {} -> {} failed: {e}",
                tmp.display(),
                path.display()
            ))
        })
    }

    fn append_line<T: serde::Serialize>(&self, path: &Path, value: &T) -> ObservabilityResult<()> {
        let mut file = Self::open_append(path)?;
        let json = serde_json::to_string(value).map_err(|e| {
            ObservabilityError::internal(format!(
                "durable journal: serialize for {} failed: {e}",
                path.display()
            ))
        })?;
        writeln!(file, "{json}").map_err(|e| {
            ObservabilityError::internal(format!(
                "durable journal: write {} failed: {e}",
                path.display()
            ))
        })?;
        file.sync_all().map_err(|e| {
            ObservabilityError::internal(format!(
                "durable journal: fsync {} failed: {e}",
                path.display()
            ))
        })
    }

    /// Append-style replace for incidents: load existing, keep all but
    /// the one with the same canonical id, append the new record, and
    /// rewrite atomically (temp + rename) with fsync.
    fn replace_line(&self, path: &Path, value: &Incident, id: &str) -> ObservabilityResult<()> {
        let existing = self.load::<Incident>(path)?;
        let mut records: Vec<Incident> = existing
            .into_iter()
            .filter(|i| i.incident_id.as_str() != id)
            .collect();
        records.push(value.clone());

        let tmp = path.with_extension("tmp");
        let mut file = Self::open_truncate(&tmp)?;
        for rec in &records {
            let json = serde_json::to_string(rec).map_err(|e| {
                ObservabilityError::internal(format!(
                    "durable journal: serialize {} failed: {e}",
                    path.display()
                ))
            })?;
            writeln!(file, "{json}").map_err(|e| {
                ObservabilityError::internal(format!(
                    "durable journal: write {} failed: {e}",
                    path.display()
                ))
            })?;
        }
        file.sync_all().map_err(|e| {
            ObservabilityError::internal(format!(
                "durable journal: fsync {} failed: {e}",
                path.display()
            ))
        })?;
        drop(file);
        Self::sync_rename(&tmp, path)
    }

    /// Rewrite the incident file keeping only incidents satisfying
    /// `keep`. Used to remove one incident from the durable quarantine.
    fn filter_incidents(
        &self,
        path: &Path,
        keep: impl Fn(&Incident) -> bool,
    ) -> ObservabilityResult<()> {
        let existing = self.load::<Incident>(path)?;
        let tmp = path.with_extension("tmp");
        let mut file = Self::open_truncate(&tmp)?;
        for rec in existing.into_iter().filter(keep) {
            let json = serde_json::to_string(&rec).map_err(|e| {
                ObservabilityError::internal(format!(
                    "durable journal: serialize {} failed: {e}",
                    path.display()
                ))
            })?;
            writeln!(file, "{json}").map_err(|e| {
                ObservabilityError::internal(format!(
                    "durable journal: write {} failed: {e}",
                    path.display()
                ))
            })?;
        }
        file.sync_all().map_err(|e| {
            ObservabilityError::internal(format!(
                "durable journal: fsync {} failed: {e}",
                path.display()
            ))
        })?;
        drop(file);
        Self::sync_rename(&tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_observability::{
        IncidentState, RedactionPolicy, Severity, TelemetryContext, TelemetrySignal,
    };

    fn incident(id: u8) -> Incident {
        let incident_id =
            IncidentId::new(format!("018e5c5e-4d9b-7f0c-8a2b-{id:012x}")).expect("valid id");
        let observed: Vec<(String, String)> = vec![("message".to_string(), "boom".to_string())];
        let envelope = RedactionPolicy::default().apply(
            TelemetrySignal::Incident,
            TelemetryContext::new(
                "svc".to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
                "svc".to_string(),
                "incident.report".to_string(),
                Severity::Error,
                Some("test".to_string()),
                None,
            )
            .expect("valid context"),
            observed,
        );
        Incident {
            incident_id,
            dedupe_key: format!("storage:unavailable:{id}"),
            severity: Severity::Error,
            classification: "unavailable".to_string(),
            source: "storage".to_string(),
            correlation: None,
            state: IncidentState::Open,
            redacted_context: envelope,
            opened_at: 100,
            acknowledged_at: None,
            resolved_at: None,
            escalated: false,
        }
    }

    fn journal_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nexus-aud057-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn aud057_incident_survives_journal_rewrite() {
        let dir = journal_dir("incident");
        let j = DurableJournal::open(&dir).expect("open journal");
        let inc = incident(1);
        j.store_incident(&inc).expect("store");
        let loaded = j.load_incidents().expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].incident_id, inc.incident_id);
        assert_eq!(loaded[0].dedupe_key, inc.dedupe_key);
        // Replace same id -> still one line.
        j.store_incident(&inc).expect("store again");
        assert_eq!(j.load_incidents().expect("load").len(), 1);
        // Remove -> empty.
        j.remove_incident(&inc.incident_id).expect("remove");
        assert!(j.quarantine_empty().expect("empty check"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn aud057_multiple_incidents_and_selective_remove() {
        let dir = journal_dir("multi");
        let j = DurableJournal::open(&dir).expect("open journal");
        let a = incident(1);
        let b = incident(2);
        j.store_incident(&a).expect("store a");
        j.store_incident(&b).expect("store b");
        let loaded = j.load_incidents().expect("load");
        assert_eq!(loaded.len(), 2);
        j.remove_incident(&a.incident_id).expect("remove a");
        let loaded = j.load_incidents().expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].incident_id, b.incident_id);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn aud057_audit_survives_append_and_reload() {
        let dir = journal_dir("audit");
        let j = DurableJournal::open(&dir).expect("open journal");
        let rec = AuditRecord::new(
            1,
            crate::AuditSeverity::Error,
            "storage",
            "put",
            "n1",
            "unavailable",
            None,
            vec![("message".to_string(), "boom".to_string())],
        )
        .expect("audit record");
        j.append_audit(&rec).expect("append");
        let loaded = j.load_audit().expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].operation, "put");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn aud057_missing_journal_is_empty() {
        let dir = journal_dir("missing");
        let j = DurableJournal::open(&dir).expect("open journal");
        assert!(j.load_incidents().expect("load incidents").is_empty());
        assert!(j.load_audit().expect("load audit").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn aud057_corrupt_incident_line_fails_closed() {
        let dir = journal_dir("corrupt");
        let j = DurableJournal::open(&dir).expect("open journal");
        std::fs::write(dir.join("quarantine.jsonl"), "{\"not\":\"an incident\"}\n")
            .expect("write corrupt line");
        let result = j.load_incidents();
        assert!(
            result.is_err(),
            "corrupt journal line must fail closed, not be silently dropped"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
