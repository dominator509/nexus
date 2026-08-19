# EP-027 Fax Operations (HylaFAX)

Operational runbook for the EP-027 fax fabric: the `nexus-hylafax`
connector (`connectors/hylafax/`), its real HylaFAX controlled fixture
(`infra/hylafax/`), the forced-failure tooling (`infra/fax/`), and the
LF-030 live-fire evidence (`tests/fax/`, `scripts/live-fire/LF-030.sh`).

Certification boundary (always current in
`.agent/state/evidence/EP-027-M3.md` / `EP-027-M4.md` / `LF-030-ep027-m5.json`):
nexus-hylafax IMPLEMENTED; hfaxd control protocol, active EPRT data
channel, MODE Z/STOT upload PROTOCOL_CERTIFIED; HylaFAX 6.0.6-8.1
fixture, faxq acceptance, exact query/readback PROVIDER_CERTIFIED;
container CONTROLLED_TEST_FIXTURE; **physical modem, PSTN, remote fax
receiver, and DELIVERED are NOT ASSERTED**.

## Components

- `connectors/hylafax/` - production adapter (`HylaFaxProvider`) and
  real hfaxd transport (control + active EPRT data channel).
- `infra/hylafax/provision-fixture.sh` - idempotent fixture bootstrap
  (pinned image digest, credential ensure, toolchain, /build workspace
  derived from the repo).
- `infra/fax/hylafax-diag.sh` - operations diagnostic + bounded
  recovery (`diagnose` / `recover`).
- `tests/fax/` - LF-030 live-fire E2E crate (`nexus-fax-e2e`).
- `scripts/live-fire/LF-030.sh` - live-fire entry point.
- `scripts/ep027-m{1..5}-tests.sh` - milestone reality gates.

## Configuration

The connector takes host, port, username, password, and minimum
approval class. The fixture uses the controlled `nexustest` user with a
known TEST-ONLY password (never production, never in evidence).
Production deployments must supply real credentials through the normal
secret path; the fixture credential must never be reused.

hfaxd authentication facts (observed):

- the `localhost` hosts.hfaxd entry auto-authenticates (USER -> 230
  with no password);
- non-loopback sources hit the wildcard entry (USER -> 331 -> PASS ->
  230/530). Wrong credentials produce a real 530 -> canonical
  AUTHORIZATION.

## Diagnostics

```sh
sh infra/fax/hylafax-diag.sh diagnose
sh infra/fax/hylafax-diag.sh recover   # bounded restart of hfaxd
```

The diagnostic prints container state, hfaxd/faxq pids, the hfaxd
greeting (version), and spool counts. It never prints credentials or
document content.

## Submission and state interpretation

- A JSUBM 200 + the job visible in `sendq` proves SUBMITTED only.
- SUBMITTED != DELIVERED. DELIVERED requires real terminal delivery
  evidence (modem/PSTN/remote receipt), which is NOT ASSERTED for this
  fixture.
- The governed path sends the FULL observed JPARM set (FROMUSER,
  LASTTIME, MAXDIALS, MAXTRIES, SCHEDPRI, DIALSTRING, NOTIFYADDR,
  VRES, PAGEWIDTH, PAGELENGTH, NOTIFY, PAGECHOP, CHOPTHRESHOLD +
  DOCUMENT). Incomplete jobs are NAK'd by the real scheduler (460/504)
  and never claimed SUBMITTED.
- `status()` reads the exact provider job id through LIST sendq; the
  state letter maps conservatively (W/B/S/R/D/E -> SUBMITTED ceiling,
  F -> FAILED, unknown -> fail closed).

## Reconciliation and exact-target lookup

To verify a job, query by the provider-assigned CARRIER job id (never
by destination or owner). Independent spool verification:
`/var/spool/hylafax/sendq/q<id>` exists and
`/var/spool/hylafax/docq/doc<docid>.ps.<id>` matches the uploaded
document digest byte-for-byte.

## Failure handling

- Provider down: transport fails closed UNAVAILABLE; the hfaxd-down
  live test terminates and restarts the real daemon and proves
  recovery.
- Policy denial: governed gates run before any provider mutation;
  denied sends create zero jobs (spool-proven).
- Duplicate in-flight command: CONFLICT until the first completes.
- Ambiguous outcomes are never blindly retried; verification fails
  closed.

## Redacted logging

Telemetry never contains PASS credentials, raw document bodies, or
unnecessary destination numbers. The adapter registers the credential
as a redaction secret at construction (poison-safe). Correlation ids
are canonical `fax-<nanos>-<seq>`.

## Shutdown, backup, restore, upgrade, disable, rollback

- Shutdown: `docker stop nexus-hylafax-fixture` (hfaxd/faxq stop with
  the container). The connector fails closed UNAVAILABLE when the
  provider is down.
- Backup/restore: the fixture spool (`/var/spool/hylafax`) is
  ephemeral container state; treat it as disposable. Production spool
  backup is out of scope for this controlled fixture.
- Upgrade: the fixture is pinned by image digest
  (`sha256:00decb6c...`). A deliberate version change requires a new
  pinned digest, capture/probe re-validation, and a Decision Log
  entry. Runtime version drift (VERSIONS.lock 6.0.7 vs tested
  6.0.6-8.1) is recorded and owned by the later lockfile owner.
- Disable: do not advertise the HylaFAX provider as operational for
  DELIVERED until modem/PSTN certification exists. The adapter itself
  fails closed when unconfigured.
- Rollback: `sh infra/hylafax/provision-fixture.sh` is idempotent;
  re-running it restores the fixture to the pinned, known state
  (credential ensure + workspace re-derivation).

## Live-fire

```sh
sh scripts/live-fire/LF-030.sh
```

Requires current-run evidence at
`.agent/state/evidence/LF-030-ep027-m5.json` embedding
`EP027_M5_RUN_ID` (stale evidence never satisfies the gate).
