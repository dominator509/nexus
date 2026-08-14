//! EP-013 M3 real model transport (SPEC-009).
//!
//! A real HTTP provider adapter behind the `ModelProvider` port. It
//! speaks the OpenAI-compatible chat completions protocol used by
//! Bifrost (preferred gateway) and DeepSeek V4 Flash (V1 primary
//! ReflexProvider). Provider manifests live under
//! `config/models/providers/` with exact component versions.
//!
//! Authority boundary: this transport only moves request/response
//! bytes. It never grants authority, never treats model output as
//! authorization, and never persists credentials. Credentials are
//! referenced by id in manifests and resolved by the caller; the
//! transport never logs a credential value.

#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod transport;

pub use config::{ProviderManifest, ProviderManifestSet};
pub use error::TransportError;
pub use transport::{OpenAiCompatibleTransport, OpenAiCompatibleTransportBuilder};
