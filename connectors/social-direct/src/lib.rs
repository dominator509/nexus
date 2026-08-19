//! EP-029 direct platform connector (SPEC-015; M3).
//!
//! Real production direct official API adapter behind the nexus-social
//! `SocialProvider` and `DirectPlatformProvider` ports implementing the
//! strategic gaps that the Postiz sidecar does not cover: community
//! inbox (conversations), analytics (metrics), and CRM lead handoff
//! (leads) via the DOCUMENTED X API v2 surface (SPEC-015 behavior 4:
//! direct official APIs implement strategic gaps).
//!
//! Permanent invariants (SPEC-015):
//! - Direct official APIs are replaceable (behavior 4).
//! - Social leads link to Hydra only through deterministic or
//!   human-reviewed resolution (behavior 6).
//! - Analytics preserve attribution.
//! - Policy before mutation; denied actions make ZERO provider calls.
//! - Unbound providers fail closed (Reality rule).

#![forbid(unsafe_code)]

pub mod adapter;
pub mod transport;

pub use adapter::{DirectAuditEntry, DirectPlatformAdapter};
pub use transport::{
    DirectPlatformTransport, HttpDirectPlatformTransport, XCreateResponse, XMention,
    XPublicMetrics, XTweet, XUser,
};
