# EP-032 Notification and Communications Router -- Operations

Operational runbook for the Nexus notification plane (SPEC-014
behavior 7): mobile push and SMS channels, privacy routing, policy
gates, escalation, reconciliation, and observability.

Only exercised behavior is documented here. Everything below was
proven against the real production components in EP-032 M1?M5 gates
(`scripts/ep032-m1-tests.sh` ? `scripts/ep032-m5-tests.sh`).

## 1. Channel providers

- **Mobile push** (`connectors/push`, `PushChannelProvider`):
  transport writes the canonical `NotificationEnvelope` as one JSON
  line and reads one ack line
  (`{"provider_ref","delivered","delivered_at_ms","error"}`,
  `deny_unknown_fields`). Any duplex byte source works (socket, pipe,
  file). Malformed ack, closed peer, and unknown ack fields fail
  closed (`External`); ack `delivered=false` is an OBSERVED `Failed`
  receipt -- never fabricated into Delivered.
- **SMS** (`connectors/sms`, `SmsChannelProvider` over
  `GammuSmsdGateway` + `SqliteSmsDb`): the production boundary is the
  DOCUMENTED Gammu SMSD SQL service surface (outbox insert with
  `CreatorID` = NotificationId and `DeliveryReport=yes`, then
  sentitems status readback). Gammu libraries are NOT linked -- the
  connector speaks the daemon's database, nothing else.

## 2. Health and diagnostics

- `scripts/sms-diag.sh -c <smsdrc> -d <db>` classifies truthfully:
  configured / provider_db (schema reachable) / daemon process /
  provider queue writable. It never reports `healthy` from
  configuration existence alone; missing config exits non-zero
  (fail closed).
- Push transport health is per-delivery: a bound transport answers a
  delivery; an unbound provider advertises nothing (`available()`
  false) and fails closed `Unavailable`.

## 3. Schema

- SQLite schema **17** is certified (the SHIPPED package schema,
  `/usr/share/doc/gammu-smsd/examples/sqlite.sql`). The connector
  fails closed at open/connect when the version row is missing or not
  17 (`External` / configuration) -- it never runs partial SQL against
  an unknown schema. Current upstream docs describe schema 18; that
  is NOT certified.
- Postgres backend (`PostgresSmsDb`) is implemented with schema-17
  validation but is NOT certified (no live Postgres fixture
  exercised; certification debt owned by deployment/ship review).

## 4. Outbox / sentitems interpretation

- **outbox** row = enqueued, not yet submitted (`Reserved` ->
  `Pending`).
- **sentitems** is the provider's post-submission record. Status
  vocabulary (documented SMSD Database Structure):
  - `SendingOK` / `SendingOKNoReport` / `DeliveryPending` /
    `DeliveryUnknown` -> canonical `Sending` (in flight, NO delivery
    authority).
  - `DeliveryOK` WITH `DeliveryDateTime` -> canonical `Delivered` (the
    ONLY delivered authority).
  - `DeliveryOK` WITHOUT `DeliveryDateTime` -> `Sending` -- **not**
    Delivered.
  - `SendingError` / `Error` / `DeliveryFailed` -> canonical `Failed`.
  - Unknown status -> `External` fail closed (never guessed).

## 5. Delivery semantics (read twice)

- **SendingOK != Delivered.** Provider acceptance is never a delivery
  authority.
- **DeliveryOK without DeliveryDateTime != Delivered.**
- Only the real daemon evolves provider state. Nexus never writes
  sentitems, never manufactures `SendingOK` / `DeliveryOK` /
  `DeliveryFailed`, and never inserts `DeliveryDateTime` manually.
- A delivery report is exact-target: a report for message X can never
  satisfy message Y (NotificationId / CreatorID / provider row ID /
  TPMR correlation).
- No delivery report after a successful `AT+CMGS` (SendingOK only)
  leaves the message in `Sending` -- it is never promoted.

