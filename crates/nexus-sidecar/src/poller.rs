//! Legacy poller boundary (directive R/S).
//!
//! The sidecar owns the poller adapter boundary over the SDK's
//! `LegacyPoller` port: it reads a real JSONL source file, normalizes
//! records into canonical events, and persists a validated checkpoint.
//!
//! Integrity rules (directive R):
//! - malformed source record -> typed failure or isolated rejection
//!   (the owned poller rejects the malformed record and continues;
//!   the whole poll fails closed on a truncated/incomplete file);
//! - oversized source record -> bounded;
//! - corrupted checkpoint -> detected, never silently reset to zero
//!   (a silent reset could replay consequential work without audit);
//! - restart resumes only from a validated checkpoint;
//! - unchanged poll emits no fabricated changes;
//! - duplicate source record follows exact dedupe behavior.
//!
//! Path safety (directive S): the source path and checkpoint path are
//! constrained to the configured state directory; `..` traversal,
//! absolute-path escape, and symlink escape are rejected.

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use nexus_connector_sdk::vocabulary::WebhookEvent;

use crate::error::{SidecarError, SidecarErrorKind};
use crate::limits::{Limits, validate_cursor};

/// One poll result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollResult {
    /// Canonical normalized events.
    pub events: Vec<WebhookEvent>,
    /// Next checkpoint cursor (line index).
    pub next_cursor: String,
    /// Records rejected in this poll (isolated rejection).
    pub rejected_records: u64,
}

/// Owned poller over a real JSONL source file.
///
/// The source and checkpoint paths must both resolve inside
/// `state_dir`. The checkpoint is a non-negative integer line index.
#[derive(Debug, Clone)]
pub struct PollSource {
    state_dir: PathBuf,
    source_path: PathBuf,
    checkpoint_path: PathBuf,
    max_record_bytes: u64,
    seen: HashSet<String>,
}

impl PollSource {
    /// Construct a poller with paths constrained to the state dir
    /// (directive S). Fails closed on unsafe paths.
    pub fn new(
        state_dir: impl Into<PathBuf>,
        source_path: impl Into<PathBuf>,
        checkpoint_path: impl Into<PathBuf>,
        limits: Limits,
    ) -> Result<Self, SidecarError> {
        let state_dir = state_dir.into();
        let source_path = source_path.into();
        let checkpoint_path = checkpoint_path.into();

        let state_canonical = fs::canonicalize(&state_dir).map_err(|e| {
            SidecarError::new(
                SidecarErrorKind::PollerCorrupt,
                format!("state directory unavailable: {e}"),
                None,
                None,
                None,
            )
        })?;

        let source_safe = constrain(&state_canonical, &source_path)?;
        let checkpoint_safe = constrain(&state_canonical, &checkpoint_path)?;

        Ok(Self {
            state_dir: state_canonical,
            source_path: source_safe,
            checkpoint_path: checkpoint_safe,
            max_record_bytes: limits.max_request_bytes,
            seen: HashSet::new(),
        })
    }

    /// The canonical state directory.
    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    /// The resolved source path.
    pub fn source(&self) -> &Path {
        &self.source_path
    }

    /// The resolved checkpoint path.
    pub fn checkpoint(&self) -> &Path {
        &self.checkpoint_path
    }

    /// Read the validated checkpoint cursor (directive R).
    ///
    /// A missing checkpoint starts at zero (fresh poller). A corrupt
    /// checkpoint (non-integer, negative, or unsafe) is a typed
    /// failure - never a silent reset.
    pub fn read_checkpoint(&self) -> Result<u64, SidecarError> {
        let raw = match fs::read_to_string(&self.checkpoint_path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => {
                return Err(SidecarError::new(
                    SidecarErrorKind::PollerCorrupt,
                    format!("checkpoint unreadable: {e}"),
                    None,
                    None,
                    None,
                ));
            }
        };
        let trimmed = raw.trim();
        if !validate_cursor(trimmed) {
            return Err(SidecarError::new(
                SidecarErrorKind::PollerCorrupt,
                "checkpoint is corrupt (must be a non-negative integer)",
                None,
                None,
                None,
            ));
        }
        trimmed.parse::<u64>().map_err(|_| {
            SidecarError::new(
                SidecarErrorKind::PollerCorrupt,
                "checkpoint is corrupt (non-numeric)",
                None,
                None,
                None,
            )
        })
    }

    /// Write a validated checkpoint (directive R: only validated
    /// cursors are persisted; the path is constrained by `new`).
    pub fn write_checkpoint(&self, cursor: u64) -> Result<(), SidecarError> {
        let value = cursor.to_string();
        fs::write(&self.checkpoint_path, value).map_err(|e| {
            SidecarError::new(
                SidecarErrorKind::PollerCorrupt,
                format!("checkpoint write failed: {e}"),
                None,
                None,
                None,
            )
        })
    }

