//! EP-030 OpenWrt connector (SPEC-013; M3).
//!
//! Real production OpenWrt adapter behind the nexus-sentinel
//! `FirewallProvider` port. OpenWrt is supported for embedded and
//! consumer installations (SPEC-013 behavior 2; COMPONENT_REGISTRY
//! external-appliance, GPL-2.0). Nexus orchestrates the documented
//! ubus HTTP JSON-RPC surface and normalizes provider payloads at the
//! infrastructure boundary - OpenWrt JSON never becomes a domain
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

pub use adapter::OpenWrtFirewallProvider;
pub use observability::{SentinelAuditEntry, SentinelObservability};
pub use transport::{HttpOpenWrtTransport, OpenWrtRule, OpenWrtRulePayload, OpenWrtTransport};
