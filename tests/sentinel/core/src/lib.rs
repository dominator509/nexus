//! EP-030 sentinel core contract-composition tests (SPEC-013).
//!
//! M1 owns the provider-neutral contracts; this crate composes the
//! contracts and proves the acceptance obligations that are
//! contract-level:
//! - OPNsense and OpenWrt share a canonical network provider
//!   (FirewallProvider port).
//! - AdGuard Home supplies DNS security and telemetry
//!   (DnsSecurityProvider port).
//! - IoT, trusted, guest, camera, and quarantine segments are modeled.
//! - Core Sentinel is light enough for a normal home and can propose
//!   verified containment (QuarantineProposal ladder, fail-closed
//!   unbound providers).
//!
//! Provider behavior lives in connectors/ (M2+); this crate proves the
//! contracts compose without provider implementation.