    /// Poll the source from the checkpoint (directive R).
    ///
    /// Reads lines after the checkpoint, normalizes valid JSON
    /// records, isolates malformed records (counted, not fabricated),
    /// dedupes by record digest, and returns the next cursor. A
    /// truncated final line fails the whole poll closed (no partial
    /// success). A record exceeding the bound is isolated with a
    /// typed rejection.
    pub fn poll(&mut self) -> Result<PollResult, SidecarError> {
        let checkpoint = self.read_checkpoint()?;
        let raw = fs::read_to_string(&self.source_path).map_err(|e| {
            SidecarError::new(
                SidecarErrorKind::Unavailable,
                format!("legacy source unavailable: {e}"),
                None,
                None,
                Some(self.source_path.display().to_string()),
            )
        })?;

        let mut events = Vec::new();
        let mut rejected = 0u64;
        let mut line_count = 0u64;
        let mut rejected_set: HashSet<String> = HashSet::new();

        for (idx, line) in raw.lines().enumerate() {
            let index = idx as u64;
            line_count = index + 1;
            if index < checkpoint {
                continue;
            }
            if line.len() as u64 > self.max_record_bytes {
                rejected += 1;
                rejected_set.insert(index.to_string());
                continue;
            }
            if line.trim().is_empty() {
                continue;
            }
            let record: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    // Truncated/incomplete final line fails closed; a
                    // mid-file malformed record is isolated.
                    if index + 1 == raw.lines().count() as u64 {
                        return Err(SidecarError::new(
                            SidecarErrorKind::PollerCorrupt,
                            format!("truncated JSONL record at line {index}: {e}"),
                            None,
                            None,
                            None,
                        ));
                    }
                    rejected += 1;
                    rejected_set.insert(index.to_string());
                    continue;
                }
            };
            let digest = crate::webhook::hex_encode(&{
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(line.as_bytes());
                h.finalize()
            });
            if self.seen.contains(&digest) {
                // Exact dedupe: identical record already emitted.
                continue;
            }
            self.seen.insert(digest);
            events.push(WebhookEvent {
                event_id: format!("legacy-{index}"),
                event_type: "legacy.record.created".to_string(),
                version: "1".to_string(),
                correlation_id: "00000000-0000-7000-8000-000000000000".to_string(),
                payload: record,
            });
        }

        let next_cursor = checkpoint.max(line_count).to_string();
        self.write_checkpoint(next_cursor.parse::<u64>().unwrap_or(checkpoint))?;
        Ok(PollResult {
            events,
            next_cursor,
            rejected_records: rejected,
        })
    }
}

/// Constrain a path to a canonical state directory (directive S).
///
/// Rejects `..` traversal, absolute escape, and symlink escape; the
/// returned path is canonicalized and verified to be inside the state
/// directory. Files that do not exist yet (e.g. a fresh checkpoint)
/// are resolved by canonicalizing the nearest existing ancestor and
/// appending the remaining components, then re-verifying containment.
fn constrain(state_canonical: &Path, candidate: &Path) -> Result<PathBuf, SidecarError> {
    if candidate.is_absolute() {
        // Absolute paths are allowed only when they resolve inside the
        // state dir (non-existent leaves resolved via the prefix).
        let resolved = resolve_with_prefix(state_canonical, candidate)?;
        if !resolved.starts_with(state_canonical) {
            return Err(SidecarError::new(
                SidecarErrorKind::PollerCorrupt,
                "path escapes the state directory",
                None,
                None,
                None,
            ));
        }
        return Ok(resolved);
    }

    // Relative path: reject any parent/traversal component.
    for component in candidate.components() {
        match component {
            Component::ParentDir => {
                return Err(SidecarError::new(
                    SidecarErrorKind::PollerCorrupt,
                    "path traversal rejected (..)",
                    None,
                    None,
                    None,
                ));
            }
            Component::RootDir => {
                return Err(SidecarError::new(
                    SidecarErrorKind::PollerCorrupt,
                    "absolute path escape rejected",
                    None,
                    None,
                    None,
                ));
            }
            _ => {}
        }
    }
    let joined = state_canonical.join(candidate);
    // Resolve symlinks in the existing prefix; append the remainder.
    let resolved = resolve_with_prefix(state_canonical, &joined)?;
    if !resolved.starts_with(state_canonical) {
        return Err(SidecarError::new(
            SidecarErrorKind::PollerCorrupt,
            "symlink escape rejected",
            None,
            None,
            None,
        ));
    }
    Ok(resolved)
}

