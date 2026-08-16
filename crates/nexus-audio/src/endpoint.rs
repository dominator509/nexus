//! EP-022 audio endpoint identity and state (provider-neutral).

use std::fmt;

use nexus_domain::PersonId;
use serde::{Deserialize, Serialize};

use crate::error::AudioError;
use crate::vocabulary::{EndpointRole, HardwareClass};

/// Typed audio endpoint id (bounded, non-empty).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AudioEndpointId(String);

impl AudioEndpointId {
    pub fn new(value: impl Into<String>) -> Result<Self, AudioError> {
        let value = value.into();
        if value.is_empty() || value.len() > 128 {
            return Err(AudioError::new(
                crate::error::AudioErrorCode::Validation,
                "audio endpoint id must be 1..=128 characters",
                None,
                None,
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AudioEndpointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Typed room id (bounded, non-empty). Rooms are owned by the audio
/// surface; nexus-domain does not define a room identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AudioRoomId(String);

impl AudioRoomId {
    pub fn new(value: impl Into<String>) -> Result<Self, AudioError> {
        let value = value.into();
        if value.is_empty() || value.len() > 64 {
            return Err(AudioError::new(
                crate::error::AudioErrorCode::Validation,
                "audio room id must be 1..=64 characters",
                None,
                None,
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AudioRoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Canonical endpoint availability (SPEC-012 behavior 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EndpointAvailability {
    Online,
    Offline,
    Connecting,
    Reconnecting,
}

impl EndpointAvailability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Online => "ONLINE",
            Self::Offline => "OFFLINE",
            Self::Connecting => "CONNECTING",
            Self::Reconnecting => "RECONNECTING",
        }
    }

    pub const fn is_available(self) -> bool {
        matches!(self, Self::Online)
    }
}

/// Serialization helper for nexus-domain PersonId (which does not
/// derive serde). Canonical wire form is the UUIDv7 string.
mod person_serde {
    use nexus_domain::PersonId;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(person: &Option<PersonId>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match person {
            Some(id) => serializer.serialize_some(id.as_str()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<PersonId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Option<String> = Option::<String>::deserialize(deserializer)?;
        match value {
            None => Ok(None),
            Some(raw) => PersonId::new(raw)
                .map(Some)
                .map_err(serde::de::Error::custom),
        }
    }
}

/// Provider-neutral audio endpoint (SPEC-012 behavior 6 matrix).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioEndpoint {
    pub endpoint_id: AudioEndpointId,
    pub hardware_class: HardwareClass,
    pub role: EndpointRole,
    pub name: String,
    pub room: Option<AudioRoomId>,
    #[serde(with = "person_serde")]
    pub person: Option<PersonId>,
    pub availability: EndpointAvailability,
}

impl AudioEndpoint {
    pub const fn schema() -> &'static str {
        "nexus.audio.endpoint.v1"
    }

    pub fn new(
        endpoint_id: AudioEndpointId,
        hardware_class: HardwareClass,
        role: EndpointRole,
        name: impl Into<String>,
    ) -> Self {
        Self {
            endpoint_id,
            hardware_class,
            role,
            name: name.into(),
            room: None,
            person: None,
            availability: EndpointAvailability::Online,
        }
    }

    pub fn with_room(mut self, room: AudioRoomId) -> Self {
        self.room = Some(room);
        self
    }

    pub fn with_person(mut self, person: PersonId) -> Self {
        self.person = Some(person);
        self
    }

    pub fn with_availability(mut self, availability: EndpointAvailability) -> Self {
        self.availability = availability;
        self
    }

    /// Versioned wire payload; never includes raw audio.
    pub fn to_wire(&self) -> serde_json::Value {
        serde_json::json!({
            "schema": Self::schema(),
            "endpoint_id": self.endpoint_id.as_str(),
            "hardware_class": self.hardware_class.as_str(),
            "role": self.role.as_str(),
            "name": self.name,
            "room": self.room.as_ref().map(|r| r.as_str()),
            "person": self.person.as_ref().map(|p| p.as_str()),
            "availability": self.availability.as_str(),
        })
    }
}
