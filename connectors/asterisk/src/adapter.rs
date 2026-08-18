//! EP-025 Asterisk adapter core (SPEC-014; M2).
//!
//! Real production adapter behind the nexus-telephony
//! `TelephonyProvider` / `AsteriskProvider` ports: real ARI channel
//! discovery, canonical state mapping from REAL Asterisk channel
//! state, capability-gated command dispatch, exact-target
//! verification, in-flight idempotency, bounded observability
//! (redacted audit ring, counters, correlation), and fail-closed
//! behavior.
//!
//! Permanent invariants (owner directive, EP-025):
//!
//! - CALL REQUESTED != SIP INVITE SENT != REMOTE RINGING != ANSWERED
//!   != MEDIA ESTABLISHED != TWO-WAY AUDIO VERIFIED != CALL COMPLETED.
//! - SIP SIGNALING IS NOT MEDIA CERTIFICATION: channel.state "Up"
//!   proves ANSWERED (signaling), never audio. BRIDGED requires a
//!   real Asterisk bridge id on the channel. MEDIA_ESTABLISHED /
//!   TWO_WAY_AUDIO_VERIFIED come only from the media bridge proof
//!   (M3/M5), never from channel state alone.
//! - A command on session A is verified ONLY by an observed state
//!   transition on session A (exact target; unrelated change never
//!   verifies).
//! - Unknown sessions are NotFound, never Verified and never benign.
//! - Unsupported/unpermitted commands fail closed (Policy) BEFORE any
//!   provider mutation.
//! - UNKNOWN OUTCOME -> VERIFY FIRST -> NO BLIND RETRY (directive 22).
//! - Caller ID and SIP display identity are advisory inputs only;
//!   they never authenticate a Nexus user or bypass EP-008
//!   (directive 16).
//! - Every operation records a correlation id; observability is
//!   bounded and poison-safe (secrets redacted at insert; raw audio
//!   and SIP Authorization headers never enter telemetry).
//!
//! No test-mode branches exist in production code.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use nexus_telephony::{
    AsteriskProvider, CallCapability, CallCommand, CallDirection, CallError, CallErrorCode,
    CallLeg, CallLegId, CallPolicy, CallSession, CallSessionId, CallState, CallVerification,
    CallVerifier, MediaState, SipEndpointId, TelephonyProvider,
};

use crate::observability::TelephonyObservability;
use crate::transport::{AriBridge, AriChannel, AriEndpoint, AriTransport, ChannelSelector};

/// Canonical mapping from REAL Asterisk ARI channel state strings to
/// the locked Nexus call state ladder.
///
/// Only DOCUMENTED ARI channel states are mapped. An unrecognized
/// state fails closed (External) - it is NEVER mapped to a safe or
/// fabricated state (directive E precedent).
pub fn map_channel_state(channel: &AriChannel) -> Result<CallState, CallError> {
    match channel.state.as_str() {
        "Up" => {
            if channel.bridge.is_some() {
                Ok(CallState::Bridged)
            } else {
                Ok(CallState::Answered)
            }
        }
        "Ring" | "Ringing" => Ok(CallState::Ringing),
        "Dialing" => Ok(CallState::InviteSent),
        "Busy" => Ok(CallState::Busy),
        "Down" => Ok(CallState::Requested),
        other => Err(CallError::new(
            CallErrorCode::External,
            format!("unrecognized ARI channel state {other:?}"),
            None,
            None,
        )),
    }
}

/// Build a CallSession from a real ARI channel (stable identity from
/// the real channel id - never enumeration order).
pub fn session_from_channel(channel: &AriChannel) -> Result<CallSession, CallError> {
    let state = map_channel_state(channel)?;
    let peer_name = channel
        .connected
        .as_ref()
        .map(|c| c.number.clone())
        .unwrap_or_else(|| channel.name.clone());
    let peer = SipEndpointId::new(peer_name)?;
    let mut session = CallSession::new(
        CallSessionId::new(channel.id.clone())?,
        CallDirection::Inbound,
        peer,
        Some(format!("ari-channel-{}", channel.id)),
        false,
        false,
    );
    session.state = state;
    let leg = CallLeg::new(
        CallLegId::new(format!("leg-{}", channel.id))?,
        session.id.clone(),
        SipEndpointId::new(channel.name.clone())?,
        state,
    );
    session.add_leg(leg);
    Ok(session)
}

/// In-flight idempotency entry for one command on one target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    command: String,
}

/// Real production adapter over a real ARI transport.
pub struct AsteriskAdapter {
    transport: Box<dyn AriTransport>,
    policy: CallPolicy,
    obs: Mutex<TelephonyObservability>,
    in_flight: Mutex<HashMap<String, InFlightEntry>>,
    verifier: CallVerifier,
    /// Bounded window for verifying real bridge membership after an
    /// addChannel 200/204 (ARI propagates the channel's bridge field
    /// asynchronously). Unit tests shrink this to keep fail-closed
    /// paths fast.
    bridge_verify_timeout: Duration,
}

impl AsteriskAdapter {
    pub fn new(transport: Box<dyn AriTransport>, policy: CallPolicy) -> Self {
        Self {
            transport,
            policy,
            obs: Mutex::new(TelephonyObservability::default()),
            in_flight: Mutex::new(HashMap::new()),
            verifier: CallVerifier,
            bridge_verify_timeout: Duration::from_secs(8),
        }
    }

    /// Test helper: shrink the bounded bridge-membership verification
    /// window (production default is 8s for real async propagation).
    #[cfg(test)]
    pub fn with_bridge_verify_timeout(mut self, timeout: Duration) -> Self {
        self.bridge_verify_timeout = timeout;
        self
    }

    pub fn with_observability(
        transport: Box<dyn AriTransport>,
        policy: CallPolicy,
        obs: TelephonyObservability,
    ) -> Self {
        Self {
            transport,
            policy,
            obs: Mutex::new(obs),
            in_flight: Mutex::new(HashMap::new()),
            verifier: CallVerifier,
            bridge_verify_timeout: Duration::from_secs(8),
        }
    }

    /// Real transport access (integration/live-fire suites read real
    /// Asterisk state through the production surface).
    pub fn transport(&self) -> &dyn AriTransport {
        self.transport.as_ref()
    }

    pub fn observability(&self) -> TelephonyObservability {
        self.obs.lock().unwrap().clone()
    }

