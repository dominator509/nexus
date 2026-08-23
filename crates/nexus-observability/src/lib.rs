//! nexus-observability: provider-neutral observability contracts (SPEC-007;
//! EP-038 M1).
//!
//! This crate owns the canonical telemetry model: context envelope,
//! fail-closed redaction, bounded metric catalog, trace policy, health
//! aggregation, incident sink, fleet health, and SLO evaluation.
//!
//! M1 is the contract layer only. No exporter, collector, or provider
//! adapter exists in M1; Prometheus/Grafana/OpenTelemetry/GlitchTip and
//! incident delivery are NOT asserted until later milestones.
//!
//! Permanent invariants encoded here and proven by tests:
//! - OBSERVED RAW EVENT != EXPORTABLE TELEMETRY (redaction before egress)
//! - TRACE ID PRESENT != TRACE EXPORTED != TRACE SAFE
//! - CONFIGURED != REACHABLE != RESPONDING != READY != HEALTHY
//! - LAST KNOWN HEALTHY != CURRENTLY HEALTHY (staleness is visible)
//! - NO EVENTS != SLO MET; NO ALERTS != SYSTEM HEALTHY

pub mod error;
pub mod model;
pub mod port;
pub mod vocabulary;

pub use error::{ObservabilityError, ObservabilityErrorCode, ObservabilityResult};
pub use model::{
    ComponentHealth, CompositeHealthAggregator, FleetHealth, FleetSummary, HealthReport, Incident,
    IncidentDeliveryResult, MetricDefinition, MetricRegistry, NodeHealthReport,
    RecordingIncidentSink, RedactedEnvelope, RedactionPolicy, SloDefinition, SloEvaluation,
    TelemetryContext, TraceContext, TraceExportDecision, TracePolicy, WindowedSloEvaluator,
};
pub use port::{HealthAggregator, IncidentSink, MetricCatalog, PortSurface, SloEvaluator};
pub use vocabulary::{
    CardinalityPolicy, HealthState, IncidentState, MetricKind, RedactionAction, Severity, SloState,
    StabilityLevel, TelemetrySignal,
};