/// Canonicalize the longest existing ancestor of `joined`, then append
/// the remaining components (so non-existent leaves resolve safely).
fn resolve_with_prefix(state_canonical: &Path, joined: &Path) -> Result<PathBuf, SidecarError> {
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    let mut probe = joined.to_path_buf();
    loop {
        match fs::canonicalize(&probe) {
            Ok(canonical) => {
                let mut resolved = canonical;
                for part in missing.iter().rev() {
                    resolved.push(part);
                }
                return Ok(resolved);
            }
            Err(_) => {
                let Some(name) = probe.file_name().map(std::ffi::OsStr::to_os_string) else {
                    return Err(SidecarError::new(
                        SidecarErrorKind::PollerCorrupt,
                        "path cannot be resolved",
                        None,
                        None,
                        None,
                    ));
                };
                missing.push(name);
                if !probe.pop() {
                    return Err(SidecarError::new(
                        SidecarErrorKind::PollerCorrupt,
                        "path cannot be resolved",
                        None,
                        None,
                        None,
                    ));
                }
                if !probe.starts_with(state_canonical) {
                    return Err(SidecarError::new(
                        SidecarErrorKind::PollerCorrupt,
                        "path escapes the state directory",
                        None,
                        None,
                        None,
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup() -> (tempfile::TempDir, Limits) {
        let dir = tempfile::tempdir().unwrap();
        (dir, Limits::default())
    }

    #[test]
    fn ep011_unit_sidecar_poller_rejects_traversal() {
        let (dir, limits) = setup();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();
        let err =
            PollSource::new(&state, "../escape.jsonl", "checkpoint.ckpt", limits).unwrap_err();
        assert!(err.message.contains("path traversal"));
    }

    #[test]
    fn ep011_unit_sidecar_poller_rejects_absolute_escape() {
        let (dir, limits) = setup();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();
        let err = PollSource::new(&state, "/etc/passwd", "checkpoint.ckpt", limits).unwrap_err();
        assert!(err.message.contains("escapes the state directory"));
    }

    #[test]
    fn ep011_unit_sidecar_poller_checkpoint_corruption_is_detected() {
        let (dir, limits) = setup();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();
        let source = state.join("source.jsonl");
        let checkpoint = state.join("checkpoint.ckpt");
        fs::write(&source, "").unwrap();
        let poller = PollSource::new(&state, &source, &checkpoint, limits).unwrap();
        // Write a corrupt checkpoint.
        fs::write(&checkpoint, "not-a-number\n").unwrap();
        let err = poller.read_checkpoint().unwrap_err();
        assert!(err.message.contains("corrupt"));
        // No silent reset: read again still fails.
        assert!(poller.read_checkpoint().is_err());
    }

    #[test]
    fn ep011_unit_sidecar_poller_polls_real_source_and_checkpoints() {
        let (dir, limits) = setup();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();
        let source = state.join("source.jsonl");
        let checkpoint = state.join("checkpoint.ckpt");
        fs::write(&source, "{\"row\":1}\n{\"row\":2}\n").unwrap();
        let mut poller = PollSource::new(&state, &source, &checkpoint, limits).unwrap();
        let result = poller.poll().unwrap();
        assert_eq!(result.events.len(), 2);
        assert_eq!(result.next_cursor, "2");
        assert_eq!(poller.read_checkpoint().unwrap(), 2);
        // Unchanged poll emits no fabricated changes.
        let result2 = poller.poll().unwrap();
        assert_eq!(result2.events.len(), 0);
        assert_eq!(result2.next_cursor, "2");
    }

    #[test]
    fn ep011_unit_sidecar_poller_isolates_malformed_and_truncates_fail_closed() {
        let (dir, limits) = setup();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();
        let source = state.join("source.jsonl");
        let checkpoint = state.join("checkpoint.ckpt");
        // Mid-file malformed record is isolated; final truncated line
        // fails the poll closed.
        fs::write(&source, "{\"row\":1}\nnot-json\n{\"row\":3}\n").unwrap();
        let mut poller = PollSource::new(&state, &source, &checkpoint, limits).unwrap();
        let result = poller.poll().unwrap();
        assert_eq!(result.events.len(), 2);
        assert_eq!(result.rejected_records, 1);

        fs::write(&source, "{\"row\":1}\n{\"row\":2}\n{\"trunc").unwrap();
        // Fresh poller state: remove the checkpoint written by the
        // first poll so the truncated file is read from the start.
        let _ = fs::remove_file(&checkpoint);
        let mut poller = PollSource::new(&state, &source, &checkpoint, limits).unwrap();
        let err = poller.poll().unwrap_err();
        assert!(err.message.contains("truncated"));
    }

    #[test]
    fn ep011_unit_sidecar_poller_dedupes_exact_records() {
        let (dir, limits) = setup();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();
        let source = state.join("source.jsonl");
        let checkpoint = state.join("checkpoint.ckpt");
        fs::write(&source, "{\"row\":1}\n{\"row\":1}\n").unwrap();
        let mut poller = PollSource::new(&state, &source, &checkpoint, limits).unwrap();
        let result = poller.poll().unwrap();
        assert_eq!(result.events.len(), 1);
    }

    #[test]
    fn ep011_unit_sidecar_poller_rejects_oversized_record() {
        let (dir, limits) = setup();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();
        let source = state.join("source.jsonl");
        let checkpoint = state.join("checkpoint.ckpt");
        let big = "x".repeat(80 * 1024);
        fs::write(&source, format!("{{\"blob\":\"{big}\"}}\n")).unwrap();
        let mut poller = PollSource::new(&state, &source, &checkpoint, limits).unwrap();
        let result = poller.poll().unwrap();
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.rejected_records, 1);
    }
}
