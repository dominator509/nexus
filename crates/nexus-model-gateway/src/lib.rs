//! EP-013 model gateway and provider registry contract crate (SPEC-009).
//!
//! Provider-neutral contracts for the model plane: `ModelProvider`,
//! `ModelGateway`, `ProviderRegistry`, `ProviderHealth`, `ModelBudget`,
//! `ModelRequest`, `ModelResponse`, and `ToolCallEnvelope`. Bifrost is
//! the preferred gateway implementation but is hidden behind the
//! `ModelGateway` contract; direct DeepSeek and OpenAI-compatible
//! adapters remain available for replacement and diagnostics.
//!
//! Authority boundary: models and gateways carry intelligence, never
//! authority. A model cannot grant scopes, approve actions, modify
//! policies, reveal secrets, or bypass output validation (SPEC-009
//! required behavior 10). Provider credentials never leave the
//! gateway; adapters reference credentials by id, never by value.

#![forbid(unsafe_code)]

pub mod budget;
pub mod error;
pub mod gateway;
pub mod health;
pub mod model;
pub mod provider;
pub mod registry;
pub mod vocabulary;

pub use budget::{BudgetDecision, BudgetLedger, ModelBudget};
pub use error::{ModelGatewayError, ModelGatewayErrorCode};
pub use gateway::{ModelGateway, ModelRoute, ModelRouteDecision};
pub use health::ProviderHealth;
pub use model::{ModelRequest, ModelResponse, NexusControlObject, PromptSegment, ToolCallEnvelope};
pub use provider::ModelProvider;
pub use registry::ProviderRegistry;
pub use vocabulary::{
    CacheHitRatio, EffortTier, Escalation, Microbrain, ModelGatewayClass,
    ModelGatewayVocabularyError, ModelRouteClass, ProviderHealthState, ProviderKind,
    ReflexProviderClass,
};