## 6. Reconciliation and idempotency

- `SmsDb::reconcile_by_creator` resolves the durable identity
  (outbox then sentitems by `CreatorID` = NotificationId) BEFORE any
  insert. An ambiguous submission (provider insert may have
  committed, client confirmation lost) is reconciled to exactly one
  provider row -- never a blind duplicate.
- Durable idempotency is cross-process: the provider row is the
  durable record, so a fresh connector instance replaying the same
  notification identity reconciles to the same single row. The
  in-memory ring covers process lifetime only.
- Duplicate NotificationId -> `Conflict` with zero second provider
  mutation.

## 7. Privacy routing and policy

- `DeliveryPolicy` gates FIRST: channel absent from the allowlist or
  below minimum urgency -> zero provider mutation (no best-effort
  bypass).
- `PrivacyRouting` (SENSITIVE-or-higher): SPEAKER / CAR are never
  used, even when a private channel is unavailable. Privacy over
  availability. CRITICAL urgency affects escalation priority only; it
  never authorizes a privacy-forbidden channel.
- Message body text is DATA, not authority: content cannot change
  urgency, privacy class, allowlist, escalation stage, or delivery
  state.

## 8. Escalation

- `EscalationPolicy` / `EscalatingNotificationRouter` chains are
  constructed with duplicates rejected (no SMS->SMS->SMS loops).
- FAILED -> exactly one attempt on the next permitted channel; no
  blind retry of the failed channel.
- PENDING / SENDING / UNKNOWN -> no escalation (delayed provider
  evidence is not failure).
- The router resolves destinations through a `DestinationResolver`
  bound at construction and performs the destination-aware SMS leg
  itself via `deliver_to` (AUD-019). Without a bound resolver the SMS
  leg FAILS CLOSED: the envelope carries no destination and one is
  never invented - the router records the fail-closed attempt without
  provider mutation.

## 9. Observability and redaction

- Bounded (256-entry) `NotificationObservability` ring records only
  safe fields: notification id, channel, provider ref, state,
  correlation, duration, escalation stage, error class, delivery
  report presence. Body, full destination, push private payload,
  credentials, and raw delivery-report PDUs are structurally
  impossible to record.
- Redaction is canary-tested: body/destination/credential canaries
  never leak into receipts, observability, errors, or gate evidence.

## 10. Recovery

- Provider restart: stop/start gammu-smsd; a fresh connector instance
  reconciles the exact durable identity and recovers with one row.
- Backend unavailable (DB replaced/refused/locked): canonical
  Unavailable/External fail closed, never a fake provider state;
  after restore, the next clean operation succeeds.

## 11. Cleanup

- `scripts/ep032-m5-tests.sh` and `scripts/ep032-m4-tests.sh`
  terminate the fixture daemon (TERM then KILL) and assert zero
  orphan gammu-smsd / AT-peer / socat processes.
- Evidence under `.agent/state/evidence/` is run_id-bound; stale
  files never satisfy a gate.

## 12. Certification boundaries (honest)

- Gammu SMSD **1.42.0** (package `1.42.0-8.1ubuntu2`, GPL-2.0,
  external daemon) is PROVIDER_CERTIFIED for the exact controlled
  fixture path (schema-17 SQLite + scripted PTY AT peer).
- AT+CMGS / SMS-SUBMIT and +CDS processing are CERTIFIED for the
  tested controlled path.
- Canonical Delivered is CERTIFIED only for the exact
  delivery-report path.
- PTY modem: CONTROLLED SIMULATION FIXTURE.
- **physical GSM modem: NOT ASSERTED**
- **carrier: NOT ASSERTED**
- **handset: NOT ASSERTED**
- arbitrary real-world SMS delivery: NOT ASSERTED.
- Real push provider (APNs/FCM/ntfy): NOT ASSERTED; push transport is
  TRANSPORT_CERTIFIED against controlled real sockets only.