    /// Provider availability: health probe. Never fabricates.
    pub fn provider_available(&self) -> Result<bool, CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        match self.transport.health() {
            Ok(()) => {
                obs.record(&correlation, "health", "ok", "asterisk reachable");
                Ok(true)
            }
            Err(e) => {
                obs.record_error(
                    &correlation,
                    "health",
                    e.code.as_str(),
                    "asterisk unreachable",
                );
                Err(e)
            }
        }
    }

    /// Idempotency guard: acquire an in-flight entry for
    /// `target:command`. Conflict when already in flight.
    fn acquire_in_flight(&self, target: &str, command: &str) -> Result<String, CallError> {
        let key = format!("{target}:{command}");
        let mut map = self.in_flight.lock().unwrap();
        if map.contains_key(&key) {
            return Err(CallError::new(
                CallErrorCode::Conflict,
                format!("command {command} already in flight for {target}"),
                None,
                Some(target.to_string()),
            ));
        }
        map.insert(
            key.clone(),
            InFlightEntry {
                command: command.to_string(),
            },
        );
        Ok(key)
    }

    fn release_in_flight(&self, key: &str) {
        self.in_flight.lock().unwrap().remove(key);
    }

    /// Capability gate: the policy must allow the command's capability
    /// BEFORE any provider mutation.
    fn check_capability(
        &self,
        command: CallCommand,
        target: &str,
        correlation: &str,
    ) -> Result<CallCapability, CallError> {
        let capability = match command {
            CallCommand::Dial => CallCapability::Dial,
            CallCommand::Answer => CallCapability::Answer,
            CallCommand::Hangup => CallCapability::Hangup,
            CallCommand::Transfer => CallCapability::Transfer,
            CallCommand::SendDtmf => CallCapability::Dtmf,
            CallCommand::Hold => CallCapability::Hold,
            CallCommand::Resume => CallCapability::Hold,
        };
        if !self.policy.allows(capability) {
            return Err(CallError::new(
                CallErrorCode::Policy,
                format!("{} not permitted by call policy", capability.as_str()),
                Some(correlation.to_string()),
                Some(target.to_string()),
            ));
        }
        Ok(capability)
    }

    fn selector_for_session(&self, session: &CallSessionId) -> Result<ChannelSelector, CallError> {
        ChannelSelector::new(session.as_str()).map_err(|e| {
            e.with_correlation("")
                .with_resource(session.as_str().to_string())
        })
    }

    /// Verify an exact-target state transition after a command.
    fn verify_transition(
        &self,
        session: &CallSessionId,
        expected: CallState,
        correlation: &str,
    ) -> Result<(), CallError> {
        let selector = self.selector_for_session(session)?;
        match self.transport.channel_state(&selector) {
            Ok(channel) => {
                let observed = map_channel_state(&channel)?;
                let result = self.verifier.verify(session, expected, session, observed);
                match result {
                    CallVerification::Verified => {
                        self.obs.lock().unwrap().record(
                            correlation,
                            "verify",
                            "ok",
                            &format!("session {} reached {}", session.as_str(), expected.as_str()),
                        );
                        Ok(())
                    }
                    other => {
                        let err = self
                            .verifier
                            .error_for(other, session, expected, Some(correlation.to_string()))
                            .unwrap_err();
                        self.obs.lock().unwrap().record_error(
                            correlation,
                            "verify",
                            "VERIFICATION",
                            &err.to_string(),
                        );
                        Err(err)
                    }
                }
            }
            Err(e) => {
                self.obs.lock().unwrap().record_error(
                    correlation,
                    "verify",
                    e.code.as_str(),
                    "readback failed after command",
                );
                Err(e)
            }
        }
    }

    /// Read the media verification state for a session. Media states
    /// beyond NONE come only from real evidence:
    /// - TransportActive requires the real ARI channel to be Up AND in
    ///   a real bridge (M3 Stasis topology).
    /// - TwoWayAudioVerified is NOT derived here: it requires decoded
    ///   audio proof (whisper canary readback) owned by the
    ///   integration/live-fire suite. Never fabricated from channel
    ///   state alone.
    pub fn media_state(&self, session: &CallSessionId) -> Result<MediaState, CallError> {
        let selector = self.selector_for_session(session)?;
        let channel = self.transport.channel_state(&selector)?;
        // Asterisk 22 ARI does NOT serialize the channel's `bridge`
        // field; real bridge membership (GET /ari/bridges -> channels)
        // is the authoritative surface.
        let in_real_bridge = match self.transport.list_bridges() {
            Ok(bridges) => bridges
                .iter()
                .any(|b| b.channels.iter().any(|c| c == session.as_str())),
            Err(_) => false,
        };
        match channel.state.as_str() {
            "Up" if in_real_bridge => Ok(MediaState::TransportActive),
            _ => Ok(MediaState::None),
        }
    }

    /// List real sessions (real ARI channels).
    pub fn list_sessions(&self) -> Result<Vec<CallSession>, CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        match self.transport.list_channels() {
            Ok(channels) => {
                let mut sessions = Vec::new();
                for channel in channels {
                    match session_from_channel(&channel) {
                        Ok(session) => sessions.push(session),
                        Err(e) => {
                            obs.record_error(
                                &correlation,
                                "list",
                                e.code.as_str(),
                                "channel mapping skipped",
                            );
                        }
                    }
                }
                obs.record(
                    &correlation,
                    "list",
                    "ok",
                    &format!("{} sessions", sessions.len()),
                );
                Ok(sessions)
            }
            Err(e) => {
                obs.record_error(&correlation, "list", e.code.as_str(), "channel list failed");
                Err(e)
            }
        }
    }

    /// Originate a call to a real PJSIP endpoint (capability-gated
    /// Dial; real Asterisk originate).
    pub fn originate(
        &self,
        endpoint: &SipEndpointId,
        context: &str,
        extension: &str,
        caller_id: Option<&str>,
    ) -> Result<CallSession, CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        let target = format!("originate:{}", endpoint.as_str());
        // Capability gate BEFORE provider mutation.
        self.check_capability(CallCommand::Dial, &target, &correlation)?;
        // Idempotency: same endpoint + same command in flight ->
        // Conflict.
        let key = self.acquire_in_flight(&target, "DIAL").map_err(|e| {
            obs.record_error(&correlation, "ORIGINATE", e.code.as_str(), "duplicate");
            e.with_correlation(correlation.clone())
                .with_resource(target.clone())
        })?;
        drop(obs);
        let result = self
            .transport
            .originate(endpoint, context, extension, caller_id);
        self.release_in_flight(&key);
        let mut obs = self.obs.lock().unwrap();
        match result {
            Ok(channel) => {
                let session = session_from_channel(&channel).map_err(|e| {
                    obs.record_error(
                        &correlation,
                        "ORIGINATE",
                        e.code.as_str(),
                        "channel mapping failed",
                    );
                    e.with_correlation(correlation.clone())
                })?;
                obs.record(
                    &correlation,
                    "ORIGINATE",
                    "ok",
                    &format!("session {} channel {}", session.id.as_str(), channel.id),
                );
                Ok(session)
            }
            Err(e) => {
                obs.record_error(
                    &correlation,
                    "ORIGINATE",
                    e.code.as_str(),
                    "originate failed",
                );
                Err(e
                    .with_correlation(correlation.clone())
                    .with_resource(target))
            }
        }
    }

    /// Originate a call directly into a real Stasis application
    /// (capability-gated Dial; the canonical ARI path for
    /// Stasis-controlled call legs, M3).
    pub fn originate_stasis(
        &self,
        endpoint: &SipEndpointId,
        app: &str,
        app_args: &str,
        caller_id: Option<&str>,
    ) -> Result<CallSession, CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        let target = format!("originate-stasis:{}", endpoint.as_str());
        self.check_capability(CallCommand::Dial, &target, &correlation)?;
        let key = self.acquire_in_flight(&target, "DIAL").map_err(|e| {
            obs.record_error(
                &correlation,
                "ORIGINATE_STASIS",
                e.code.as_str(),
                "duplicate",
            );
            e.with_correlation(correlation.clone())
                .with_resource(target.clone())
        })?;
        drop(obs);
        let result = self
            .transport
            .originate_with_app(endpoint, app, app_args, caller_id);
        self.release_in_flight(&key);
        let mut obs = self.obs.lock().unwrap();
        match result {
            Ok(channel) => {
                let session = session_from_channel(&channel).map_err(|e| {
                    obs.record_error(
                        &correlation,
                        "ORIGINATE_STASIS",
                        e.code.as_str(),
                        "channel mapping failed",
                    );
                    e.with_correlation(correlation.clone())
                })?;
                obs.record(
                    &correlation,
                    "ORIGINATE_STASIS",
                    "ok",
                    &format!(
                        "session {} channel {} in stasis app {}",
                        session.id.as_str(),
                        channel.id,
                        app
                    ),
                );
                Ok(session)
            }
            Err(e) => {
                obs.record_error(
                    &correlation,
                    "ORIGINATE_STASIS",
                    e.code.as_str(),
                    "stasis originate failed",
                );
                Err(e
                    .with_correlation(correlation.clone())
                    .with_resource(target))
            }
        }
    }

    /// Create a real ARI mixing bridge (M3 Stasis topology). Returns
    /// the real bridge id. Never fabricates membership.
    pub fn create_mixing_bridge(&self, name: &str) -> Result<String, CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        match self.transport.create_bridge("mixing", name) {
            Ok(bridge) => {
                obs.record(
                    &correlation,
                    "BRIDGE_CREATE",
                    "ok",
                    &format!("bridge {} ({})", bridge.id, name),
                );
                Ok(bridge.id)
            }
            Err(e) => {
                obs.record_error(
                    &correlation,
                    "BRIDGE_CREATE",
                    e.code.as_str(),
                    "bridge create failed",
                );
                Err(e.with_correlation(correlation.clone()))
            }
        }
    }

    /// Add a real Stasis-controlled channel to a real ARI bridge,
    /// then verify the exact session's channel is actually a member
    /// (bridge id observed on the channel's real ARI object).
    pub fn add_to_bridge(&self, session: &CallSessionId, bridge_id: &str) -> Result<(), CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        let selector = match self.selector_for_session(session) {
            Ok(s) => s,
            Err(e) => return Err(e.with_correlation(correlation.clone())),
        };
        match self.transport.add_channel_to_bridge(bridge_id, &selector) {
            Ok(()) => {
                obs.record(
                    &correlation,
                    "BRIDGE_ADD",
                    "ok",
                    &format!("session {} added to bridge {}", session.as_str(), bridge_id),
                );
                drop(obs);
                // Verify membership from the REAL bridge object.
                // Asterisk 22 ARI does NOT serialize the channel's
                // `bridge` field on GET /ari/channels/{id} (verified
                // live: keys = accountcode/caller/connected/... no
                // bridge). The authoritative membership surface is the
                // bridge resource (GET /ari/bridges/{id} -> channels).
                // ARI returns 200/204 for addChannel immediately, but
                // membership propagates asynchronously (ChannelEntered-
                // Bridge fires after the response). Bounded retry: poll
                // the real bridge object until the exact session's
                // channel is listed, or fail closed with Verification.
                let deadline = Instant::now() + self.bridge_verify_timeout;
                let mut last: Option<Vec<String>> = None;
                loop {
                    let readback = self.transport.get_bridge(bridge_id);
                    match readback {
                        Ok(bridge) => {
                            let members = bridge.channels.clone();
                            last = Some(members.clone());
                            if members.iter().any(|c| c == session.as_str()) {
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            let _ = e;
                        }
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(300));
                }
                Err(CallError::new(
                    CallErrorCode::Verification,
                    format!(
                        "channel not a member of bridge {bridge_id} (bridge channels {:?})",
                        last.unwrap_or_default()
                    ),
                    Some(correlation.clone()),
                    Some(session.to_string()),
                ))
            }
            Err(e) => {
                obs.record_error(
                    &correlation,
                    "BRIDGE_ADD",
                    e.code.as_str(),
                    "bridge add failed",
                );
                Err(e
                    .with_correlation(correlation.clone())
                    .with_resource(session.to_string()))
            }
        }
    }

    /// Query a real ARI bridge (real membership; never fabricated).
    pub fn get_bridge(&self, bridge_id: &str) -> Result<AriBridge, CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        match self.transport.get_bridge(bridge_id) {
            Ok(bridge) => {
                obs.record(
                    &correlation,
                    "BRIDGE_GET",
                    "ok",
                    &format!(
                        "bridge {} has {} channels",
                        bridge.id,
                        bridge.channels.len()
                    ),
                );
                Ok(bridge)
            }
            Err(e) => {
                obs.record_error(
                    &correlation,
                    "BRIDGE_GET",
                    e.code.as_str(),
                    "bridge get failed",
                );
                Err(e.with_correlation(correlation.clone()))
            }
        }
    }

    /// Delete a real ARI bridge (real teardown).
    pub fn delete_bridge(&self, bridge_id: &str) -> Result<(), CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        match self.transport.delete_bridge(bridge_id) {
            Ok(()) => {
                obs.record(
                    &correlation,
                    "BRIDGE_DELETE",
                    "ok",
                    &format!("bridge {bridge_id} deleted"),
                );
                Ok(())
            }
            Err(e) => {
                obs.record_error(
                    &correlation,
                    "BRIDGE_DELETE",
                    e.code.as_str(),
                    "bridge delete failed",
                );
                Err(e.with_correlation(correlation.clone()))
            }
        }
    }

    /// Real PJSIP endpoint registration state from Asterisk's own ARI
    /// surface (state "online" when a real contact is registered).
    pub fn endpoint_state(&self, resource: &str) -> Result<AriEndpoint, CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        match self.transport.endpoint_state(resource) {
            Ok(endpoint) => {
                obs.record(
                    &correlation,
                    "ENDPOINT_STATE",
                    "ok",
                    &format!(
                        "endpoint {}/{} state {}",
                        endpoint.technology,
                        endpoint.resource,
                        endpoint.state.as_deref().unwrap_or("unknown")
                    ),
                );
                Ok(endpoint)
            }
            Err(e) => {
                obs.record_error(
                    &correlation,
                    "ENDPOINT_STATE",
                    e.code.as_str(),
                    "endpoint state failed",
                );
                Err(e.with_correlation(correlation.clone()))
            }
        }
    }

    /// Answer a real channel (capability-gated Answer), then verify
    /// the exact target reached ANSWERED/BRIDGED.
    pub fn answer(&self, session: &CallSessionId) -> Result<(), CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        self.check_capability(CallCommand::Answer, session.as_str(), &correlation)?;
        let key = self
            .acquire_in_flight(session.as_str(), "ANSWER")
            .map_err(|e| {
                obs.record_error(&correlation, "ANSWER", e.code.as_str(), "duplicate");
                e.with_correlation(correlation.clone())
                    .with_resource(session.to_string())
            })?;
        drop(obs);
        let selector = match self.selector_for_session(session) {
            Ok(s) => s,
            Err(e) => {
                self.release_in_flight(&key);
                return Err(e.with_correlation(correlation.clone()));
            }
        };
        let result = self.transport.answer(&selector);
        self.release_in_flight(&key);
        let mut obs = self.obs.lock().unwrap();
        match result {
            Ok(()) => {
                obs.record(
                    &correlation,
                    "ANSWER",
                    "ok",
                    &format!("session {} answered", session.as_str()),
                );
                drop(obs);
                self.verify_transition(session, CallState::Answered, &correlation)
            }
            Err(e) => {
                obs.record_error(&correlation, "ANSWER", e.code.as_str(), "answer failed");
                Err(e
                    .with_correlation(correlation.clone())
                    .with_resource(session.to_string()))
            }
        }
    }

    /// Hangup a real channel (capability-gated Hangup), then verify
    /// the exact target is gone (NotFound readback = verified).
    pub fn hangup(&self, session: &CallSessionId) -> Result<(), CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        self.check_capability(CallCommand::Hangup, session.as_str(), &correlation)?;
        let key = self
            .acquire_in_flight(session.as_str(), "HANGUP")
            .map_err(|e| {
                obs.record_error(&correlation, "HANGUP", e.code.as_str(), "duplicate");
                e.with_correlation(correlation.clone())
                    .with_resource(session.to_string())
            })?;
        drop(obs);
        let selector = match self.selector_for_session(session) {
            Ok(s) => s,
            Err(e) => {
                self.release_in_flight(&key);
                return Err(e.with_correlation(correlation.clone()));
            }
        };
        let result = self.transport.hangup(&selector);
        self.release_in_flight(&key);
        let mut obs = self.obs.lock().unwrap();
        match result {
            Ok(()) => {
                obs.record(
                    &correlation,
                    "HANGUP",
                    "ok",
                    &format!("session {} hung up", session.as_str()),
                );
                // Verify: the channel must be gone (NotFound readback).
                // ARI DELETE returns 200 immediately, but the channel
                // object propagates destruction asynchronously
                // (ChannelDestroyed fires after the response). Bounded
                // retry, same pattern as bridge membership.
                drop(obs);
                let deadline = Instant::now() + Duration::from_secs(8);
                let mut last_err: Option<CallError>;
                loop {
                    let selector = match self.selector_for_session(session) {
                        Ok(s) => s,
                        Err(e) => {
                            last_err = Some(e);
                            break;
                        }
                    };
                    match self.transport.channel_state(&selector) {
                        Err(e) if e.code == CallErrorCode::NotFound => {
                            self.obs.lock().unwrap().record(
                                &correlation,
                                "verify",
                                "ok",
                                "channel removed after hangup",
                            );
                            return Ok(());
                        }
                        Ok(_) => {
                            last_err = None;
                        }
                        Err(e) => {
                            last_err = Some(e);
                        }
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(300));
                }
                let err = CallError::new(
                    CallErrorCode::Verification,
                    format!(
                        "channel still present after hangup (last readback {:?})",
                        last_err.as_ref().map(|e| e.code.as_str())
                    ),
                    Some(correlation.clone()),
                    Some(session.to_string()),
                );
                self.obs.lock().unwrap().record_error(
                    &correlation,
                    "verify",
                    "VERIFICATION",
                    "channel still present after hangup",
                );
                Err(err)
            }
            Err(e) => {
                obs.record_error(&correlation, "HANGUP", e.code.as_str(), "hangup failed");
                Err(e
                    .with_correlation(correlation.clone())
                    .with_resource(session.to_string()))
            }
        }
    }

    /// Send DTMF digits (capability-gated Dtmf; SUBMITTED semantics -
    /// reception verification is the M3 endpoint proof).
    pub fn send_dtmf(&self, session: &CallSessionId, digits: &str) -> Result<(), CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        self.check_capability(CallCommand::SendDtmf, session.as_str(), &correlation)?;
        let key = self
            .acquire_in_flight(session.as_str(), "DTMF")
            .map_err(|e| {
                obs.record_error(&correlation, "DTMF", e.code.as_str(), "duplicate");
                e.with_correlation(correlation.clone())
                    .with_resource(session.to_string())
            })?;
        drop(obs);
        let selector = match self.selector_for_session(session) {
            Ok(s) => s,
            Err(e) => {
                self.release_in_flight(&key);
                return Err(e.with_correlation(correlation.clone()));
            }
        };
        let result = self.transport.send_dtmf(&selector, digits);
        self.release_in_flight(&key);
        let mut obs = self.obs.lock().unwrap();
        match result {
            Ok(()) => {
                obs.record(
                    &correlation,
                    "DTMF",
                    "ok",
                    &format!("dtmf submitted to session {}", session.as_str()),
                );
                Ok(())
            }
            Err(e) => {
                obs.record_error(&correlation, "DTMF", e.code.as_str(), "dtmf failed");
                Err(e
                    .with_correlation(correlation.clone())
                    .with_resource(session.to_string()))
            }
        }
    }

    /// Hold (capability-gated Hold; real MOH on the channel).
    pub fn hold(&self, session: &CallSessionId) -> Result<(), CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        self.check_capability(CallCommand::Hold, session.as_str(), &correlation)?;
        let key = self
            .acquire_in_flight(session.as_str(), "HOLD")
            .map_err(|e| {
                obs.record_error(&correlation, "HOLD", e.code.as_str(), "duplicate");
                e.with_correlation(correlation.clone())
                    .with_resource(session.to_string())
            })?;
        drop(obs);
        let selector = match self.selector_for_session(session) {
            Ok(s) => s,
            Err(e) => {
                self.release_in_flight(&key);
                return Err(e.with_correlation(correlation.clone()));
            }
        };
        let result = self.transport.start_moh(&selector);
        self.release_in_flight(&key);
        let mut obs = self.obs.lock().unwrap();
        match result {
            Ok(()) => {
                obs.record(
                    &correlation,
                    "HOLD",
                    "ok",
                    &format!("hold started on session {}", session.as_str()),
                );
                Ok(())
            }
            Err(e) => {
                obs.record_error(&correlation, "HOLD", e.code.as_str(), "hold failed");
                Err(e
                    .with_correlation(correlation.clone())
                    .with_resource(session.to_string()))
            }
        }
    }

    /// Resume from hold (capability-gated Hold; stop MOH).
    pub fn resume(&self, session: &CallSessionId) -> Result<(), CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        self.check_capability(CallCommand::Resume, session.as_str(), &correlation)?;
        let key = self
            .acquire_in_flight(session.as_str(), "RESUME")
            .map_err(|e| {
                obs.record_error(&correlation, "RESUME", e.code.as_str(), "duplicate");
                e.with_correlation(correlation.clone())
                    .with_resource(session.to_string())
            })?;
        drop(obs);
        let selector = match self.selector_for_session(session) {
            Ok(s) => s,
            Err(e) => {
                self.release_in_flight(&key);
                return Err(e.with_correlation(correlation.clone()));
            }
        };
        let result = self.transport.stop_moh(&selector);
        self.release_in_flight(&key);
        let mut obs = self.obs.lock().unwrap();
        match result {
            Ok(()) => {
                obs.record(
                    &correlation,
                    "RESUME",
                    "ok",
                    &format!("hold stopped on session {}", session.as_str()),
                );
                Ok(())
            }
            Err(e) => {
                obs.record_error(&correlation, "RESUME", e.code.as_str(), "resume failed");
                Err(e
                    .with_correlation(correlation.clone())
                    .with_resource(session.to_string()))
            }
        }
    }
}

