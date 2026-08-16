# ADR-027 - Home Provider Vocabulary and Authority Semantics

Status: Accepted
Date: 2026-08-16
Owner: EP-020 (Home Assistant Provider and Device Control)

## Context

SPEC-011 defines Home Assistant as the primary abstraction for home
device state and automation truth, with a local fast path, state
verification, and provider-neutral device capabilities. The canonical
terms are HomeProvider, Area, Device, Entity, DeviceCapability,
FastPathIntent, StateVerification, AutomationHandoff, and RobotProvider.
None of these vocabulary classes existed in `crates/nexus-domain` or a
home crate. EP-020 owns the home provider contracts and must encode the
authority distinctions the owner directive requires: a Home Assistant
service call being accepted means SUBMITTED, never VERIFIED;
`POST /api/states/<entity_id>` must never be the implementation of a
physical command; unrelated state changes never satisfy verification;
unknown/unavailable remains unknown; and the model may propose device
intents but can never call Home Assistant directly outside the Action
Gateway.

## Decision

Add the EP-020-owned vocabulary in `crates/nexus-home` (vocabulary
module), documented in `docs/vocabulary/README.md`, with unknown-value
rejection at parse time:

- `DeviceCategory`: `LIGHT`, `SWITCH`, `LOCK`, `CLIMATE`, `COVER`,
  `SENSOR`, `BINARY_SENSOR`, `MEDIA_PLAYER`, `CAMERA`, `FAN`, `VACUUM`,
  `ALARM`, `SCENE`, `BUTTON`, `NUMBER`, `SELECT`, `OTHER`. Provider
  domain names are normalized into these canonical categories at the
  infrastructure boundary; HA domain names never leak upward.
- `CommandState`: `AUTHORIZED`, `SUBMITTED`, `VERIFICATION_PENDING`,
  `VERIFIED`, `VERIFICATION_TIMEOUT`, `UNKNOWN`. There is no
  `FIXED`/`VERIFIED-as-accepted` escape. Provider acknowledgement is
  SUBMITTED; only exact-target observed verification produces VERIFIED.
- `EntityAvailability`: `AVAILABLE`, `UNAVAILABLE`, `UNKNOWN`. Unknown
  remains unknown; never treated as off/closed/locked/safe.
- `FastPathDecision`: `EXECUTE_LOCALLY`, `REQUIRES_MODEL`, `DENIED`.
  Known low-risk commands execute locally without model calls after
  authorization.
- `VerificationOutcome`: `VERIFIED`, `TIMEOUT`, `UNRELATED_CHANGE`,
  `MISMATCH`, `UNKNOWN`. Unrelated state_changed events never satisfy
  verification.
- `ProviderConnectionState`: `CONNECTED`, `DEGRADED`, `DISCONNECTED`,
  `RECONNECTING`. A dropped WebSocket is typed, never a silent claim of
  live cache.

Contract types owned by EP-020 in `crates/nexus-home`:

- `DeviceTwin` (canonical `Device`): stable canonical `DeviceId`,
  mutable `friendly_name` (never identity), `AreaId`, owner `PersonId`,
  `HaDeviceRef`/`HaEntityRef` provider references, provider domain,
  canonical `DeviceCategory`, manufacturer/model, availability, state,
  attributes, capabilities, via-device topology.
- `HomeIntent` (canonical intent; `FastPathIntent` is its fast-path
  subset): canonical device, capability, action, parameters,
  correlation, idempotency key.
- `CommandReceipt`: provider acceptance proof; state is SUBMITTED at
  most, never VERIFIED.
- `FastPathMatcher` port: deterministic local fast-path decision, never
  consults a model.
- `StateVerifier` port + deterministic `StateVerifierAdapter`:
  exact-target binding; unrelated entity change is UNRELATED_CHANGE.
- `AutomationHandoff` port + `AutomationSpec`/`AutomationHandle`/
  `AutomationStatus`: real provider automation creation/invocation/
  readback, never a fabricated automation object.
- `HomeProvider` port: discovery, read, execute, verify, reconnect.
- `HomeAssistantProvider` port: HA-specific surface behind the same
  authority boundary.
- Typed errors `HomeError`/`HomeErrorCode` with SPEC-006 codes
  (validation, authorization, policy, not-found, conflict, unavailable,
  timeout, verification, vocabulary, external, internal), preserving
  correlation and redacting secrets.

Authority semantics locked by this ADR:

1. **COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.** A Home
   Assistant service/action request being accepted means EXECUTED/
   SUBMITTED, not VERIFIED. Verification binds to the exact target
   entity, the requested action, and the expected resulting
   state/attribute; unrelated state_changed events never satisfy it.

2. **`POST /api/states/<entity_id>` is never physical device control.**
   Home Assistant documents that this endpoint updates the in-HA
   representation and does not communicate with the actual device.
   Production Nexus device commands use the real service/action
   mechanism (`/api/services/<domain>/<service>` or the equivalent
   WebSocket call). State writes are allowed only for synthetic/
   state-only entities and never satisfy device command execution or
   verification. A regression test enforces the absence of the
   state-write path in the adapter (M4).

3. **Unknown/unavailable remains unknown.** `EntityAvailability::Unknown`
   is never treated as off/closed/locked/safe; stale cache is never used
   to claim verification.

4. **The model is never device authority.** A model may produce a
   `HomeIntent` (device, action, parameters); it cannot call Home
   Assistant directly outside the Action Gateway. Authorization belongs
   to EP-008; provider credentials are infrastructure credentials, never
   user authorization. Risk/step-up decisions come from upstream policy,
   not a second competing risk system in the adapter.

5. **Deterministic local fast path.** Known low-risk commands execute
   locally without model calls after authorization; the matcher is
   deterministic and never consults a model.

6. **Reconnect/resubscribe is proven, never assumed.** A dropped
   WebSocket transitions to a typed DISCONNECTED state; reconnect must
   authenticate, resubscribe, refresh canonical state, and resume event
   flow. Cached state is never claimed live while disconnected.

7. **Automation handoff is real.** Automation creation/invocation/
   readback use the provider's real automation machinery; the handoff
   never fabricates an automation object. If persistent automation
   authoring is later-owned, the interface is recorded now and provider
   certification is deferred to the named owner.

## Alternatives considered

- Treating provider acknowledgement as success (rejected: violates the
  primary invariant that command-accepted != device-verified).
- Using `POST /api/states/<entity_id>` for fast control (rejected:
  does not communicate with the actual device; forbidden by the
  directive).
- Letting any state change satisfy verification (rejected: unrelated
  events would create false success).
- Treating UNKNOWN/UNAVAILABLE as OFF/CLOSED/LOCKED/SAFE (rejected:
  dishonest and unsafe).
- Embedding a second risk model in the HA adapter (rejected: risk
  decisions belong to EP-008 policy).

## Consequences

The home contract is fail closed and unambiguous: a provider acceptance
is SUBMITTED, verification is exact-target, unknown is unknown, and the
adapter cannot forge device state. Later implementation cannot confuse
command submission, device change, or device verification. The
vocabulary README and the nexus-home crate must stay in sync; new public
names require a new ADR and schema/vocabulary update.

## Reversal

Reversing requires a new ADR demonstrating that the authority
distinctions are preserved by an equivalent or stronger model.
