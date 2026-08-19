//! EP-028 M5 live-fire evidence crate (SPEC-015; LF-015, LF-025).
//!
//! Drives the REAL production Hydra adapter + HTTP transport against a
//! controlled local HTTP fixture over REAL std::net sockets emitting
//! REAL Hydra-shaped responses (the versioned canonical surface from
//! schemas/hydra/). Mocks control the peer only; the transport and
//! adapter under test are never mocked.
//!
//! Certification boundary: this proves the end-to-end governed
//! business-control seam (context projection, governed action, event
//! consumption, CEO brief provenance/freshness) over the canonical
//! surface. It does NOT certify a real Hydra/CRM provider (no
//! component selected in COMPONENT_REGISTRY; real provider
//! certification is DEFERRED with owner recorded; Postiz is EP-029's
//! node).
//!
//! Each proof writes current-run machine-readable evidence under
//! `.agent/state/evidence/` embedding `EP028_M5_RUN_ID` (stale
//! evidence never satisfies the M5 gate).

pub mod fixture;
