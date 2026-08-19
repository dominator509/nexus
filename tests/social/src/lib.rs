//! EP-029 M5 live-fire evidence crate (SPEC-015; LF-014, LF-027).
//!
//! Drives the REAL production Postiz adapter + direct-platform adapter
//! (with their real HTTP transports) against controlled local fixtures
//! over REAL std::net sockets emitting REAL Postiz-shaped and REAL
//! X API v2-shaped responses (documented surfaces; anti-hallucination -
//! no invented vendor endpoints). Mocks control the peer only; the
//! transports and adapters under test are never mocked.
//!
//! Certification boundary: this proves the end-to-end social command
//! center (variants, approvals, publish acceptance, engagement,
//! attribution, lead handoff, governed replies) over the canonical
//! documented surfaces. It does NOT certify a real Postiz or real X
//! provider (no owned account/credentials exist in this environment;
//! real provider certification is DEFERRED with owner recorded at
//! M5/deployment).
//!
//! Each proof writes current-run machine-readable evidence under
//! `.agent/state/evidence/` embedding `EP029_M5_RUN_ID` (stale
//! evidence never satisfies the M5 gate).

pub mod fixture;
