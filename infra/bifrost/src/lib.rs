//! EP-013 M2 Bifrost-preferred model gateway adapter (SPEC-009).
//!
//! A real `ModelGateway` implementation with deterministic routing,
//! budgets, retries, rate limits, fallbacks, and usage accounting.
//! Bifrost is the preferred gateway but is hidden behind the
//! `ModelGateway` contract; direct providers remain available for
//! replacement and diagnostics.
//!
//! Layering: this crate is an infrastructure adapter. It imports the
//! application ports (`nexus-model-gateway`) and never the reverse.
//! I/O for actual provider calls sits behind the `ModelProvider` and
//! `ModelBudget` ports; the real HTTP transports for Bifrost and
//! direct providers are wired in EP-013 M3 (`config/models/`).
//!
//! Authority boundary: the gateway never treats model output as
//! authority. Budget exhaustion, rate limits, missing providers, and
//! provider failures fail closed with typed SPEC-006 errors. Provider
//! credentials are referenced by id, never stored or logged here.

#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod gateway;
pub mod router;
pub mod telemetry;

pub use config::{BifrostConfig, RateLimitPolicy, RetryPolicy};
pub use error::BifrostError;
pub use gateway::{BifrostGateway, BifrostGatewayBuilder};
pub use router::{BifrostRouter, RouterInput};
pub use telemetry::{GatewayEvent, GatewayEventClass, GatewayTelemetry};
