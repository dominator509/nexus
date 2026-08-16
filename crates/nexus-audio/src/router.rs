//! EP-022 endpoint router (SPEC-012 behaviors 6, 9; acceptance
//! obligation 4).
//!
//! Input and output endpoints are selected by person, room, privacy,
//! and availability. Selection is deterministic: available endpoints
//! preferred, then person-assigned, then room-assigned, then
//! privacy-compatible. A sensitive interaction in a shared room never
//! selects an audible shared-room output (LF-028 precedent).

use nexus_domain::PersonId;

use crate::endpoint::{AudioEndpoint, AudioEndpointId, AudioRoomId};
use crate::error::{AudioError, AudioErrorCode};
use crate::vocabulary::{EndpointRole, HardwareClass};

/// Routing policy flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouterPolicy {
    /// Prefer endpoints bound to the requesting person.
    pub prefer_person: bool,
    /// Sensitive content requires a private-capable output.
    pub sensitive: bool,
}

impl Default for RouterPolicy {
    fn default() -> Self {
        Self {
            prefer_person: true,
            sensitive: false,
        }
    }
}

/// Inputs to a routing decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingInput<'a> {
    pub candidates: &'a [AudioEndpoint],
    pub room: Option<&'a AudioRoomId>,
    pub person: Option<&'a PersonId>,
    pub role: EndpointRole,
}

/// Output of a routing decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingOutput {
    pub endpoint_id: AudioEndpointId,
    pub hardware_class: HardwareClass,
}

/// Endpoint router port + deterministic default selection.
pub trait EndpointRouter {
    fn select(
        &self,
        input: RoutingInput<'_>,
        policy: RouterPolicy,
    ) -> Result<RoutingOutput, AudioError>;
}

/// Deterministic router implementing the selection rules.
#[derive(Debug, Clone, Default)]
pub struct DeterministicRouter;

impl EndpointRouter for DeterministicRouter {
    fn select(
        &self,
        input: RoutingInput<'_>,
        policy: RouterPolicy,
    ) -> Result<RoutingOutput, AudioError> {
        let candidates: Vec<&AudioEndpoint> = input
            .candidates
            .iter()
            .filter(|e| e.role == input.role)
            .filter(|e| e.availability.is_available())
            .collect();
        if candidates.is_empty() {
            return Err(AudioError::new(
                AudioErrorCode::NotFound,
                "no available audio endpoint for the requested role",
                None,
                None,
            ));
        }
        // Privacy: sensitive content never routes to a shared-room
        // audible output. The room field on the endpoint carries the
        // shared-room marker when the satellite is room-bound.
        if policy.sensitive && input.role == EndpointRole::Output {
            // Sensitive content requires a private-capable (person-bound)
            // output. A shared-room speaker is never selected; if no
            // private-capable endpoint exists the router fails closed
            // (LF-028 precedent, SPEC-012 behavior 9).
            let private_candidates: Vec<&AudioEndpoint> = candidates
                .iter()
                .copied()
                .filter(|e| e.person.is_some())
                .collect();
            if private_candidates.is_empty() {
                return Err(AudioError::new(
                    AudioErrorCode::NotFound,
                    "sensitive output requires a private endpoint; none available",
                    None,
                    None,
                ));
            }
            return Ok(pick_best(
                private_candidates,
                input.person,
                policy.prefer_person,
            ));
        }
        if policy.prefer_person {
            if let Some(person) = input.person {
                let person_candidates: Vec<&AudioEndpoint> = candidates
                    .iter()
                    .copied()
                    .filter(|e| e.person.as_ref() == Some(person))
                    .collect();
                if !person_candidates.is_empty() {
                    return Ok(pick_best(person_candidates, Some(person), true));
                }
            }
        }
        if let Some(room) = input.room {
            let room_candidates: Vec<&AudioEndpoint> = candidates
                .iter()
                .copied()
                .filter(|e| e.room.as_ref() == Some(room))
                .collect();
            if !room_candidates.is_empty() {
                return Ok(pick_best(room_candidates, input.person, false));
            }
        }
        Ok(pick_best(candidates, input.person, false))
    }
}

fn pick_best(
    candidates: Vec<&AudioEndpoint>,
    _person: Option<&PersonId>,
    _prefer_person: bool,
) -> RoutingOutput {
    // Deterministic tie-break: stable endpoint id order.
    let mut sorted = candidates;
    sorted.sort_by(|a, b| a.endpoint_id.cmp(&b.endpoint_id));
    let best = sorted[0];
    RoutingOutput {
        endpoint_id: best.endpoint_id.clone(),
        hardware_class: best.hardware_class,
    }
}
