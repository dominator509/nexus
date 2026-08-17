//! EP-025 Asterisk connector (SPEC-014; M2/M3/M4).
//!
//! Provider-neutral telephony adapter core behind the nexus-telephony
//! ports, using the REAL Asterisk 22 ARI HTTP surface. Nexus
//! orchestrates Asterisk through ARI; it does NOT implement SIP
//! signaling, RTP, codecs, TLS, or SRTP itself (directive 3).
//!
//! Classification:
//! - EP-025 Asterisk adapter: REAL_PRODUCTION_IMPLEMENTATION
//! - Asterisk 22.10.1 (pinned image): REAL_EXTERNAL_DEPENDENCY
//!   (certified at M3 with a real container)
//! - SIP test endpoints: CONTROLLED_TEST_FIXTURE
//! - external SIP carrier / PSTN: NOT CERTIFIED until a real carrier
//!   is exercised (directive 25; later-owned)

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::{map_channel_state, session_from_channel, AsteriskAdapter};
pub use observability::{TelephonyAuditEntry, TelephonyObservability};
pub use transport::{
    classify_status_pub, AriCallerId, AriChannel, AriDialplan, AriTransport, ChannelSelector,
    RestAriTransport,
};
