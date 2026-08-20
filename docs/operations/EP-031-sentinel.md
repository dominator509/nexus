# EP-031 Advanced Sentinel Operations Runbook

Applies to the EP-031 advanced detection components: the
`nexus-sentinel-advanced` contract crate, the Suricata/Zeek/CrowdSec/
Wazuh/osquery connectors, the LF-009 sentinel-quarantine live-fire,
and the OPNsense containment engine (EP-030) they compose.

## Component map

- `infra/sentinel/advanced/` - provider-neutral contract crate
  (`nexus-sentinel-advanced`). Vocabulary, models, ports, fail-closed
  unbound implementations.
- `connectors/suricata/` - Enhanced profile boundary + EVE JSON
  vocabulary (transport owned by deployment).
- `connectors/zeek/` - Advanced profile network detection over the
  documented Zeek JSON Streaming Logs surface.
- `connectors/crowdsec/` - optional reputation enforcement over the
  documented CrowdSec LAPI surface.
- `connectors/wazuh/` - Endpoint profile alerts over the documented
  Wazuh server API.
- `connectors/osquery/` - Endpoint profile collector implementing the
  documented osquery TLS remote API (enroll / distributed_read /
  distributed_write) as a self-hosted server.
- `infra/sentinel/advanced-live-fire/` - LF-009 sentinel-quarantine
  live-fire evidence crate (real-socket journey).

## Health and readiness

- Connector health: `sh scripts/wazuh-diag.sh` is the Wazuh
  fail-closed diagnostic (unreachable -> rc=3 reachable=no;
  configured != healthy; bounded recovery = one re-probe). The same
  fail-closed principle applies to every connector: capabilities are
  advertised only when the transport answers; an unbound provider
  fails closed with Unavailable and never fabricates telemetry.
- osquery collector: `POST /enroll` accepts a node only with the
  configured enroll secret; `POST /distributed_read` issues the owned
  queries; `POST /distributed_write` collects observed rows. A
  non-zero distributed status is an OBSERVED query failure and fails
  closed (ExternalProvider).
- Runtime smoke: run `sh scripts/nodes/EP-031.sh M5` and observe
  `EP-031 M5: ok`.

## Diagnostics and failure classification

All failures use SPEC-006 codes: Validation (400-class/malformed
input), Authorization (401/403, denied enroll secret, invalid
node_key), NotFound (404), Conflict (409), RateLimit (429), Timeout
(silent peer), Unavailable (5xx, refused connect, unbound provider),
ExternalProvider (malformed JSON, failed osquery distributed query).
Fail closed: an unbound or failing provider never fabricates events.

## Authentication and secrets

- Zeek: no credentials (log surface).
- CrowdSec: machine_id/password used ONLY for the documented
  `/v1/watchers/login` exchange; JWT cached with ONE bounded re-login
  on 401. Never an unbounded retry loop.
- Wazuh: username/password used ONLY for `/security/user/
authenticate`; JWT sent as Bearer on `/alerts`; ONE bounded
  re-authenticate on 401/403.
- osquery: enroll secret used ONLY to validate enrollment; node_key
  cached per node.
- OPNsense (EP-030): API key/secret for the firewall automation API.
- All credentials are redaction-registered: they never appear in
  audit rings, evidence, errors, or diagnostics (canary-tested).

## Correlation and triage

- Incidents correlate over COMPATIBLE OBSERVED FACTS: a shared
  observed source indicator (e.g. the scanner IP) corroborated by
  independent observation planes (network + reputation + endpoint).
  Confidence is High when >=2 independent planes corroborate the SAME
  indicator; raw sensor count never inflates confidence.
- Priority derives from observed severity and confidence only.
- Investigation preserves evidence references; conclusions reference
  observed evidence.

## Containment and rollback

- Bounded containment (Quarantine/Block/IsolateEndpoint) may be
  preauthorized when high-confidence and reversible.
- Destructive response (Wipe/FactoryReset/BroadLockout/
  CredentialRotation) is NEVER preauthorized and always requires
  human procedure (ApprovalClass Human or stronger); planning under
  an insufficient approval class fails closed.
- Containment lifecycle (OPNsense): propose (DATA) -> approve ->
  apply (addRule + apply) -> verify (independent exact-target
  readback by `nexus-quarantine-<proposal_id>`) -> revoke (toggleRule
  0 + apply) -> verify fails after revoke.
- Verification is only true when independent readback proves the
  exact effect; it is never assumed.

## Zero-orphan cleanup

- Live-fire fixture threads are bounded (deadline + serve count) and
  joined at test end; no sidecar processes, no listeners, no temp
  checkpoints survive a green run.
- Check: `ps aux | grep -E "wazuh|ep031"` and `ss -tlnp | grep 59999`
  must be empty after a run.

## Evidence

- LF-009 evidence: `.agent/state/evidence/LF-009-ep031-m5.json`,
  embedding `EP031_M5_RUN_ID`. Stale evidence (run_id mismatch)
  never satisfies the gate. The evidence records sensor observations
  with provenance, the correlated incident, triage priority,
  authorization state, execution rule_ref, verification state,
  rollback state, redaction result, and the certification boundary.

## Certification boundary

- Advanced contract (`nexus-sentinel-advanced`): INTERNAL_CERTIFIED.
- Connector transports: TRANSPORT_CERTIFIED against controlled
  real-socket fixtures (production transports never mocked; mocks
  control the peer only).
- Real Suricata/Zeek/CrowdSec/Wazuh/osquery sensors: NOT ASSERTED.
- Real OPNsense/OpenWrt firewall appliances: NOT ASSERTED.
- Certification debt owned by deployment/ship review (EP-040/EP-043).

## Recovery

- Idempotence: rerunning any gate is safe; evidence is rewritten per
  run with a fresh run_id.
- Rollback: `git revert` the milestone commit, or `git checkout`
  the previous green tag; never cross a completed green tag.
- Disk: if a workspace battery fails with disk pressure, prune
  dangling docker volumes (`docker volume prune -f`) - environmental,
  not a code defect.
