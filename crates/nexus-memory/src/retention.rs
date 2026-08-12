//! Retention enforcement and legal hold (SPEC-002 behavior 4, SPEC-020).
//!
//! A record's retention policy determines when it may be deleted. Legal
//! hold overrides automatic expiry. The engine is deterministic: given a
//! record and a reference time, it reports whether the record is eligible
//! for deletion or must be retained.

use nexus_data::{DataError, DataErrorCode, MemoryRecord, MemoryStatus, RetentionUnit};

/// Retention evaluation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetentionError {
    /// The record is under legal hold and must not be deleted.
    LegalHold,
    /// The retention unit is unsupported.
    UnsupportedUnit,
}

impl std::fmt::Display for RetentionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LegalHold => f.write_str("record is under legal hold"),
            Self::UnsupportedUnit => f.write_str("unsupported retention unit"),
        }
    }
}

impl std::error::Error for RetentionError {}

/// Retention enforcement engine (SPEC-002 behavior 4, SPEC-020).
#[derive(Debug, Clone, Copy, Default)]
pub struct RetentionEngine;

/// Whether a record is eligible for deletion at a reference time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionDecision {
    /// Eligible for deletion (retention period elapsed, no hold).
    Eligible,
    /// Must be retained (within retention period or under legal hold).
    Retain,
}

impl RetentionEngine {
    /// Evaluate whether `record` is eligible for deletion at `now_unix_s`.
    ///
    /// `on_legal_hold(record)` is the caller-provided legal hold check; the
    /// engine is deterministic and holds override expiry.
    pub fn evaluate<F>(
        &self,
        record: &MemoryRecord,
        now_unix_s: i64,
        on_legal_hold: F,
    ) -> Result<RetentionDecision, DataError>
    where
        F: FnOnce(&MemoryRecord) -> bool,
    {
        if on_legal_hold(record) {
            return Err(DataError::new(
                DataErrorCode::Policy,
                "record is under legal hold",
            ));
        }
        if record.retention.is_indefinite() {
            return Ok(RetentionDecision::Retain);
        }
        let created = parse_rfc3339_unix(&record.created_at)?;
        let retention_seconds = duration_seconds(record)?;
        let expires = created
            .checked_add(retention_seconds)
            .ok_or_else(|| DataError::new(DataErrorCode::Invariant, "retention overflow"))?;
        if now_unix_s < expires {
            return Ok(RetentionDecision::Retain);
        }
        Ok(RetentionDecision::Eligible)
    }

    /// Compute the RFC 3339 deletion time for a record, if it is ever
    /// eligible. Returns `None` for indefinite retention.
    pub fn deletion_time(&self, record: &MemoryRecord) -> Result<Option<String>, DataError> {
        if record.retention.is_indefinite() {
            return Ok(None);
        }
        let created = parse_rfc3339_unix(&record.created_at)?;
        let retention_seconds = duration_seconds(record)?;
        let expires = created
            .checked_add(retention_seconds)
            .ok_or_else(|| DataError::new(DataErrorCode::Invariant, "retention overflow"))?;
        // Render as RFC 3339 UTC.
        Ok(Some(format_unix_rfc3339(expires)))
    }
}

fn parse_rfc3339_unix(s: &str) -> Result<i64, DataError> {
    // Accept the canonical format "YYYY-MM-DDTHH:MM:SSZ" used by the tests
    // and the schema's date-time format. We deliberately avoid chrono in
    // this crate (dependency-direction); parse the fixed-width prefix.
    let bytes = s.as_bytes();
    if s.len() < 20 || bytes[10] != b'T' || bytes[19] != b'Z' {
        return Err(DataError::new(
            DataErrorCode::Validation,
            "created_at must be RFC 3339 UTC",
        ));
    }
    let year: i64 = s[0..4]
        .parse()
        .map_err(|_| DataError::new(DataErrorCode::Validation, "bad year"))?;
    let month: i64 = s[5..7]
        .parse()
        .map_err(|_| DataError::new(DataErrorCode::Validation, "bad month"))?;
    let day: i64 = s[8..10]
        .parse()
        .map_err(|_| DataError::new(DataErrorCode::Validation, "bad day"))?;
    let hour: i64 = s[11..13]
        .parse()
        .map_err(|_| DataError::new(DataErrorCode::Validation, "bad hour"))?;
    let minute: i64 = s[14..16]
        .parse()
        .map_err(|_| DataError::new(DataErrorCode::Validation, "bad minute"))?;
    let second: i64 = s[17..19]
        .parse()
        .map_err(|_| DataError::new(DataErrorCode::Validation, "bad second"))?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return Err(DataError::new(
            DataErrorCode::Validation,
            "created_at out of range",
        ));
    }
    // Days since epoch via civil-from-days (Howard Hinnant algorithm).
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Ok(days * 86400 + hour * 3600 + minute * 60 + second)
}