impl TelephonyProvider for AsteriskAdapter {
    fn list_sessions(&self) -> Result<Vec<CallSessionId>, CallError> {
        let sessions = self.list_sessions()?;
        Ok(sessions.into_iter().map(|s| s.id).collect())
    }

    fn capabilities(&self, session: &CallSessionId) -> Result<Vec<CallCapability>, CallError> {
        // A session is governed by the policy; capabilities are the
        // policy's allowed set (authorization remains EP-008's
        // authority - directive R).
        let _ = session;
        Ok(self.policy.allowed_capabilities.clone())
    }

    fn session_state(&self, session: &CallSessionId) -> Result<CallState, CallError> {
        let mut obs = self.obs.lock().unwrap();
        let correlation = obs.correlation();
        let selector = self.selector_for_session(session)?;
        match self.transport.channel_state(&selector) {
            Ok(channel) => {
                let base = map_channel_state(&channel)?;
                // Asterisk 22 ARI does NOT serialize the channel's
                // `bridge` field, so BRIDGED can only be derived from
                // real bridge membership (GET /ari/bridges -> channels).
                // An Up channel that is a member of a real bridge is
                // Bridged; otherwise Answered.
                let state = if base == CallState::Answered {
                    match self.transport.list_bridges() {
                        Ok(bridges) => {
                            if bridges
                                .iter()
                                .any(|b| b.channels.iter().any(|c| c == session.as_str()))
                            {
                                CallState::Bridged
                            } else {
                                base
                            }
                        }
                        Err(_) => base,
                    }
                } else {
                    base
                };
                obs.record(
                    &correlation,
                    "STATUS",
                    "ok",
                    &format!("session {} state {}", session.as_str(), state.as_str()),
                );
                Ok(state)
            }
            Err(e) => {
                obs.record_error(
                    &correlation,
                    "STATUS",
                    e.code.as_str(),
                    "state readback failed",
                );
                Err(e
                    .with_correlation(correlation.clone())
                    .with_resource(session.to_string()))
            }
        }
    }
}

