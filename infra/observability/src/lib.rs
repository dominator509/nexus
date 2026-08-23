//! EP-038 M4 -- observability/ops runtime (SPEC-007; node contract).
//!
//! This crate is the operations surface that composes the closed
//! milestone layers into one fail-closed runtime:
//!
//! - M1 contracts: `RedactionPolicy`, `MetricRegistry`,
//!   `CompositeHealthAggregator`, `WindowedSloEvaluator`,
//!   `RecordingIncidentSink` (provider-neutral recording) and the
//!   `RedactedEnvelope` export boundary.
//! - M2 writers: structured JSON log lines, Prometheus text 0.0.4,
//!   OTLP/JSON serialization (local fallback per the node contract).
//! - M3 provider: `GlitchTipIncidentSink` for real incident delivery
//!   when a DSN is configured.
//!
//! M4's job is to prove this stack FAILS SAFELY under dependency,
//! policy, security, and resource faults. Permanent invariants:
//!
//! - RAW EVENT != SAFE TO EXPORT: every observed field passes through
//!   `RedactionPolicy` before any writer/sink sees it.
//! - CONFIGURED != REACHABLE != RESPONDING != READY != HEALTHY: the
//!   ops diagnostic and the health aggregator never promote a weaker
//!   observation to a stronger claim.
//! - NO EVENTS != SLO MET; NO ALERTS != SYSTEM HEALTHY: silence is
//!   never success.
//! - Delivery failure is classified (Unavailable/Timeout/Authorization/
//!   ExternalProvider), never collapsed into a generic send failure,
//!   and the incident is retained (quarantined) for bounded recovery.
//! - Recovery is deadline-bounded with an attempt counter and last
//!   observed failure; a budget-exhausted recovery fails closed.
//!
//! Dependency direction: this crate imports the M1 contracts and the
//! M2/M3 providers; nothing imports it for telemetry.

pub mod audit;
pub mod diag;
pub mod recovery;
pub mod runtime;

pub use audit::{AuditRecord, AuditSeverity};
pub use diag::{OpsDiagnostic, StackState};
pub use recovery::{recover_with_budget, RecoveryBudget, RecoveryOutcome};
pub use runtime::{fields, ops_metric_definitions, ObservabilityRuntime, RuntimeConfig};
