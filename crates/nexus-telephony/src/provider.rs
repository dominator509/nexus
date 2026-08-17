//! EP-025 provider ports (fail-closed defaults; SPEC-014).
//!
//! Asterisk 22 LTS is the telephony gateway; SIP carriers are
//! providers (SPEC-014 behavior 3). Nexus orchestrates Asterisk, it
//! does not replace SIP signaling, RTP, codecs, TLS, or SRTP with a
//! home-grown stack (directive 3). Unbound providers fail closed and
//! never fabricate call state (Reality rule).

use crate::error::CallError;
use crate::vocabulary::{CallCapability, CallSessionId, CallState, CarrierId, SipEndpointId};

/// Telephony provider port (provider-neutral; SPEC-014
/// CommunicationRouter uses this boundary).
pub trait TelephonyProvider {
    fn list_sessions(&self) -> Result<Vec<CallSessionId>, CallError> {
        Err(CallError::unavailable(
            "telephony provider has no implementation bound",
        ))
    }

    fn capabilities(&self, session: &CallSessionId) -> Result<Vec<CallCapability>, CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "telephony provider has no implementation bound",
        ))
    }

    fn session_state(&self, session: &CallSessionId) -> Result<CallState, CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "telephony provider has no implementation bound",
        ))
    }
}

/// Asterisk provider port: real operations on real Asterisk channels
/// (directive 11: no in-memory CallSession mutation may substitute for
/// actual channel state).
pub trait AsteriskProvider {
    fn originate(
        &self,
        endpoint: &SipEndpointId,
        context: &str,
        extension: &str,
    ) -> Result<CallSessionId, CallError> {
        let _ = (endpoint, context, extension);
        Err(CallError::unavailable(
            "asterisk provider has no implementation bound",
        ))
    }

    fn answer(&self, session: &CallSessionId) -> Result<(), CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "asterisk provider has no implementation bound",
        ))
    }

    fn hangup(&self, session: &CallSessionId) -> Result<(), CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "asterisk provider has no implementation bound",
        ))
    }

    fn bridge(&self, session: &CallSessionId, other: &CallSessionId) -> Result<(), CallError> {
        let _ = (session, other);
        Err(CallError::unavailable(
            "asterisk provider has no implementation bound",
        ))
    }

    fn transfer(&self, session: &CallSessionId, target: &str) -> Result<(), CallError> {
        let _ = (session, target);
        Err(CallError::unavailable(
            "asterisk provider has no implementation bound",
        ))
    }

    fn send_dtmf(&self, session: &CallSessionId, digits: &str) -> Result<(), CallError> {
        let _ = (session, digits);
        Err(CallError::unavailable(
            "asterisk provider has no implementation bound",
        ))
    }

    fn hold(&self, session: &CallSessionId) -> Result<(), CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "asterisk provider has no implementation bound",
        ))
    }

    fn channel_state(&self, session: &CallSessionId) -> Result<CallState, CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "asterisk provider has no implementation bound",
        ))
    }
}

/// SIP carrier provider port: carrier configuration adapters
/// (directive 26: never hard-code Nexus to one carrier; future
/// carriers must not require rewriting Nexus call semantics).
pub trait SipCarrierProvider {
    fn list_carriers(&self) -> Result<Vec<CarrierId>, CallError> {
        Err(CallError::unavailable(
            "sip carrier provider has no implementation bound",
        ))
    }

    fn carrier_available(&self, carrier: &CarrierId) -> Result<bool, CallError> {
        let _ = carrier;
        Err(CallError::unavailable(
            "sip carrier provider has no implementation bound",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::CallDirection;

    #[test]
    fn ep025_unit_provider_ports_fail_closed() {
        struct Unbound;
        impl TelephonyProvider for Unbound {}
        impl AsteriskProvider for Unbound {}
        impl SipCarrierProvider for Unbound {}

        let tel = Unbound;
        assert_eq!(
            tel.list_sessions().unwrap_err().code,
            crate::error::CallErrorCode::Unavailable
        );
        let session = CallSessionId::new("s/1").unwrap();
        assert_eq!(
            tel.session_state(&session).unwrap_err().code,
            crate::error::CallErrorCode::Unavailable
        );
        assert_eq!(
            tel.capabilities(&session).unwrap_err().code,
            crate::error::CallErrorCode::Unavailable
        );

        let endpoint = SipEndpointId::new("endpoint-a").unwrap();
        assert!(Unbound.originate(&endpoint, "internal", "100").is_err());
        assert!(Unbound.answer(&session).is_err());
        assert!(Unbound.hangup(&session).is_err());
        assert!(Unbound.bridge(&session, &session).is_err());
        assert!(Unbound.transfer(&session, "200").is_err());
        assert!(Unbound.send_dtmf(&session, "1").is_err());
        assert!(Unbound.hold(&session).is_err());
        assert!(Unbound.channel_state(&session).is_err());

        let carrier = CarrierId::new("carrier-1").unwrap();
        assert!(Unbound.list_carriers().is_err());
        assert!(Unbound.carrier_available(&carrier).is_err());

        // Compile-time proof that the vocabulary is visible from the
        // port layer without importing the provider.
        let _dir = CallDirection::Outbound;
    }
}