impl AsteriskProvider for AsteriskAdapter {
    fn originate(
        &self,
        endpoint: &SipEndpointId,
        context: &str,
        extension: &str,
    ) -> Result<CallSessionId, CallError> {
        let session = self.originate(endpoint, context, extension, None)?;
        Ok(session.id)
    }

    fn answer(&self, session: &CallSessionId) -> Result<(), CallError> {
        self.answer(session)
    }

    fn hangup(&self, session: &CallSessionId) -> Result<(), CallError> {
        self.hangup(session)
    }

    fn bridge(&self, session: &CallSessionId, other: &CallSessionId) -> Result<(), CallError> {
        // M3: bind the production bridge surface to a real ARI mixing
        // bridge. Both sessions must be real Stasis-controlled
        // channels; membership is verified from the real bridge and
        // channel objects (never fabricated).
        let bridge_id = self.create_mixing_bridge("nexus-bridge")?;
        let add_a = self.add_to_bridge(session, &bridge_id);
        let add_b = self.add_to_bridge(other, &bridge_id);
        if add_a.is_err() || add_b.is_err() {
            let _ = self.delete_bridge(&bridge_id);
        }
        add_a?;
        add_b?;
        let bridge = self.get_bridge(&bridge_id)?;
        let has_a = bridge.channels.iter().any(|c| c == session.as_str());
        let has_b = bridge.channels.iter().any(|c| c == other.as_str());
        if !(has_a && has_b) {
            return Err(CallError::new(
                CallErrorCode::Verification,
                "bridge membership missing after add",
                None,
                Some(session.to_string()),
            ));
        }
        Ok(())
    }

