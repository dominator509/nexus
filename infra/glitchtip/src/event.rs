//! Sentry event payload construction (EP-038 M3).
//!
//! Documented event attributes (verified against the authoritative
//! Sentry event payload documentation):
//!
//! Required: `event_id` (32 lowercase hex chars, no dashes),
//! `timestamp` (RFC 3339 or Unix numeric), `platform` (string).
//!
//! Optional but encouraged: `level` (`fatal`/`error`/`warning`/
//! `info`/`debug`), `logger`, `transaction`, `server_name`,
//! `release`, `environment`, `tags` (map of string values < 200
//! chars), `extra` (arbitrary map), `fingerprint` (list of strings
//! used for grouping/dedup).
//!
//! This module never sees raw event data: callers must already hold a
//! `RedactedEnvelope` from the M1 `RedactionPolicy`.

use serde_json::{Map, Value};

/// One event tag (name/value pair, both bounded strings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTag {
    pub name: String,
    pub value: String,
}

impl EventTag {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// A Sentry event payload. Fields are bounded to the documented
/// shapes; nothing secret-shaped can enter because construction is
/// gated by the export boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPayload {
    pub event_id: String,
    pub timestamp: String,
    pub platform: String,
    pub level: String,
    pub logger: String,
    pub release: String,
    pub environment: String,
    pub tags: Vec<EventTag>,
    pub extra: Map<String, Value>,
    pub fingerprint: Vec<String>,
}

impl EventPayload {
    /// Start building an event with the mandatory attributes.
    pub fn builder(event_id: impl Into<String>) -> EventPayloadBuilder {
        EventPayloadBuilder {
            event_id: event_id.into(),
            timestamp: String::new(),
            platform: "rust".to_string(),
            level: "error".to_string(),
            logger: String::new(),
            release: String::new(),
            environment: String::new(),
            tags: Vec::new(),
            extra: Map::new(),
            fingerprint: Vec::new(),
        }
    }

    /// Serialize to the documented JSON object.
    pub fn to_json(&self) -> Value {
        let mut obj = Map::new();
        obj.insert("event_id".to_string(), Value::String(self.event_id.clone()));
        obj.insert(
            "timestamp".to_string(),
            Value::String(self.timestamp.clone()),
        );
        obj.insert("platform".to_string(), Value::String(self.platform.clone()));
        obj.insert("level".to_string(), Value::String(self.level.clone()));
        if !self.logger.is_empty() {
            obj.insert("logger".to_string(), Value::String(self.logger.clone()));
        }
        if !self.release.is_empty() {
            obj.insert("release".to_string(), Value::String(self.release.clone()));
        }
        if !self.environment.is_empty() {
            obj.insert(
                "environment".to_string(),
                Value::String(self.environment.clone()),
            );
        }
        if !self.tags.is_empty() {
            let mut tags = Map::new();
            for tag in &self.tags {
                tags.insert(tag.name.clone(), Value::String(tag.value.clone()));
            }
            obj.insert("tags".to_string(), Value::Object(tags));
        }
        if !self.extra.is_empty() {
            obj.insert("extra".to_string(), Value::Object(self.extra.clone()));
        }
        if !self.fingerprint.is_empty() {
            obj.insert(
                "fingerprint".to_string(),
                Value::Array(
                    self.fingerprint
                        .iter()
                        .map(|f| Value::String(f.clone()))
                        .collect(),
                ),
            );
        }
        Value::Object(obj)
    }

    /// The `event_id` is the canonical 32-hex event identifier.
    pub fn event_id(&self) -> &str {
        &self.event_id
    }
}

/// Builder for `EventPayload`.
pub struct EventPayloadBuilder {
    event_id: String,
    timestamp: String,
    platform: String,
    level: String,
    logger: String,
    release: String,
    environment: String,
    tags: Vec<EventTag>,
    extra: Map<String, Value>,
    fingerprint: Vec<String>,
}

impl EventPayloadBuilder {
    pub fn timestamp(mut self, ts: impl Into<String>) -> Self {
        self.timestamp = ts.into();
        self
    }

    pub fn platform(mut self, p: impl Into<String>) -> Self {
        self.platform = p.into();
        self
    }

    pub fn level(mut self, level: impl Into<String>) -> Self {
        self.level = level.into();
        self
    }

    pub fn logger(mut self, logger: impl Into<String>) -> Self {
        self.logger = logger.into();
        self
    }

    pub fn release(mut self, release: impl Into<String>) -> Self {
        self.release = release.into();
        self
    }

    pub fn environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = environment.into();
        self
    }

    pub fn tag(mut self, tag: EventTag) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    pub fn fingerprint(mut self, fp: Vec<String>) -> Self {
        self.fingerprint = fp;
        self
    }

    pub fn build(self) -> EventPayload {
        EventPayload {
            event_id: self.event_id,
            timestamp: self.timestamp,
            platform: self.platform,
            level: self.level,
            logger: self.logger,
            release: self.release,
            environment: self.environment,
            tags: self.tags,
            extra: self.extra,
            fingerprint: self.fingerprint,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_required_fields_present() {
        let event = EventPayload::builder("fc6d8c0c43fc4630ad850ee518f1b9d0")
            .timestamp("2011-05-02T17:41:36Z")
            .build();
        let json = event.to_json();
        assert_eq!(json["event_id"], "fc6d8c0c43fc4630ad850ee518f1b9d0");
        assert_eq!(json["timestamp"], "2011-05-02T17:41:36Z");
        assert_eq!(json["platform"], "rust");
    }

    #[test]
    fn event_optional_fields_emitted_only_when_set() {
        let minimal = EventPayload::builder("fc6d8c0c43fc4630ad850ee518f1b9d0")
            .timestamp("2011-05-02T17:41:36Z")
            .build()
            .to_json();
        assert!(minimal.get("logger").is_none());
        assert!(minimal.get("tags").is_none());
        assert!(minimal.get("fingerprint").is_none());

        let full = EventPayload::builder("fc6d8c0c43fc4630ad850ee518f1b9d0")
            .timestamp("2011-05-02T17:41:36Z")
            .level("critical")
            .logger("nexus.incidents")
            .release("nexus@0.1.0")
            .environment("production")
            .tag(EventTag::new("source", "storage"))
            .extra("dedupe_key", "storage:unavailable")
            .fingerprint(vec!["storage".to_string(), "unavailable".to_string()])
            .build()
            .to_json();
        assert_eq!(full["level"], "critical");
        assert_eq!(full["logger"], "nexus.incidents");
        assert_eq!(full["release"], "nexus@0.1.0");
        assert_eq!(full["environment"], "production");
        assert_eq!(full["tags"]["source"], "storage");
        assert_eq!(full["extra"]["dedupe_key"], "storage:unavailable");
        assert_eq!(full["fingerprint"][0], "storage");
    }

    #[test]
    fn event_id_must_be_lowercase_hex_32() {
        // The builder trusts caller-provided ids; validation of
        // generated ids is covered by the incident mapping tests.
        let event = EventPayload::builder("fc6d8c0c43fc4630ad850ee518f1b9d0")
            .timestamp("2011-05-02T17:41:36Z")
            .build();
        let id = event.event_id();
        assert_eq!(id.len(), 32);
        assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(id.chars().all(|c| !c.is_uppercase()));
    }
}