fn duration_seconds(record: &MemoryRecord) -> Result<i64, DataError> {
    let unit = record.retention.unit;
    let value = i64::from(record.retention.value);
    let seconds = match unit {
        RetentionUnit::Hours => value.checked_mul(3600),
        RetentionUnit::Days => value.checked_mul(86400),
        RetentionUnit::Weeks => value.checked_mul(7 * 86400),
        RetentionUnit::Months => value.checked_mul(30 * 86400),
        RetentionUnit::Years => value.checked_mul(365 * 86400),
        RetentionUnit::Indefinite => return Ok(i64::MAX),
    }
    .ok_or_else(|| DataError::new(DataErrorCode::Invariant, "retention overflow"))?;
    Ok(seconds)
}

fn format_unix_rfc3339(unix: i64) -> String {
    // Reverse of civil-from-days (Hinnant). Only used for tests and audit.
    let days = unix.div_euclid(86400);
    let rem = unix.rem_euclid(86400);
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hour, minute, second
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_data::{RetentionPolicy, Sensitivity};
    use nexus_domain::MemoryType;
    use nexus_domain::{NexusId, TenantId};

    fn record(created_at: &str, retention: RetentionPolicy) -> MemoryRecord {
        MemoryRecord {
            memory_id: NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6c01").unwrap(),
            tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6c02").unwrap(),
            namespace: "household".to_string(),
            memory_type: MemoryType::Semantic,
            content: serde_json::json!({ "fact": true }),
            content_hash: "c".repeat(64),
            source: "test".to_string(),
            actor: "principal".to_string(),
            created_at: created_at.to_string(),
            observed_at: created_at.to_string(),
            confidence: 0.9,
            sensitivity: Sensitivity::Household,
            purpose: "remember".to_string(),
            retention,
            status: MemoryStatus::Active,
            derived_from: vec![],
            supersedes: None,
            embedding_ref: None,
        }
    }

    fn time(year: i64, month: i64, day: i64) -> i64 {
        // Reuse the parser: it is deterministic and tested.
        let s = format!("{year:04}-{month:02}-{day:02}T00:00:00Z");
        parse_rfc3339_unix(&s).unwrap()
    }

    #[test]
    fn ep004_unit_retention_keeps_within_period() {
        let engine = RetentionEngine;
        let r = record(
            "2026-08-01T00:00:00Z",
            RetentionPolicy::for_duration(RetentionUnit::Days, 30),
        );
        let decision = engine.evaluate(&r, time(2026, 8, 15), |_| false).unwrap();
        assert_eq!(decision, RetentionDecision::Retain);
    }

    #[test]
    fn ep004_unit_retention_eligible_after_period() {
        let engine = RetentionEngine;
        let r = record(
            "2026-08-01T00:00:00Z",
            RetentionPolicy::for_duration(RetentionUnit::Days, 30),
        );
        let decision = engine.evaluate(&r, time(2026, 9, 1), |_| false).unwrap();
        assert_eq!(decision, RetentionDecision::Eligible);
    }

    #[test]
    fn ep004_unit_retention_indefinite_never_eligible() {
        let engine = RetentionEngine;
        let r = record("2026-08-01T00:00:00Z", RetentionPolicy::indefinite());
        let decision = engine.evaluate(&r, time(2030, 1, 1), |_| false).unwrap();
        assert_eq!(decision, RetentionDecision::Retain);
    }

    #[test]
    fn ep004_unit_retention_legal_hold_overrides_expiry() {
        let engine = RetentionEngine;
        let r = record(
            "2026-08-01T00:00:00Z",
            RetentionPolicy::for_duration(RetentionUnit::Days, 30),
        );
        let err = engine.evaluate(&r, time(2026, 9, 1), |_| true).unwrap_err();
        assert_eq!(err.code(), DataErrorCode::Policy);
    }

    #[test]
    fn ep004_unit_retention_deletion_time_matches_duration() {
        let engine = RetentionEngine;
        let r = record(
            "2026-08-01T00:00:00Z",
            RetentionPolicy::for_duration(RetentionUnit::Days, 30),
        );
        assert_eq!(
            engine.deletion_time(&r).unwrap().as_deref(),
            Some("2026-08-31T00:00:00Z")
        );
        let indefinite = record("2026-08-01T00:00:00Z", RetentionPolicy::indefinite());
        assert_eq!(engine.deletion_time(&indefinite).unwrap(), None);
    }

    #[test]
    fn ep004_unit_rfc3339_parser_round_trips() {
        let unix = parse_rfc3339_unix("2026-08-12T10:30:45Z").unwrap();
        assert_eq!(format_unix_rfc3339(unix), "2026-08-12T10:30:45Z");
    }
}
