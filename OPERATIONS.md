# OPERATIONS

## Daily operations

Use the Fleet view and `nexusctl status`. Required signals are control-plane health, PostgreSQL replication and disk, NATS stream lag, Temporal backlog, identity status, policy versions, secret lease health, edge reachability, backup age, provider health, cache ratio, costs, incidents, and release drift.

## Common failures

| Symptom | Diagnostic | Safe action |
| --- | --- | --- |
| Dashboard unavailable | Caddy health, control-plane readiness, Keycloak health | Fail over ingress or roll back release |
| Home commands delayed | edge CPU, Home Assistant WebSocket, local policy cache, workflow queue | Use local degraded profile; do not route known commands to cloud |
| DeepSeek latency or errors | provider health, cache hit, gateway circuit, budget | Fail over to certified provider or local route |
| NATS lag | stream status, consumer pending, disk, outbox | Pause noncritical producers and recover consumer |
| Workflow backlog | worker health, task queue, activity errors | Scale or restart workers; workflows remain durable |
| Voice failures | endpoint, mute, VAD, wake, STT, TTS, AEC, Bluetooth | Move session to mobile or API fallback |
| Camera unavailable | VLAN, camera, go2rtc, Frigate, vendor fallback | Notify loss; never expose camera publicly |
| Sentinel alert storm | sensor health, rule version, baseline, duplication | Increase observation, preserve evidence, avoid broad block |
| Backup stale | job, storage health, encryption key, capacity | Run backup and restore verification before update |
| Connector denied | token scope, binding, provider health, certification | Reauthorize least privilege; never use owner token |

## Backup and restore

Nightly encrypted database and configuration backup, configurable artifact backup, periodic full manifest, and weekly automated scratch restore. Monthly operator restore drill and pre-update backup. Recovery keys are stored separately. Restore creates a new deployment identity and reconnects edges through verified trust procedures.

## Scheduled jobs

Memory consolidation, retention, event compaction, workflow schedules, certificate rotation, secret lease rotation, dependency advisories, provider health, social queues, backup, restore verification, object integrity, hardware status, cache analysis, and security baselines.

## Incident triage

1. Acknowledge and preserve incident ID.
2. Determine safety, data, identity, external effects, and affected scope.
3. Contain using the smallest reversible action.
4. Preserve logs, traces, events, configuration, versions, and hashes.
5. Diagnose and reproduce in isolation.
6. Prepare remediation and rollback.
7. Obtain required approval.
8. Canary, verify, promote or roll back.
9. Notify affected users according to policy.
10. Close with cause, controls, memory, tests, and skill candidate where useful.

## Maintenance

Updates are staged, signed, backed up, and reversible. Sidecar updates have separate compatibility matrices. Hardware firmware updates require vendor notes, backup or recovery path, lab canary, and capability retest.

## Operational safety

Never run destructive database, firewall, identity, secret, storage, or device commands outside a documented workflow and approval. Never troubleshoot by exposing private services to the internet.

## Release operations (EP-043)

EP-043 owns the production readiness and ship boundary: the readiness
evaluation engine, the release manifest producer, and the operational
commands below. All commands are real repository tools under
`release-evidence/` and run under Node 24 native TypeScript with the
resolution-only ESM loader. Commands read the real repository state
(GRAPH.md, LEDGER.md, live-fire registry, certification RESULTS.md
files) and the real release artifact bytes under
`infra/release/fixtures/components/`.

### Prerequisites

- Working tree checked out at the release candidate commit.
- Node 24 with `--experimental-transform-types` support.
- Workspace dependencies installed (`pnpm install` at the repository root).
- No secret values in the environment are required by any command.

### Generate the production readiness report

```sh
cd /root/nexus
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts readiness --output PRODUCTION_READINESS.md
```

Expected output:

```text
readiness: NOT_READY (N blocking reasons)
wrote PRODUCTION_READINESS.md (M bytes)
```

Exit 0 when the report is written. The report declares production
readiness only when every acceptance obligation is met with real
evidence; otherwise it declares NOT READY with the exact blocking
reasons. Blocking reasons are computed from repository state, never
accepted as input.

### Build the release manifest

```sh
cd /root/nexus
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts manifest --output-dir dist/release
```

Expected output:

```text
manifest: wrote dist/release/RELEASE_MANIFEST.json digest=sha256:...
manifest: 2 components, signatures PRESENT_NOT_VERIFIED
```

Exit 0 when the manifest is written. Component digests are real sha256
over the real artifact bytes. Signature state is
`SIGNATURE_PRESENT_NOT_VERIFIED`: no release signing key is exercised,
so no signature is claimed verified.

### Verify the release manifest

```sh
cd /root/nexus
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts verify-manifest --manifest dist/release/RELEASE_MANIFEST.json
```

Expected output on success:

```text
verify-manifest: ok (2 components verified against real artifact bytes)
verify-manifest: manifest digest sha256:... valid
```

Exit 0 only when every component digest matches the real artifact bytes
and the manifest digest is valid. The command fails closed with a typed
error on: missing manifest, tampered manifest digest, missing artifact
bytes, or component digest mismatch. Exit code 1 with a `VERIFICATION_FAILED`
or `NOT_FOUND` message on failure.

### Inspect the ship gate

