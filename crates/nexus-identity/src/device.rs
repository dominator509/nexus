//! Device identity (SPEC-001, SPEC-005, EP-003).
//!
//! A device is independently scoped from people and sessions. Device
//! identity records trust posture; trust is evidence, never cryptographic
//! authentication (INV-003).

use std::fmt;

use nexus_domain::{DeviceId, PersonId, TenantId};
use serde::{Deserialize, Serialize};

/// Provider-neutral device class (SPEC-012/017; ADR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceKind {
    Phone,
    Tablet,
    Desktop,
    Laptop,
    Speaker,
    Camera,
    Display,
    Server,
    Appliance,
    Unknown,
}

impl DeviceKind {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Phone => "PHONE",
            Self::Tablet => "TABLET",
            Self::Desktop => "DESKTOP",
            Self::Laptop => "LAPTOP",
            Self::Speaker => "SPEAKER",
            Self::Camera => "CAMERA",
            Self::Display => "DISPLAY",
            Self::Server => "SERVER",
            Self::Appliance => "APPLIANCE",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Device trust ladder (SPEC-005; ADR-007). Trust is evidence; it never
/// substitutes for a cryptographic step-up on R3/R4 actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrustLevel {
    Unverified,
    Local,
    Verified,
}

impl TrustLevel {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "UNVERIFIED",
            Self::Local => "LOCAL",
            Self::Verified => "VERIFIED",
        }
    }
}

impl fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned by device identity construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceIdentityError {
    /// The device display name is empty or whitespace only.
    EmptyDisplayName,
}

impl fmt::Display for DeviceIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDisplayName => f.write_str("device display name must not be empty"),
        }
    }
}

impl std::error::Error for DeviceIdentityError {}

/// A device identity record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentity {
    /// Device identifier (SPEC-001).
    pub device_id: DeviceId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Display name.
    pub display_name: String,
    /// Provider-neutral device class.
    pub kind: DeviceKind,
    /// Current trust ladder position.
    pub trust_level: TrustLevel,
    /// Owning person reference, when the device is personal.
    pub owner_person_id: Option<PersonId>,
}

impl DeviceIdentity {
    /// Construct a validated device identity.
    pub fn new(
        device_id: DeviceId,
        tenant_id: TenantId,
        display_name: impl Into<String>,
        kind: DeviceKind,
        trust_level: TrustLevel,
        owner_person_id: Option<PersonId>,
    ) -> Result<Self, DeviceIdentityError> {
        let display_name = display_name.into();
        if display_name.trim().is_empty() {
            return Err(DeviceIdentityError::EmptyDisplayName);
        }
        Ok(Self {
            device_id,
            tenant_id,
            display_name,
            kind,
            trust_level,
            owner_person_id,
        })
    }

    /// Whether this device is verified enough for privileged operations.
    ///
    /// Verification is a trust signal only; it does not replace a
    /// cryptographic step-up for R3/R4 actions (SPEC-005 behavior 4).
    pub fn is_verified(&self) -> bool {
        self.trust_level >= TrustLevel::Verified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";

    #[test]
    fn ep003_unit_device_identity_constructs() {
        let d = DeviceIdentity::new(
            DeviceId::new(DID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "Kitchen Speaker",
            DeviceKind::Speaker,
            TrustLevel::Local,
            Some(PersonId::new(PID).unwrap()),
        )
        .unwrap();
        assert_eq!(d.kind.as_str(), "SPEAKER");
        assert!(!d.is_verified());
    }

    #[test]
    fn ep003_unit_device_identity_rejects_empty_display_name() {
        let res = DeviceIdentity::new(
            DeviceId::new(DID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "",
            DeviceKind::Phone,
            TrustLevel::Unverified,
            None,
        );
        assert_eq!(res, Err(DeviceIdentityError::EmptyDisplayName));
    }

    #[test]
    fn ep003_unit_device_identity_serde_roundtrip() {
        let d = DeviceIdentity::new(
            DeviceId::new(DID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            "Front Door Cam",
            DeviceKind::Camera,
            TrustLevel::Verified,
            None,
        )
        .unwrap();
        assert!(d.is_verified());
        let json = serde_json::to_string(&d).unwrap();
        let back: DeviceIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
        assert!(json.contains("\"trust_level\":\"VERIFIED\""));
    }
}
