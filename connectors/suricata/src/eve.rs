//! Suricata EVE JSON surface (documented upstream Suricata eve.json
//! output format).
//!
//! Only the documented event vocabulary and bounded fields are
//! modeled. Provider payloads are normalized at the infrastructure
//! boundary and never become domain contracts; unknown event types
//! fail closed rather than being guessed.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Error returned when an EVE event type string is not documented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EveVocabularyError(pub String);

impl fmt::Display for EveVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown suricata eve event type: {}", self.0)
    }
}

impl std::error::Error for EveVocabularyError {}

/// Documented Suricata eve.json event types (upstream eve.json
/// format). The connector normalizes only these; unknown types are
/// rejected at the boundary (fail closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EveEventType {
    #[serde(rename = "alert")]
    Alert,
    #[serde(rename = "dns")]
    Dns,
    #[serde(rename = "flow")]
    Flow,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "tls")]
    Tls,
    #[serde(rename = "smtp")]
    Smtp,
    #[serde(rename = "ssh")]
    Ssh,
    #[serde(rename = "fileinfo")]
    Fileinfo,
    #[serde(rename = "netflow")]
    Netflow,
}

impl EveEventType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Alert => "alert",
            Self::Dns => "dns",
            Self::Flow => "flow",
            Self::Http => "http",
            Self::Tls => "tls",
            Self::Smtp => "smtp",
            Self::Ssh => "ssh",
            Self::Fileinfo => "fileinfo",
            Self::Netflow => "netflow",
        }
    }
}

impl fmt::Display for EveEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EveEventType {
    type Err = EveVocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "alert" => Ok(Self::Alert),
            "dns" => Ok(Self::Dns),
            "flow" => Ok(Self::Flow),
            "http" => Ok(Self::Http),
            "tls" => Ok(Self::Tls),
            "smtp" => Ok(Self::Smtp),
            "ssh" => Ok(Self::Ssh),
            "fileinfo" => Ok(Self::Fileinfo),
            "netflow" => Ok(Self::Netflow),
            other => Err(EveVocabularyError(other.to_string())),
        }
    }
}

/// Suricata alert severity (documented bound 1..=4, 1 highest).
/// Constructed values outside the documented bound are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SuricataAlertSeverity(u8);

impl SuricataAlertSeverity {
    pub fn new(value: u8) -> Result<Self, EveVocabularyError> {
        if (1..=4).contains(&value) {
            Ok(Self(value))
        } else {
            Err(EveVocabularyError(format!(
                "suricata alert severity must be 1..=4, got {value}"
            )))
        }
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl fmt::Display for SuricataAlertSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