    fn transfer(&self, session: &CallSessionId, _target: &str) -> Result<(), CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "transfer not yet bound to a real ARI redirect",
        ))
    }

    fn send_dtmf(&self, session: &CallSessionId, digits: &str) -> Result<(), CallError> {
        self.send_dtmf(session, digits)
    }

    fn hold(&self, session: &CallSessionId) -> Result<(), CallError> {
        self.hold(session)
    }

    fn channel_state(&self, session: &CallSessionId) -> Result<CallState, CallError> {
        self.session_state(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{AriCallerId, AriDialplan};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Controlled transport for deterministic unit tests: injectable
    /// responses, call counting to prove capability gating happens
    /// BEFORE any provider mutation. Interior mutability because the
    /// port takes `&self`.
    #[derive(Clone)]
    struct ControlledTransport {
        health_ok: Arc<AtomicBool>,
        channels: Arc<Mutex<Vec<AriChannel>>>,
        calls: Arc<Mutex<Vec<String>>>,
        noop_bridge_add: Arc<AtomicBool>,
    }

    impl Default for ControlledTransport {
        fn default() -> Self {
            Self {
                health_ok: Arc::new(AtomicBool::new(true)),
                channels: Arc::new(Mutex::new(Vec::new())),
                calls: Arc::new(Mutex::new(Vec::new())),
                noop_bridge_add: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    impl ControlledTransport {
        fn record(&self, op: &str) {
            self.calls.lock().unwrap().push(op.to_string());
        }
    }

    impl AriTransport for ControlledTransport {
        fn health(&self) -> Result<(), CallError> {
            self.record("health");
            if self.health_ok.load(Ordering::SeqCst) {
                Ok(())
            } else {
                Err(CallError::unavailable("asterisk down"))
            }
        }

        fn list_channels(&self) -> Result<Vec<AriChannel>, CallError> {
            self.record("list");
            Ok(self.channels.lock().unwrap().clone())
        }

        fn channel_state(&self, channel: &ChannelSelector) -> Result<AriChannel, CallError> {
            self.record(&format!("state:{}", channel.as_str()));
            self.channels
                .lock()
                .unwrap()
                .iter()
                .find(|c| c.id == channel.as_str())
                .cloned()
                .ok_or_else(|| CallError::not_found("channel not found"))
        }

        fn originate(
            &self,
            endpoint: &SipEndpointId,
            _context: &str,
            _extension: &str,
            _caller_id: Option<&str>,
        ) -> Result<AriChannel, CallError> {
            self.record(&format!("originate:{}", endpoint.as_str()));
            let channel = AriChannel {
                id: format!("PJSIP/{}-00000001", endpoint.as_str()),
                name: format!("PJSIP/{}", endpoint.as_str()),
                state: "Ring".to_string(),
                caller: Some(AriCallerId {
                    name: "Nexus".to_string(),
                    number: "100".to_string(),
                }),
                connected: None,
                dialplan: None,
                bridge: None,
                creationtime: None,
                language: None,
            };
            self.channels.lock().unwrap().push(channel.clone());
            Ok(channel)
        }

        fn originate_with_app(
            &self,
            endpoint: &SipEndpointId,
            app: &str,
            app_args: &str,
            _caller_id: Option<&str>,
        ) -> Result<AriChannel, CallError> {
            self.record(&format!(
                "originate-app:{}:{}:{}",
                endpoint.as_str(),
                app,
                app_args
            ));
            let channel = AriChannel {
                id: format!("PJSIP/{}-00000001", endpoint.as_str()),
                name: format!("PJSIP/{}", endpoint.as_str()),
                state: "Ring".to_string(),
                caller: Some(AriCallerId {
                    name: "Nexus".to_string(),
                    number: "100".to_string(),
                }),
                connected: None,
                dialplan: Some(AriDialplan {
                    context: "stasis".to_string(),
                    exten: "s".to_string(),
                    priority: 1,
                }),
                bridge: None,
                creationtime: None,
                language: None,
            };
            self.channels.lock().unwrap().push(channel.clone());
            Ok(channel)
        }

        fn create_bridge(&self, bridge_type: &str, name: &str) -> Result<AriBridge, CallError> {
            self.record(&format!("bridge-create:{bridge_type}:{name}"));
            Ok(AriBridge {
                id: format!("bridge-{name}"),
                name: Some(name.to_string()),
                technology: Some("native_rtp".to_string()),
                bridge_type: Some(bridge_type.to_string()),
                bridge_class: Some("stasis".to_string()),
                creator: Some("Stasis".to_string()),
                channels: Vec::new(),
                creationtime: None,
            })
        }

        fn get_bridge(&self, bridge_id: &str) -> Result<AriBridge, CallError> {
            self.record(&format!("bridge-get:{bridge_id}"));
            let channels = self
                .channels
                .lock()
                .unwrap()
                .iter()
                .filter(|c| c.bridge.as_deref() == Some(bridge_id))
                .map(|c| c.id.clone())
                .collect::<Vec<_>>();
            Ok(AriBridge {
                id: bridge_id.to_string(),
                name: Some(bridge_id.to_string()),
                technology: Some("native_rtp".to_string()),
                bridge_type: Some("mixing".to_string()),
                bridge_class: Some("stasis".to_string()),
                creator: Some("Stasis".to_string()),
                channels,
                creationtime: None,
            })
        }

        fn delete_bridge(&self, bridge_id: &str) -> Result<(), CallError> {
            self.record(&format!("bridge-delete:{bridge_id}"));
            Ok(())
        }

        fn add_channel_to_bridge(
            &self,
            bridge_id: &str,
            channel: &ChannelSelector,
        ) -> Result<(), CallError> {
            self.record(&format!("bridge-add:{bridge_id}:{}", channel.as_str()));
            if self.noop_bridge_add.load(Ordering::SeqCst) {
                // Simulate a transport that returns Ok but does not
                // actually move the channel (Asterisk refused / the
                // channel is not a member).
                return Ok(());
            }
            let mut channels = self.channels.lock().unwrap();
            if let Some(c) = channels.iter_mut().find(|c| c.id == channel.as_str()) {
                c.bridge = Some(bridge_id.to_string());
                Ok(())
            } else {
                Err(CallError::not_found("channel not found"))
            }
        }

        fn endpoint_state(&self, resource: &str) -> Result<AriEndpoint, CallError> {
            self.record(&format!("endpoint:{resource}"));
            Ok(AriEndpoint {
                technology: "PJSIP".to_string(),
                resource: resource.to_string(),
                state: Some("online".to_string()),
                channel_ids: Vec::new(),
            })
        }

        fn list_bridges(&self) -> Result<Vec<AriBridge>, CallError> {
            self.record("bridges");
            let channels = self.channels.lock().unwrap();
            let mut by_bridge: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();
            for c in channels.iter() {
                if let Some(bid) = c.bridge.clone() {
                    by_bridge.entry(bid.clone()).or_default().push(c.id.clone());
                }
            }
            let bridges = by_bridge
                .into_iter()
                .map(|(id, members)| AriBridge {
                    id: id.clone(),
                    name: Some(id.clone()),
                    technology: Some("native_rtp".to_string()),
                    bridge_type: Some("mixing".to_string()),
                    bridge_class: Some("stasis".to_string()),
                    creator: Some("Stasis".to_string()),
                    channels: members,
                    creationtime: None,
                })
                .collect();
            Ok(bridges)
        }

        fn answer(&self, channel: &ChannelSelector) -> Result<(), CallError> {
            self.record(&format!("answer:{}", channel.as_str()));
            let mut channels = self.channels.lock().unwrap();
            if let Some(c) = channels.iter_mut().find(|c| c.id == channel.as_str()) {
                c.state = "Up".to_string();
                Ok(())
            } else {
                Err(CallError::not_found("channel not found"))
            }
        }

        fn hangup(&self, channel: &ChannelSelector) -> Result<(), CallError> {
            self.record(&format!("hangup:{}", channel.as_str()));
            let mut channels = self.channels.lock().unwrap();
            channels
                .iter()
                .position(|c| c.id == channel.as_str())
                .map(|i| channels.remove(i))
                .map(|_| ())
                .ok_or_else(|| CallError::not_found("channel not found"))
        }

        fn send_dtmf(&self, channel: &ChannelSelector, digits: &str) -> Result<(), CallError> {
            self.record(&format!("dtmf:{}:{}", channel.as_str(), digits));
            Ok(())
        }

        fn start_moh(&self, channel: &ChannelSelector) -> Result<(), CallError> {
            self.record(&format!("moh:{}", channel.as_str()));
            Ok(())
        }

        fn stop_moh(&self, channel: &ChannelSelector) -> Result<(), CallError> {
            self.record(&format!("moh-stop:{}", channel.as_str()));
            Ok(())
        }
    }

    fn policy_with(caps: &[CallCapability]) -> CallPolicy {
        CallPolicy {
            allowed_capabilities: caps.to_vec(),
            max_duration_seconds: 300,
            cost_cap: 1.0,
            disclosure: nexus_telephony::DisclosurePolicy::new(false, true, "US", 0).unwrap(),
        }
    }

    fn channel(id: &str, state: &str, bridge: Option<&str>) -> AriChannel {
        AriChannel {
            id: id.to_string(),
            name: id.to_string(),
            state: state.to_string(),
            caller: None,
            connected: None,
            dialplan: None,
            bridge: bridge.map(|s| s.to_string()),
            creationtime: None,
            language: None,
        }
    }

    #[test]
    fn ep025_unit_state_mapping_documented() {
        // DOCUMENTED ARI states map canonically; unrecognized fails
        // closed.
        assert_eq!(
            map_channel_state(&channel("a", "Up", None)).unwrap(),
            CallState::Answered
        );
        assert_eq!(
            map_channel_state(&channel("a", "Up", Some("b1"))).unwrap(),
            CallState::Bridged
        );
        assert_eq!(
            map_channel_state(&channel("a", "Ring", None)).unwrap(),
            CallState::Ringing
        );
        assert_eq!(
            map_channel_state(&channel("a", "Dialing", None)).unwrap(),
            CallState::InviteSent
        );
        assert_eq!(
            map_channel_state(&channel("a", "Busy", None)).unwrap(),
            CallState::Busy
        );
        assert_eq!(
            map_channel_state(&channel("a", "Down", None)).unwrap(),
            CallState::Requested
        );
        // Unrecognized state -> External, never fabricated.
        assert_eq!(
            map_channel_state(&channel("a", "MAGIC", None))
                .unwrap_err()
                .code,
            CallErrorCode::External
        );
    }

    #[test]
    fn ep025_unit_session_identity_from_channel() {
        let session = session_from_channel(&channel("PJSIP/a-00000001", "Up", None)).unwrap();
        assert_eq!(session.id.as_str(), "PJSIP/a-00000001");
        assert_eq!(session.state, CallState::Answered);
        assert_eq!(session.legs.len(), 1);
    }

    #[test]
    fn ep025_unit_capability_gate_before_provider_mutation() {
        let transport = ControlledTransport::default();
        let calls = transport.calls.clone();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Answer]));
        let endpoint = SipEndpointId::new("endpoint-a").unwrap();
        // Dial NOT in policy -> Policy error, and the transport is
        // NEVER called (proven by zero calls).
        let err = adapter
            .originate(&endpoint, "internal", "100", None)
            .unwrap_err();
        assert_eq!(err.code, CallErrorCode::Policy);
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn ep025_unit_originate_and_answer_verified() {
        let transport = ControlledTransport::default();
        let adapter = AsteriskAdapter::new(
            Box::new(transport),
            policy_with(&[
                CallCapability::Dial,
                CallCapability::Answer,
                CallCapability::Hangup,
            ]),
        );
        let endpoint = SipEndpointId::new("endpoint-a").unwrap();
        let session = adapter
            .originate(&endpoint, "internal", "100", None)
            .unwrap();
        assert!(session.id.as_str().starts_with("PJSIP/endpoint-a"));
        // Answer and verify: the controlled transport flips state to
        // Up, so the exact target reaches ANSWERED.
        adapter.answer(&session.id).unwrap();
        assert_eq!(
            adapter.session_state(&session.id).unwrap(),
            CallState::Answered
        );
    }

    #[test]
    fn ep025_unit_answer_verification_mismatch_fails() {
        // A transport that does NOT change state: answer accepted but
        // readback stays Ring -> Verification (SUBMITTED != VERIFIED).
        struct StubbornTransport;
        impl AriTransport for StubbornTransport {
            fn health(&self) -> Result<(), CallError> {
                Ok(())
            }
            fn list_channels(&self) -> Result<Vec<AriChannel>, CallError> {
                Ok(vec![channel("ch-1", "Ring", None)])
            }
            fn channel_state(&self, c: &ChannelSelector) -> Result<AriChannel, CallError> {
                if c.as_str() == "ch-1" {
                    Ok(channel("ch-1", "Ring", None))
                } else {
                    Err(CallError::not_found("channel not found"))
                }
            }
            fn originate(
                &self,
                _e: &SipEndpointId,
                _c: &str,
                _x: &str,
                _cid: Option<&str>,
            ) -> Result<AriChannel, CallError> {
                Ok(channel("ch-1", "Ring", None))
            }
            fn answer(&self, _c: &ChannelSelector) -> Result<(), CallError> {
                Ok(()) // accepted but state never changes
            }
            fn hangup(&self, _c: &ChannelSelector) -> Result<(), CallError> {
                Ok(())
            }
        }
        let adapter = AsteriskAdapter::new(
            Box::new(StubbornTransport),
            policy_with(&[CallCapability::Dial, CallCapability::Answer]),
        );
        let endpoint = SipEndpointId::new("endpoint-a").unwrap();
        let session = adapter
            .originate(&endpoint, "internal", "100", None)
            .unwrap();
        let err = adapter.answer(&session.id).unwrap_err();
        assert_eq!(err.code, CallErrorCode::Verification);
    }

    #[test]
    fn ep025_unit_hangup_verified_by_channel_gone() {
        let transport = ControlledTransport {
            channels: Arc::new(Mutex::new(vec![channel("PJSIP/a-00000001", "Up", None)])),
            ..Default::default()
        };
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Hangup]));
        let session = CallSessionId::new("PJSIP/a-00000001").unwrap();
        adapter.hangup(&session).unwrap();
        // Channel gone -> NotFound readback.
        assert_eq!(
            adapter.session_state(&session).unwrap_err().code,
            CallErrorCode::NotFound
        );
    }

    #[test]
    fn ep025_unit_unknown_session_not_found() {
        let transport = ControlledTransport::default();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Status]));
        let session = CallSessionId::new("PJSIP/nope-00000001").unwrap();
        let err = adapter.session_state(&session).unwrap_err();
        assert_eq!(err.code, CallErrorCode::NotFound);
    }

    #[test]
    fn ep025_unit_idempotency_duplicate_conflict() {
        // In-flight same endpoint + same command -> Conflict; after
        // completion -> retry allowed.
        let transport = ControlledTransport::default();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Dial]));
        let endpoint = SipEndpointId::new("endpoint-a").unwrap();
        let target = format!("originate:{}", endpoint.as_str());
        let key = adapter.acquire_in_flight(&target, "DIAL").unwrap();
        let err = adapter.acquire_in_flight(&target, "DIAL").unwrap_err();
        assert_eq!(err.code, CallErrorCode::Conflict);
        adapter.release_in_flight(&key);
        // After release, same key is acquirable again (retry not
        // Conflict).
        adapter.acquire_in_flight(&target, "DIAL").unwrap();
    }

    #[test]
    fn ep025_unit_exact_target_never_verified_by_other() {
        let transport = ControlledTransport {
            channels: Arc::new(Mutex::new(vec![
                channel("PJSIP/a-00000001", "Up", None),
                channel("PJSIP/b-00000001", "Ring", None),
            ])),
            ..Default::default()
        };
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Answer]));
        // A command on A cannot be verified by B's state.
        let a = CallSessionId::new("PJSIP/a-00000001").unwrap();
        let b = CallSessionId::new("PJSIP/b-00000001").unwrap();
        let result = adapter
            .verifier
            .verify(&a, CallState::Answered, &b, CallState::Answered);
        assert_eq!(result, CallVerification::UnrelatedChange);
    }

    #[test]
    fn ep025_unit_availability_truth_table() {
        let transport = ControlledTransport::default();
        let health = transport.health_ok.clone();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Status]));
        assert!(adapter.provider_available().unwrap());
        // Health flips to down -> Unavailable (never benign).
        health.store(false, Ordering::SeqCst);
        let err = adapter.provider_available().unwrap_err();
        assert_eq!(err.code, CallErrorCode::Unavailable);
    }

    #[test]
    fn ep025_unit_media_state_never_fabricated() {
        let transport = ControlledTransport::default();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Status]));
        let session = CallSessionId::new("PJSIP/a-00000001").unwrap();
        // Unknown session: NotFound propagates (never fabricates NONE
        // for a session that does not exist).
        let err = adapter.media_state(&session).unwrap_err();
        assert_eq!(err.code, CallErrorCode::NotFound);
        // Real channel Up WITHOUT a bridge: NONE (signaling only, no
        // media claim from channel state alone).
        {
            let transport = ControlledTransport::default();
            transport.channels.lock().unwrap().push(AriChannel {
                id: "PJSIP/a-00000001".into(),
                name: "PJSIP/a-00000001".into(),
                state: "Up".into(),
                caller: None,
                connected: None,
                dialplan: None,
                bridge: None,
                creationtime: None,
                language: None,
            });
            let adapter =
                AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Status]));
            assert_eq!(adapter.media_state(&session).unwrap(), MediaState::None);
        }
        // Real channel Up IN a real bridge: TRANSPORT_ACTIVE only
        // (two-way audio verification still requires decoded proof).
        {
            let transport = ControlledTransport::default();
            transport.channels.lock().unwrap().push(AriChannel {
                id: "PJSIP/a-00000001".into(),
                name: "PJSIP/a-00000001".into(),
                state: "Up".into(),
                caller: None,
                connected: None,
                dialplan: None,
                bridge: Some("bridge-1".into()),
                creationtime: None,
                language: None,
            });
            let adapter =
                AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Status]));
            assert_eq!(
                adapter.media_state(&session).unwrap(),
                MediaState::TransportActive
            );
        }
    }

    #[test]
    fn ep025_unit_correlation_preserved_on_error() {
        let transport = ControlledTransport::default();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Answer]));
        let session = CallSessionId::new("PJSIP/nope-00000001").unwrap();
        let err = adapter.answer(&session).unwrap_err();
        // Correlation attached to the error (Policy gate path).
        assert!(err.correlation.is_some());
        assert!(err.resource.is_some());
    }

    #[test]
    fn ep025_unit_observability_records_operations() {
        let transport = ControlledTransport::default();
        let adapter = AsteriskAdapter::new(
            Box::new(transport),
            policy_with(&[
                CallCapability::Dial,
                CallCapability::Answer,
                CallCapability::Status,
            ]),
        );
        let endpoint = SipEndpointId::new("endpoint-a").unwrap();
        adapter
            .originate(&endpoint, "internal", "100", None)
            .unwrap();
        adapter.provider_available().unwrap();
        let counters = adapter.observability().counters();
        assert!(counters.contains_key("ORIGINATE:ok"));
        assert!(counters.contains_key("health:ok"));
    }

    #[test]
    fn ep025_unit_dtmf_submitted_never_verified() {
        let transport = ControlledTransport {
            channels: Arc::new(Mutex::new(vec![channel("PJSIP/a-00000001", "Up", None)])),
            calls: Arc::new(Mutex::new(Vec::new())),
            health_ok: Arc::new(AtomicBool::new(false)),
            noop_bridge_add: Arc::new(AtomicBool::new(false)),
        };
        let adapter = AsteriskAdapter::new(
            Box::new(transport.clone()),
            policy_with(&[CallCapability::Dtmf]),
        );
        let session = CallSessionId::new("PJSIP/a-00000001").unwrap();
        // SUBMITTED semantics: DTMF accepted is not reception proof
        // (reception verified at M3 with a real endpoint).
        adapter.send_dtmf(&session, "123").unwrap();
        assert!(transport
            .calls
            .lock()
            .unwrap()
            .contains(&"dtmf:PJSIP/a-00000001:123".to_string()));
    }

    #[test]
    fn ep025_unit_hold_resume_governed() {
        let transport = ControlledTransport {
            channels: Arc::new(Mutex::new(vec![channel("PJSIP/a-00000001", "Up", None)])),
            ..Default::default()
        };
        let calls = transport.calls.clone();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Hold]));
        let session = CallSessionId::new("PJSIP/a-00000001").unwrap();
        adapter.hold(&session).unwrap();
        adapter.resume(&session).unwrap();
        let calls = calls.lock().unwrap();
        assert!(calls.contains(&"moh:PJSIP/a-00000001".to_string()));
        assert!(calls.contains(&"moh-stop:PJSIP/a-00000001".to_string()));
    }

    #[test]
    fn ep025_unit_call_leg_state_reflects_channel() {
        let transport = ControlledTransport {
            channels: Arc::new(Mutex::new(vec![channel("PJSIP/a-00000001", "Ring", None)])),
            ..Default::default()
        };
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Status]));
        let sessions = adapter.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].legs[0].state, CallState::Ringing);
        assert_eq!(sessions[0].state, CallState::Ringing);
    }

    #[test]
    fn ep025_unit_originate_stasis_capability_gated() {
        // Dial capability missing -> Policy BEFORE any transport call.
        let transport = ControlledTransport::default();
        let calls = transport.calls.clone();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Status]));
        let endpoint = SipEndpointId::new("endpoint-a").unwrap();
        let err = adapter
            .originate_stasis(&endpoint, "nexus-telephony", "leg=a", None)
            .unwrap_err();
        assert_eq!(err.code, CallErrorCode::Policy);
        assert!(
            calls.lock().unwrap().is_empty(),
            "no transport call on denial"
        );
    }

    #[test]
    fn ep025_unit_originate_stasis_real_channel() {
        let transport = ControlledTransport::default();
        let calls = transport.calls.clone();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Dial]));
        let endpoint = SipEndpointId::new("endpoint-a").unwrap();
        let session = adapter
            .originate_stasis(&endpoint, "nexus-telephony", "leg=a", None)
            .unwrap();
        assert_eq!(session.id.as_str(), "PJSIP/endpoint-a-00000001");
        assert!(calls
            .lock()
            .unwrap()
            .contains(&"originate-app:endpoint-a:nexus-telephony:leg=a".to_string()));
    }

    #[test]
    fn ep025_unit_bridge_orchestration_verified_membership() {
        // Full bridge journey: create mixing bridge, add two real
        // channels, verify membership, delete.
        let transport = ControlledTransport::default();
        let adapter = AsteriskAdapter::new(
            Box::new(transport.clone()),
            policy_with(&[
                CallCapability::Dial,
                CallCapability::Answer,
                CallCapability::Status,
            ]),
        );
        let ep_a = SipEndpointId::new("endpoint-a").unwrap();
        let ep_b = SipEndpointId::new("endpoint-b").unwrap();
        let a = adapter
            .originate_stasis(&ep_a, "nexus-telephony", "leg=a", None)
            .unwrap();
        let b = adapter
            .originate_stasis(&ep_b, "nexus-telephony", "leg=b", None)
            .unwrap();
        adapter.answer(&a.id).unwrap();
        adapter.answer(&b.id).unwrap();
        let bridge_id = adapter.create_mixing_bridge("nexus-m3-unit").unwrap();
        adapter.add_to_bridge(&a.id, &bridge_id).unwrap();
        adapter.add_to_bridge(&b.id, &bridge_id).unwrap();
        let bridge = adapter.get_bridge(&bridge_id).unwrap();
        assert_eq!(bridge.channels.len(), 2);
        assert!(bridge.channels.contains(&a.id.as_str().to_string()));
        assert!(bridge.channels.contains(&b.id.as_str().to_string()));
        adapter.delete_bridge(&bridge_id).unwrap();
        // media_state now reflects real bridge membership.
        assert_eq!(
            adapter.media_state(&a.id).unwrap(),
            MediaState::TransportActive
        );
    }

    #[test]
    fn ep025_unit_bridge_membership_never_fabricated() {
        // Verification reads the REAL channel object: if the channel
        // is not actually a member of the requested bridge (transport
        // did not move it), the add fails closed with Verification.
        let transport = ControlledTransport {
            noop_bridge_add: Arc::new(AtomicBool::new(true)),
            ..Default::default()
        };
        let adapter = AsteriskAdapter::new(
            Box::new(transport.clone()),
            policy_with(&[
                CallCapability::Dial,
                CallCapability::Answer,
                CallCapability::Status,
            ]),
        )
        .with_bridge_verify_timeout(Duration::from_millis(200));
        let ep = SipEndpointId::new("endpoint-a").unwrap();
        let session = adapter
            .originate_stasis(&ep, "nexus-telephony", "leg=a", None)
            .unwrap();
        adapter.answer(&session.id).unwrap();
        let bridge_id = adapter.create_mixing_bridge("nexus-m3x").unwrap();
        // add_channel_to_bridge returns Ok but the channel object is
        // not updated -> membership verification must fail closed.
        let err = adapter.add_to_bridge(&session.id, &bridge_id).unwrap_err();
        assert_eq!(err.code, CallErrorCode::Verification);
    }

    #[test]
    fn ep025_unit_provider_bridge_binds_real_ari_bridge() {
        // The AsteriskProvider::bridge() surface (previously
        // unavailable) now orchestrates a real mixing bridge with
        // verified membership.
        let transport = ControlledTransport::default();
        let adapter = AsteriskAdapter::new(
            Box::new(transport.clone()),
            policy_with(&[
                CallCapability::Dial,
                CallCapability::Answer,
                CallCapability::Status,
            ]),
        );
        let ep_a = SipEndpointId::new("endpoint-a").unwrap();
        let ep_b = SipEndpointId::new("endpoint-b").unwrap();
        let a = adapter
            .originate_stasis(&ep_a, "nexus-telephony", "leg=a", None)
            .unwrap();
        let b = adapter
            .originate_stasis(&ep_b, "nexus-telephony", "leg=b", None)
            .unwrap();
        adapter.answer(&a.id).unwrap();
        adapter.answer(&b.id).unwrap();
        let provider: &dyn AsteriskProvider = &adapter;
        provider.bridge(&a.id, &b.id).unwrap();
        assert_eq!(
            adapter.media_state(&a.id).unwrap(),
            MediaState::TransportActive
        );
    }

    #[test]
    fn ep025_unit_endpoint_state_online() {
        let transport = ControlledTransport::default();
        let adapter =
            AsteriskAdapter::new(Box::new(transport), policy_with(&[CallCapability::Status]));
        let endpoint = adapter.endpoint_state("endpoint-a").unwrap();
        assert_eq!(endpoint.technology, "PJSIP");
        assert_eq!(endpoint.resource, "endpoint-a");
        assert_eq!(endpoint.state.as_deref(), Some("online"));
    }
}
