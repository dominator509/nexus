//! Deterministic helpers for the EP-016 memory workers.
//!
//! These utilities never read a clock, never touch I/O, and never use
//! randomness. All values are pure functions of their inputs so the
//! workers remain replayable and cache-friendly.

use nexus_data::memory::{RetentionPolicy, RetentionUnit, Sensitivity};
use nexus_domain::MemoryType;

/// Canonical sensitivity ladder rank (higher = more sensitive).
/// Matches the classification ladder used by EP-004/EP-008 policy.
pub fn sensitivity_rank(sensitivity: Sensitivity) -> u8 {
    match sensitivity {
        Sensitivity::Public => 0,
        Sensitivity::Household => 1,
        Sensitivity::Personal => 2,
        Sensitivity::Sensitive => 3,
        Sensitivity::BusinessConfidential => 4,
        Sensitivity::Security => 5,
        Sensitivity::Secret => 6,
    }
}

/// Approximate seconds in a retention unit. `Indefinite` yields `None`.
/// Months and years use fixed calendar approximations (30 days / 365
/// days) so retention checks are deterministic without a calendar.
pub fn retention_seconds(policy: &RetentionPolicy) -> Option<u64> {
    let base = match policy.unit {
        RetentionUnit::Hours => 3_600u64,
        RetentionUnit::Days => 86_400,
        RetentionUnit::Weeks => 604_800,
        RetentionUnit::Months => 2_592_000,
        RetentionUnit::Years => 31_536_000,
        RetentionUnit::Indefinite => return None,
    };
    Some(base.saturating_mul(u64::from(policy.value)))
}

/// Parse an RFC 3339 UTC timestamp (`2026-01-01T00:00:00Z`) into
/// milliseconds since the Unix epoch. Returns `None` for malformed or
/// non-UTC input; callers treat unparseable timestamps as unknown and
/// use a deterministic default rather than failing the whole request.
pub fn rfc3339_utc_millis(value: &str) -> Option<u64> {
    let value = value.trim();
    // Accept an optional trailing 'Z' (canonical wire form). Fractional
    // seconds and offsets are not part of the canonical memory record
    // timestamp format; reject them rather than guess.
    let body = value.strip_suffix('Z')?;
    if body.contains('+') || body[10..].contains('-') || body.contains('.') {
        return None;
    }
    let (date, time) = body.split_once('T')?;
    let (y, m, d) = parse_date(date)?;
    let (hh, mm, ss) = parse_time(time)?;
    if y < 1970 {
        return None;
    }
    let days = days_from_civil(y, m, d);
    let secs = days * 86_400 + i64::from(hh) * 3_600 + i64::from(mm) * 60 + i64::from(ss);
    if secs < 0 {
        return None;
    }
    Some(secs as u64 * 1_000)
}

fn parse_date(date: &str) -> Option<(i64, u32, u32)> {
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

fn parse_time(time: &str) -> Option<(u32, u32, u32)> {
    let mut parts = time.split(':');
    let hh: u32 = parts.next()?.parse().ok()?;
    let mm: u32 = parts.next()?.parse().ok()?;
    let ss: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    Some((hh, mm, ss))
}

/// Days from civil date (proleptic Gregorian) to Unix epoch, using the
/// classic `days_from_civil` algorithm (deterministic, no calendar
/// dependency).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as i64 + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Stable 64-bit FNV-1a hash of a string. Deterministic across runs and
/// platforms (unlike `DefaultHasher`), used for namespace fingerprints
/// and telemetry labels. Never a security primitive.
pub fn fnv1a64(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Short stable fingerprint (first 16 hex chars of FNV-1a) for a
/// namespace in redacted telemetry. Never reversible to the namespace.
pub fn namespace_fingerprint(namespace: &str) -> String {
    format!("{:016x}", fnv1a64(namespace))
}

/// Rank helper for memory types used by purpose policy (no Ord on
/// MemoryType in nexus-domain). Lower is more fundamental.
pub fn memory_type_rank(memory_type: MemoryType) -> u8 {
    match memory_type {
        MemoryType::System => 0,
        MemoryType::Working => 1,
        MemoryType::Procedural => 2,
        MemoryType::Semantic => 3,
        MemoryType::Entity => 4,
        MemoryType::Episodic => 5,
        MemoryType::Decision => 6,
        MemoryType::Skill => 7,
    }
}

/// Clamp a score into `[0.0, 1.0]`.
pub fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep016_unit_rfc3339_utc_millis_parses_canonical() {
        let ms = rfc3339_utc_millis("2026-01-01T00:00:00Z").unwrap();
        // 2026-01-01T00:00:00Z = 1767225600s.
        assert_eq!(ms, 1_767_225_600_000);
    }

    #[test]
    fn ep016_unit_rfc3339_utc_millis_rejects_offsets_and_fractions() {
        assert!(rfc3339_utc_millis("2026-01-01T00:00:00+02:00").is_none());
        assert!(rfc3339_utc_millis("2026-01-01T00:00:00.123Z").is_none());
        assert!(rfc3339_utc_millis("not-a-time").is_none());
        assert!(rfc3339_utc_millis("").is_none());
    }

    #[test]
    fn ep016_unit_sensitivity_ladder_is_stable() {
        assert!(sensitivity_rank(Sensitivity::Public) < sensitivity_rank(Sensitivity::Secret));
        assert_eq!(sensitivity_rank(Sensitivity::Personal), 2);
        assert_eq!(sensitivity_rank(Sensitivity::BusinessConfidential), 4);
    }

    #[test]
    fn ep016_unit_retention_seconds_is_deterministic() {
        let days = RetentionPolicy::for_duration(RetentionUnit::Days, 30);
        assert_eq!(retention_seconds(&days), Some(30 * 86_400));
        assert!(retention_seconds(&RetentionPolicy::indefinite()).is_none());
    }

    #[test]
    fn ep016_unit_fnv_fingerprint_is_stable_and_short() {
        let a = namespace_fingerprint("household");
        let b = namespace_fingerprint("household");
        let c = namespace_fingerprint("business");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 16);
        assert!(!a.contains("household"));
    }

    #[test]
    fn ep016_unit_clamp01_bounds() {
        assert_eq!(clamp01(-1.0), 0.0);
        assert_eq!(clamp01(0.5), 0.5);
        assert_eq!(clamp01(2.0), 1.0);
    }
}
