//! EP-030 provider-neutral sentinel core contracts (SPEC-013).
//!
//! Sentinel Core uses firewall telemetry, AdGuard DNS, inventory,
//! expected destinations, flow baselines, identity events, and Nexus
//! system events. OPNsense is the primary serious firewall; OpenWrt is
//! supported for embedded and consumer installations; AdGuard Home is
//! the DNS security default. This crate owns the provider-neutral
//! contract layer; connector implementations live under
//! `connectors/opnsense`, `connectors/openwrt`, and
//! `connectors/adguard-home` (M2+). M1 owns the vocabulary, value
//! objects, and fail-closed provider ports.
//!
//! Permanent invariants (SPEC-013):
//! - Every device has expected protocols, destinations, internal
//!   access, baseline, owner, firmware, provider, and trust class.
//! - Automated containment is limited to preauthorized high-confidence
//!   reversible rules and always notifies the owner.
//! - Destructive remediation, credential rotation, wipes, factory
//!   resets, and broad lockouts require human procedure.
//! - A provider advertises only capabilities it actually holds;
//!   unbound providers fail closed and never fabricate devices,
//!   findings, baselines, or containment (Reality rule).
//! - Free-form provider payloads are normalized at the infrastructure
//!   boundary and never become domain contracts.
//!
//! Dependency direction: this crate depends only on nexus-domain
//! (contract crate) and serde/serde_json. Provider implementations
//! never appear here.

#![forbid(unsafe_code)]

pub mod capability;
pub mod digest;
pub mod error;
pub mod model;
pub mod provider;
pub mod vocabulary;

pub use capability::{SentinelCapabilityKind, SentinelCapabilityMap};
pub use error::{SentinelError, SentinelErrorCode};
pub use model::{
    BehaviorBaseline, ContainmentVerification, DeviceFingerprint, DnsBlocklistEntry, DnsTelemetry,
    NetworkDevice, NetworkFinding, QuarantineApproval, QuarantineProposal,
};
pub use provider::{
    DnsSecurityProvider, FirewallProvider, NetworkInventory, UnboundDnsSecurityProvider,
    UnboundFirewallProvider, UnboundNetworkInventory,
};
pub use vocabulary::{
    BaselineId, BehaviorBaselineState, DeviceFingerprintId, FindingKind, FindingSeverity,
    FindingState, FirewallAction, NetworkDeviceId, NetworkFindingId, NetworkSegment,
    QuarantineProposalId, QuarantineState, SentinelProfile, SentinelVocabularyError, TrustClass,
};
