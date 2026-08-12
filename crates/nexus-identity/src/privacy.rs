//! Privacy context (SPEC-005, EP-003).
//!
//! Privacy classification and shared-room handling. SPEC-005 behavior 8:
//! shared-room responses route sensitive content to a private device when
//! other people may be present.

use nexus_domain::Privacy;
use serde::{Deserialize, Serialize};

/// Privacy classification for one interaction.
///
/// `privacy_class` uses the canonical `Privacy` vocabulary (PUBLIC,
/// HOUSEHOLD, PERSONAL, SENSITIVE, BUSINESS_CONFIDENTIAL, SECURITY, SECRET).
/// `shared_room` records whether other people may be present; when true and
/// the class is at least PERSONAL, sensitive responses must route to a
/// private device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyContext {
    /// Canonical privacy class.
    pub privacy_class: Privacy,
    /// Whether other people may be present in the interaction space.
    pub shared_room: bool,
}

impl PrivacyContext {
    /// Construct a privacy context.
    pub fn new(privacy_class: Privacy, shared_room: bool) -> Self {
        Self {
            privacy_class,
            shared_room,
        }
    }

    /// Whether sensitive content must route to a private device.
    ///
    /// True when the space is shared AND the class is at least PERSONAL
    /// (SPEC-005 behavior 8). PUBLIC and HOUSEHOLD content may be spoken in
    /// a shared room; PERSONAL and above must not.
    pub fn requires_private_routing(&self) -> bool {
        self.shared_room && privacy_rank(self.privacy_class) >= privacy_rank(Privacy::Personal)
    }
}

/// Deterministic rank of the canonical privacy classes.
///
/// `PUBLIC=0, HOUSEHOLD=1, PERSONAL=2, SENSITIVE=3, BUSINESS_CONFIDENTIAL=4,
/// SECURITY=5, SECRET=6`. The canonical enum does not derive `PartialOrd`,
/// so comparisons are explicit here (SPEC-001 ordering).
fn privacy_rank(class: Privacy) -> u8 {
    match class {
        Privacy::Public => 0,
        Privacy::Household => 1,
        Privacy::Personal => 2,
        Privacy::Sensitive => 3,
        Privacy::BusinessConfidential => 4,
        Privacy::Security => 5,
        Privacy::Secret => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep003_unit_privacy_private_routing_rules() {
        // Public content may be spoken in a shared room.
        assert!(!PrivacyContext::new(Privacy::Public, true).requires_private_routing());
        // Household content may be spoken in a shared room.
        assert!(!PrivacyContext::new(Privacy::Household, true).requires_private_routing());
        // Personal content in a shared room must route privately.
        assert!(PrivacyContext::new(Privacy::Personal, true).requires_private_routing());
        // Personal content in a private room is fine.
        assert!(!PrivacyContext::new(Privacy::Personal, false).requires_private_routing());
        // Secret content always routes privately when shared.
        assert!(PrivacyContext::new(Privacy::Secret, true).requires_private_routing());
    }

    #[test]
    fn ep003_unit_privacy_serde_roundtrip() {
        let p = PrivacyContext::new(Privacy::Sensitive, true);
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"privacy_class\":\"SENSITIVE\""));
        let back: PrivacyContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }
}