```sh
cd /root/nexus
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts ship-gate-status
```

Expected output shape:

```text
ship-gate verdict: BLOCKED
readiness decision: NOT_READY
obligation: all graph nodes are DONE: NOT MET
...
blocking reasons (N):
  - ...
```

Exit 0 when the inspection itself succeeds. The verdict field carries
the truth: `PASSED` only when every obligation is met, otherwise
`BLOCKED`. Inspecting a blocked gate is a successful observation, not a
pass. Ship gate execution and signing are NOT performed by this command.

### Inspect certification rows

```sh
cd /root/nexus
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts certification-rows
```

Expected output shape:

```text
PROVIDER provider-1-... RELEASE-BLOCKING-PENDING
HARDWARE hardware-1-... RELEASE-BLOCKING-PENDING
certification rows: 2
```

Exit 0 when the rows are read from the real RESULTS.md files. A
`RELEASE-BLOCKING-PENDING` or `PENDING` row means the release is
blocked for that certification domain.

### Runtime health check

The control plane health endpoint is the runtime smoke source:

```sh
curl -fsS http://127.0.0.1:8443/healthz
```

Exit 0 and a 200 response means the control plane is healthy. The
canonical node verify ladder requires the control plane to be up for
the runtime smoke stage; start it with the repository canonical
local-start before running the ladder.

The runtime smoke base URL resolves by precedence
(`scripts/smoke/runtime.sh` and `scripts/live-fire/LF-029.sh`):

1. `NEXUS_SMOKE_URL` - operator override, always wins. Set this when
   hosting remotely (public domain or dynamic IP), e.g.
   `export NEXUS_SMOKE_URL=https://nexus.example.com`.
2. `NEXUS_BASE_DOMAIN` - used only when it is a real deployable domain
   (`https://<domain>`); local/test placeholders (`.test`, `.local`,
   `.example.test`, `localhost`) are ignored and fall through.
3. Canonical local mapping `http://127.0.0.1:8443` - the compose core
   profile binds the control plane on host `127.0.0.1:8443`.

Every deployment (including a fresh clone on a foreign host or dynamic
IP) must set `NEXUS_SMOKE_URL` in its `.env` (or use a real
`NEXUS_BASE_DOMAIN`) so the verify ladder smoke stage targets the
actual control plane.

### Release evidence refresh

After any evidence-bearing commit, regenerate the readiness report and
the release manifest, verify the manifest, and commit the refreshed
artifacts:

```sh
cd /root/nexus
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts readiness --output PRODUCTION_READINESS.md
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts manifest --output-dir dist/release
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts verify-manifest
```

The refreshed report binds the new commit via its `git_commit` audit
field. Evidence commits must be followed by a canonical node verify on
that exact commit.

### Fresh-clone procedure

The fresh-clone-equivalent rerun proves the release builds and verifies
from a clean checkout with no hidden local state:

```sh
tmp=$(mktemp -d)
git clone --depth 1 file:///root/nexus "$tmp/nexus"
cd "$tmp/nexus"
pnpm install
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts readiness --output PRODUCTION_READINESS.md
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts manifest --output-dir dist/release
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts verify-manifest
```

The operational path above is exercised by the M3 integration suite.
The full fresh-clone-equivalent rerun as a release acceptance
obligation is owned by M5 and remains NOT ASSERTED until it runs.

### Rollback

Rollback procedure and proofs are owned by M5 under `ROLLBACK.md`.
Until then, rollback execution remains NOT ASSERTED; this document
does not claim a rollback procedure beyond the reference.

### Exit and sentinel semantics

- `readiness`: exit 0 = report written; report content carries the verdict.
- `manifest`: exit 0 = manifest written.
- `verify-manifest`: exit 0 = every digest verified; exit 1 = typed
  `VERIFICATION_FAILED` or `NOT_FOUND` failure.
- `ship-gate-status`: exit 0 = inspection succeeded; verdict field
  carries the truth (`PASSED` or `BLOCKED`).
- `certification-rows`: exit 0 = rows read.
- No command exits 0 on a missing dependency or a failed verification.

### Component and dependency facts

- `@nexus/release-evidence` 0.1.0: workspace package, private, ESM.
  Source: this repository (`release-evidence/`). License: Nexus
  proprietary (internal). Replacement contract: the SPEC-008
  vocabulary and the M1 contract surface (ShipGate, ReleaseEvidence,
  ManualDeployHandoff, ProductionReadinessDecision).
- Transport: local filesystem repository state and local artifact
  bytes under `infra/release/fixtures/components/`. No cloud transport
  is exercised by any command in this document.
- Runtime: Node 24 native TypeScript with the resolution-only ESM
  loader (`release-evidence/scripts/ts-resolve-loader.mjs`). The loader
  is shared from the EP-042 M4 installer machinery and never rewrites
  file contents.

### Not available

The following are NOT available and are never advertised as available:

- Real release signature verification (all signatures remain
  `SIGNATURE_PRESENT_NOT_VERIFIED`).
- Ship gate execution and signing (EP-043 M5).
- Fresh-clone-equivalent rerun as an acceptance obligation (EP-043 M5).
- Production deployment and rollback execution (M5; auto-deploy is not
  authorized).
- AWS S3, R2, and B2 transport (EP-042 did not certify cloud transport).
