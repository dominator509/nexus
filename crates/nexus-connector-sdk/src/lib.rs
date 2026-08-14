//! Nexus connector SDK contract (EP-011 M1).
//!
//! The SDK is the client surface of the universal connector contract
//! (SPEC-022): the same typed capability surface that the Rust,
//! TypeScript, and Python SDKs implement, plus the sandboxed legacy
//! Connector Sidecar ports (`SidecarAdapter`, `LegacyPoller`,
//! `WebhookNormalizer`) and the `CredentialBroker` reference boundary.
//!
//! This crate is the provider-neutral contract only. Language-specific
//! bindings are generated from the canonical schemas under `schemas/`
//! (SPEC-022 behavior 4) and real sidecar transports are proven in
//! later EP-011 milestones. The SDK surface never grants authority:
//! authorization to invoke remains EP-008's boundary and discovery
//! remains EP-010's boundary.

#![forbid(unsafe_code)]

pub mod credential;
pub mod error;
pub mod legacy;
pub mod sdk;
pub mod sidecar;
pub mod vocabulary;
pub mod webhook;

pub use credential::{CredentialBroker, CredentialBrokerError, CredentialReference};
pub use error::{SdkError, SdkErrorCode};
pub use legacy::{LegacyPoller, LegacyPollerError, PolledBatch};
pub use sdk::{
    CONTRACT_VERSION, ConnectorSdk, PythonConnectorSdk, RustConnectorSdk, TypeScriptConnectorSdk,
};
pub use sidecar::{SidecarAdapter, SidecarAdapterError, SidecarRequest, SidecarResponse};
pub use vocabulary::{
    LegacyTransport, SdkLanguage, SidecarTransport, WebhookDeliveryState, WebhookEvent,
    WebhookSignature, WebhookVerification,
};
pub use webhook::{WebhookNormalizer, WebhookNormalizerError};

/// Version of the SDK contract corpus shared by Rust, TypeScript, and
/// Python bindings (SPEC-022 behavior 4: one conformance suite).
pub const SDK_CONTRACT_VERSION: &str = "1.0.0";
