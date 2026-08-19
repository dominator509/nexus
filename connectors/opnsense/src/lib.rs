//! EP-030 OPNsense connector (SPEC-013; M2).
//!
//! Real production OPNsense adapter behind the nexus-sentinel
//! `FirewallProvider` port. OPNsense is the primary serious firewall
//! (SPEC-013 behavior 2; COMPONENT_REGISTRY external-appliance,
//! BSD-2-Clause). Nexus orchestrates its documented firewall
//! automation API and normalizes provider payloads at the
//! infrastructure boundary - OPNsense JSON never becomes a domain
//! contract.
//!
//! Permanent invariants (SPEC-013):
//! - OPNsense and OpenWrt share the canonical FirewallProvider
//!   contract (acceptance obligation 1).
//! - Automated containment is limited to preauthorized
//!   high-confidence reversible rules and always notifies the owner
//!   (behavior 5); destructive remediation requires human procedure
//!   (behavior 6).
//! - A quarantine proposal is DATA until approved, applied, and
//!   verified; verification binds to the exact rule/device by
//!   independent readback.
//! - Policy before mutation; denied actions make ZERO provider calls.
//! - Unbound providers fail closed (Reality rule).

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::OpnsenseFirewallProvider;
pub use observability::{SentinelAuditEntry, SentinelObservability};
pub use transport::{HttpOpnsenseTransport, OpnsenseRule, OpnsenseRulePayload, OpnsenseTransport};
