# EP-020 M5 Evidence — Real provider live-fire and node closure

Node: EP-020 (Home Assistant provider and device control)
Milestone: M5
Date: 2026-08-16
Runner: autonomous GraphLock execution (ledger lease `5dc7425`)
Correction: 2026-08-16 (pre-closure automation create() repair)

## Certification split (exact; directive J)

- Home Assistant real server: **PROVIDER_CERTIFIED** (M3 real container)
- Home Assistant real authentication: **PROVIDER_CERTIFIED** (M3 OAuth
  flow + M5 proof auth_check)
- Home Assistant API provider integration: **PROVIDER_CERTIFIED** (M3
  discovery/state/services)
- Home Assistant command/readback: **PROVIDER_CERTIFIED** (M3 service
  call + exact-target verification)
- Home Assistant automation behavior (conditional execution /
  cancellation / persistence): **PROVIDER_CERTIFIED** (LF-007 through
  the real config-API create() path)
- programmatic automation creation
  (`AutomationHandoffAdapter::create`): **PROVIDER_CERTIFIED** (repaired
  against real HA 2026.8.2, real-live-fired; see correction below)
- controlled template-light entity (`light.nexus_test_light`):
  **CONTROLLED_TEST_FIXTURE**
- physical light hardware: **NOT ASSERTED / DEFERRED TO ITS
  CERTIFICATION OWNER** (no NODE_BLOCK)

The fixture is entity-only; it is never fabricated into a physical
Device object (M3 entity-only regression).

## Pre-closure correction (automation handoff)

The M5 gate exposed a real production-path defect: the original
`AutomationHandoffAdapter::create()` called the `automation.turn_on`
service with a full config payload, and real HA 2026.8.2 returns HTTP
400 (it is an enable service, not a create service).

Ownership (directive A): EP-020 owns creation/provisioning of real
runnable Home Assistant automations. Sources: SPEC-011 required
behavior 4; EP-020 node contract (`AutomationHandoff` public
interface); ExecPlan interface map; contract.rs trait doc; ADR-027. No
later graph node owns HA automation provisioning. (Full record in the
ExecPlan Decision Log.)

Real mechanism (directive B, verified from the pinned image source):
`POST /api/config/automation/config/<id>` is HA's real supported
provisioning API. It validates the automation config
(`async_validate_config_item` / PLATFORM_SCHEMA), writes it ATOMICALLY
to `automations.yaml`, then fires `automation.reload` with
`{CONF_ID: <id>}` which creates the runnable automation entity. The
fixture now uses the standard `automation: !include automations.yaml`
layout and starts with an EMPTY `automations.yaml`; the LF-007
automations are created at runtime through the adapter (no pre-written
YAML bypass).

Creation semantics (directive C): `create()` returns a handle ONLY
after the automation entity appears through provider readback and is
enabled. Provider acceptance alone is never success. SUBMITTED !=
VERIFIED is preserved: `create()` proves provisioning; the LF-007 proof
chain proves behavioral verification (condition true executes, false
cancels, restart persists).

Second defect found by the mandatory M5 rerun (directive I): the
adapter derived canonical `DeviceId` from the `/api/states`
ENUMERATION INDEX, which is NOT stable across restarts (HA re-registers
entities in a different order). After reconnect, a queued intent
silently re-targeted a different device (observed: `light` intent
executed `input_boolean/turn_on`). Fixed with `stable_device_id()`:
canonical id is a deterministic UUIDv7-shaped FNV-1a mix of the exact
provider entity id — identity survives restart and discovery refresh
(contract requirement). Regression test:
`ep020_unit_device_identity_survives_enumeration_order_change`.

## Real proofs (this milestone)

- `LF-006` `deterministic-home-control` — production adapter drives the
  REAL pinned HA container: real auth, discovery -> canonical DeviceTwin,
  deterministic fast path EXECUTE_LOCALLY (no model call - the proof
  process never constructs a model provider), real `light.turn_on`
  service call -> CommandReceipt SUBMITTED (COMMAND ACCEPTED != DEVICE
  VERIFIED), exact-target fresh-readback verification -> VERIFIED, and
  the driver observes the real HA `state_changed` audit event for the
  exact target on the WebSocket.
- `LF-007` `conditional-home-workflow` — canonical `AutomationSpec`
  (name, provider_trigger, provider_condition, action intent) ->
  `AutomationHandoffAdapter::create()` -> `POST
  /api/config/automation/config/<id>` (real HA 2026.8.2 config API) ->
  runnable automation entities appear through provider readback and are
  enabled BEFORE create() returns; persistence proven by readback after
  a real container restart (durable automations.yaml reload); correct
  conditional EXECUTION (condition true -> action runs) and conditional
  CANCELLATION (condition false -> action does not run). The production
  adapter reads/identifies the installed automations afterward
  (readback). Temporal boundary recorded: real Temporal machinery is
  proven by the EP-019 workflow suite; a Temporal-hosted home workflow
  is owned by the Temporal-owning/deployment nodes.
- `LF-024` `offline-degraded-operation` — with the real provider stopped
  (cloud/public-internet analog unreachable): command execution fails
  closed (typed, never fabricated success); the bounded idempotent
  offline queue retains the authorized command (duplicate -> CONFLICT);
  low-risk local capability retained offline (deterministic fast path
  EXECUTE_LOCALLY, no model call); reconnect refreshes canonical state
  (stable device identity survives the restart) and drains the queue
  through the real service path with exact-target verification
  (`queued_verified: VERIFIED`).

## Real wire facts recorded (decision-relevant)

- `POST /api/config/automation/config/<id>` IS the real supported
  provisioning API in HA 2026.8.2: validates, writes `automations.yaml`
  atomically, reloads the single automation, creates the runnable
  entity. The earlier "stores but does not activate" observation was a
  FIXTURE artifact: `configuration.yaml` defined automations inline and
  never included `automations.yaml`, so the reload hook re-read a
  config that did not contain the written file. Fixed with the standard
  `automation: !include automations.yaml` layout.
- `automation.turn_on` is an ENABLE service, not a create service (400
  on config payload) — the original create() defect.
- Template-light mirror caveat: actions must target a non-derived entity
  (switch2) so conditional execution/cancellation is not confounded.
- `/api/states` enumeration order is NOT stable across restarts;
  canonical device identity must be derived from the provider entity id
  (stable_device_id), never the enumeration index.

## Gates (observed at node closure)

- `EP-020 M5: ok` (node gate: artifact check + full nexus-home cargo
  suite + LF-006 + LF-007 + LF-024)
- `node verify EP-020: ok`
- `scope audit EP-020: ok`
- security / license / reality / format / lint / dependency gates: ok

## Teardown / hygiene

- HA container removed after each suite; generated config state removed
  (only configuration.yaml checked in; automations.yaml is generated per
  run by the config API and cleaned in teardown); no credentials/tokens
  persisted; no orphan containers/processes; control plane runtime
  stopped cleanly if running.
