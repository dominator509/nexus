//! EP-031 advanced sentinel error surface (SPEC-006 codes; SPEC-013
//! error states).
//!
//! The canonical sentinel error type lives in nexus-sentinel
//! (SentinelError, SPEC-006 codes). This crate re-exports it so EP-031
//! provider ports and services return the SAME typed error surface as
//! the sentinel core, never a parallel vocabulary.

pub use nexus_sentinel::{SentinelError as AdvancedSentinelError, SentinelErrorCode};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep031_unit_advanced_error_reuses_spec006_codes() {
        // EP-031 failures use the canonical SPEC-006 codes; they are
        // never redefined or widened.
        let err = AdvancedSentinelError::unavailable("no advanced sensor bound");
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        let err = AdvancedSentinelError::validation("bad input");
        assert_eq!(err.code, SentinelErrorCode::Validation);
        let json = serde_json::to_string(&SentinelErrorCode::Verification).unwrap();
        assert_eq!(json, "\"VERIFICATION\"");
    }
}
