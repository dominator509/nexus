//! EP-020 Home Assistant provider adapter (SPEC-011; ADR-027).
//!
//! Real production adapter behind the `nexus-home` ports: real REST
//! transport, discovery, canonical mapping, local fast path,
//! exact-target verification, reconnect/resubscribe, offline queueing,
//! and automation handoff. Physical device commands use the real HA
//! service/action mechanism; `POST /api/states/<entity_id>` is never
//! used to implement physical control.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod transport;

pub use adapter::{
    default_fast_path_decision, stable_device_id, verification_rule_for, AutomationHandoffAdapter,
    HomeAssistantAdapter,
};
pub use transport::{HaEntityState, HaService, HaTransport, RestTransport};
