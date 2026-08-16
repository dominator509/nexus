# EP-020 M3 Evidence -- Real Home Assistant provider integration

Node: EP-020 (Home Assistant provider and device control)
Milestone: M3 -- Real dependency and transport integration
Date: 2026-08-16
Runner: autonomous GraphLock execution (ledger lease `5dc7425`)

## Image anchor (immutable reproduction)

- Image: `ghcr.io/home-assistant/home-assistant:stable`
- Digest: `sha256:56690a89c79a0de98035e1719f8324a92d5859c1192ff45adb0230ea81cb42a5`
- License: Apache-2.0
- Running instance version: **Home Assistant 2026.8.2**
  (extracted from `homeassistant/const.py` of the pinned image:
  MAJOR=2026, MINOR=8, PATCH="2"; the same digest is the running instance)
- Component registered: `COMPONENT_REGISTRY.yaml` (home-assistant entry)

## Certification split (directive J -- exact)

- Home Assistant real server: **PASS**
- Home Assistant real authentication: **PASS**
- Home Assistant API provider integration: **PASS**
- Home Assistant command/readback: **PASS**
- controlled template-light entity (`light.nexus_test_light`):
  **CONTROLLED_TEST_FIXTURE**
- physical light hardware: **NOT ASSERTED / DEFERRED TO ITS
  CERTIFICATION OWNER** (no NODE_BLOCK; owned debt)

The fixture is entity-only: no Home Assistant device-registry object is
created for it, and it is never fabricated into a physical Device object
(`ep020_integration_entity_only_no_device_fabricated`).

## Real wire facts observed (decision-relevant)

- `GET /api/` authenticated returns `{"message": "API running."}` -- a
  JSON object, not a bare string (test assertion matches the real shape).
- Unknown service/action `POST /api/services/light/turn_sideways`
  returns **400 Bad Request** in this HA version (not 404).
- `POST /api/states/<entity_id>` is a state write; writing `"on"` to the
  fixture light does NOT change the backing `input_boolean` -- it never
  reaches the controlled entity. Only the service/action path
  (`/api/services/light/turn_on`) changes the backing entity
  (`ep020_integration_no_state_forgery_for_control`).
- WebSocket event receive: a socket timeout means "no event yet" -- the
  test receive loops poll against their deadline and the post-loop
  assertion still enforces the real requirement (no gate weakening).
- Auth bootstrap (headless): `auth` CLI verb `add` (no add-user /
  add-token), restart-to-load the user, then the REAL OAuth flow
  `login_flow -> authorization_code -> access_token`. Fresh random
  password + fresh token per run; nothing checked in.

## Real proof matrix (19 integration tests, real container)

Suite: `infra/home-assistant/tests/test_ep020_integration_home_assistant.py`
Runner: `sh scripts/ep020-m3-tests.sh`
Result: `19 passed in 88.58s` -- sentinel `EP-020 M3: ok`

- authenticated API request (`auth_check`)
- bad credential rejected (`bad_credential_fails`)
- discovery: fixture entities + `light`/`input_boolean` service domains
- entity state + attributes readback
- real service call accepted + target reaches expected state
  (`service_call_accepted`)
- exact-target verification after the command (`verify_after_service_call`)
- unknown entity 404, invalid service 400, missing sensor 404
- WebSocket: auth, subscribe, receive state_changed for the exact target
- unrelated entity change does not satisfy verification
  (`unrelated_change_not_verified`)
- verification window expiry is not success (`verification_timeout`)
- reconnect + resubscribe resumes event flow
- HA offline fails closed (`ha_offline_fails`)
- config mount is the repo fixture config; `infra/infra` never mounts
  (`config_mount_is_repo_config`)
- no state forgery for control (`no_state_forgery_for_control`)
- running HA version recorded (`version_recorded`)
- entity-only fixture never fabricated into a Device
  (`entity_only_no_device_fabricated`)
- container teardown leaves zero orphans (`container_cleanup...`)

## Teardown / hygiene (directive L)

- Container `nexus-ep020-ha` removed after the suite.
- Generated state in the mounted config dir (`.storage`, logs, db,
  default yaml/blueprint templates) removed after the suite -- only
  `configuration.yaml` is checked in.
- No test network or volume created; no OAuth/token temp material
  persisted; no helper processes remain.

## Root defect fixed this milestone

`ROOT = Path(__file__).resolve().parents[2]` resolved one level too
shallow for `infra/home-assistant/tests/`, mounting
`<repo>/infra/infra/home-assistant/config`; HA booted with DEFAULTS and
the fixture entities never appeared. Fixed to
`Path(__file__).resolve().parents[2].parent` (the real repo root) with a
pre-Docker assertion (`AGENTS.md` present, config present, no
`infra/infra`), a post-boot mount proof, and a regression test.
