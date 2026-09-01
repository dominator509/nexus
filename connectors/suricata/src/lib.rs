//! EP-031 Suricata connector (SPEC-013: Enhanced profile adds
//! Suricata; AUD-030).
//!
//! This crate is the Suricata network-detection adapter boundary: it
//! implements the provider-neutral `NetworkDetectionProvider` port
//! from nexus-sentinel-advanced over the DOCUMENTED Suricata EVE JSON
//! surface (eve.json). The real transport parses newline-delimited
//! EVE alert records from an arbitrary byte source (file, pipe,
//! socket); the adapter normalizes observed alerts into canonical
//! sentinel findings without inventing root causes.
//!
//! Dependency direction: this connector depends on nexus-domain,
//! nexus-sentinel, and nexus-sentinel-advanced (contract crates). It
//! never defines a parallel domain vocabulary.

#![forbid(unsafe_code)]

/// Suricata EVE JSON event surface (documented upstream: Suricata
/// eve.json output). Free-form provider payloads are normalized at
/// the infrastructure boundary and never become domain contracts.
pub mod eve;

/// Real production transport over the documented EVE JSON alert
/// surface (newline-delimited records; event_type discriminator).
pub mod transport;

/// Real production adapter behind the `NetworkDetectionProvider` port.
pub mod adapter;

pub use adapter::SuricataNetworkDetectionProvider;
pub use transport::{JsonLinesSuricataTransport, SuricataAlert, SuricataTransport};

#[cfg(test)]
mod tests {
    use super::eve::*;

    #[test]
    fn ep031_unit_suricata_eve_event_type_vocabulary_locked() {
        assert_eq!(EveEventType::Alert.as_str(), "alert");
        assert_eq!(EveEventType::Dns.as_str(), "dns");
        assert_eq!(EveEventType::Flow.as_str(), "flow");
        let json = serde_json::to_string(&EveEventType::Alert).unwrap();
        assert_eq!(json, "\"alert\"");
        let back: EveEventType = serde_json::from_str("\"http\"").unwrap();
        assert_eq!(back, EveEventType::Http);
    }

    #[test]
    fn ep031_unit_suricata_alert_severity_is_bounded() {
        // Suricata alert severity is 1..=4 (1 highest). The connector
        // never fabricates a severity outside the documented bound.
        assert!(SuricataAlertSeverity::new(1).is_ok());
        assert!(SuricataAlertSeverity::new(4).is_ok());
        assert!(SuricataAlertSeverity::new(0).is_err());
        assert!(SuricataAlertSeverity::new(5).is_err());
        assert_eq!(SuricataAlertSeverity::new(3).unwrap().as_u8(), 3);
    }
}
