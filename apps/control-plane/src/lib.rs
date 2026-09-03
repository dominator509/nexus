//! Nexus Control Plane Runtime (EP-044).
//!
//! The real runnable control-plane server binary. Owns the canonical
//! runtime endpoints (`/healthz`, `/readyz`, `/v1/capabilities`), the
//! application composition root, canonical runtime configuration, base
//! URL/domain resolution, graceful startup/shutdown, runtime smoke
//! ownership, and local deterministic runtime bring-up.
//!
//! Vocabulary-locked names (ADR-019): `ControlPlaneConfig`,
//! `RuntimeHealth`, `RuntimeReadiness`, `CapabilityList`,
//! `ControlPlaneServer`, `RuntimeLifecycle`, `RuntimeSmoke`.

#![forbid(unsafe_code)]

pub mod capabilities;
pub mod composition;
pub mod config;
pub mod error;
pub mod health;
pub mod lifecycle;
pub mod readiness;
pub mod server;
pub mod smoke;
pub mod telemetry;
pub mod vocabulary;

pub use capabilities::{CapabilityList, CapabilityListSource};
pub use composition::{MemoryArtifactStore, MemoryOutbox, RuntimeComposition};
pub use config::{ControlPlaneConfig, ControlPlaneConfigError};
pub use error::{RuntimeError, RuntimeErrorCode};
pub use health::RuntimeHealth;
pub use lifecycle::{RuntimeLifecycle, RuntimeLifecycleError};
pub use readiness::RuntimeReadiness;
pub use server::{ControlPlaneServer, ControlPlaneServerError};
pub use smoke::{RuntimeSmoke, RuntimeSmokeError};
pub use telemetry::RuntimeTelemetry;
pub use vocabulary::{RuntimeState, RuntimeVocabularyError};
