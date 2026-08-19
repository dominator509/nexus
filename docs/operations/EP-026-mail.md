# EP-026 Mail Operations

Operations and diagnostics for the EP-026 mail plane: universal
mailboxes, Gmail, Microsoft Graph, IMAP/SMTP, attachments, drafts,
sends, and audit. Every command below was exercised by the owning
milestones (M1-M5); commands are listed exactly as run.

## Provider configuration

Connectors live behind the `EmailProvider` port in `crates/nexus-email`
(SPEC-014). Each connector owns its transport and credential surface:

- Gmail: `connectors/gmail` (nexus-gmail) - Gmail REST transport;
  OAuth bearer for the Authorization header only. Separate
  ReadOnly/Send/Full scopes. Credentials are never logged.
- Microsoft Graph: `connectors/microsoft-mail` (nexus-microsoft-mail) -
  Graph v1.0 mail REST transport; FOUR separate authorities
  (ReadOnly=Mail.Read, ReadWrite=Mail.ReadWrite, Send=Mail.Send, Full).
- IMAP/SMTP: `connectors/imap-smtp` (nexus-imap-smtp) - real IMAP read
  - SMTP submission (imap 3.0.0-alpha.15, lettre 0.11.23). IMAP and
    SMTP are SEPARATE authorities: SMTP credentials cannot read, IMAP
    credentials cannot send.

Configuration is fixture/tenant scoped through environment files
(e.g. `infra/mail/provision.sh` writes `/tmp/ep026-mail.env`). Real
external providers (Gmail, Microsoft Graph) require credentials
provisioned by the deployment/ship owner; their live certification is
NOT ASSERTED in this node (certification debt recorded in
`.agent/state/evidence/CERTIFICATION_REGISTRY.md`).

## Fixture lifecycle (controlled test fixture)

    sh infra/mail/provision.sh    # start GreenMail 2.1.0 (pinned digest)
    sh infra/mail/teardown.sh     # idempotent teardown + state removal

Provision writes per-run connection facts to `/tmp/ep026-mail.env`
(host, ports, accounts, per-run credentials, TLS cert). Teardown
removes every `ep026-mail-*` container. FIXED host ports 39525-39528
survive `docker restart` (ephemeral bindings are re-randomized by
Docker and break restart tests).

## Gmail diagnostics

- 401/403 -> `Authorization` (check token validity/scope)
- 404 -> `NotFound` (unknown mailbox/message; never served)
- 429 -> `RateLimit`
- 500/502/503 -> `Unavailable`
- silent peer -> `Timeout`; connection refused -> `Unavailable`
- malformed JSON -> `External` (fail closed)

## Microsoft Graph diagnostics

- sendMail / draft-send / reply / forward return **202 Accepted with
  NO body** - status-only helper; JSON is never parsed from a 202.
  Submission is SENT, never DELIVERED.
- update = 200 + structured message (used for exact-target
  verification); delete = 204 + empty body.
- reply uses the comment-only shape (comment OR message.body are
  mutually exclusive). forward puts toRecipients TOP-LEVEL.
- 401/403/404/429/5xx mapping as Gmail above.

## IMAP/SMTP diagnostics

- SMTP phase-exact transaction tracking: AUTH -> MAIL FROM -> RCPT TO
  -> DATA -> message. `SmtpOutcome::Accepted(mid)` = SENT (250);
  `SmtpOutcome::Ambiguous` = the provider MAY have accepted before the
  connection was lost.
- **Ambiguous outcome**: NEVER blind-retry. The idempotency ledger
  records Ambiguous; a replay with the same key is REFUSED until
  reconciliation. Verify recipient-side state first (real IMAP
  readback), then decide.
- IMAP readback binds delivery: the recipient INBOX holding the exact
  runtime canary message id is the recipient-side evidence. A message
  in the sender Sent folder is never proof of recipient delivery.
- GreenMail quirks (recorded): folders Drafts/Sent/Trash are in-memory
  and lost on restart (re-provision through real IMAP CREATE); users
  survive restart; one-line AUTH PLAIN is rejected (two-step 334
  challenge works); bundled keystore has no SAN (use the per-run
  CN=localhost + SAN keystore).

## Send-state interpretation

State ladder (canonical): DRAFT < QUEUED < SENDING < SENT < DELIVERED,
plus terminal Failed/Archived/Deleted. `SENT != DELIVERED`:

- SMTP 250 / Graph 202 / Gmail send success = SUBMISSION (SENT)
- Sent folder presence = NOT delivery
- provider queue state = NOT delivery
- independent recipient-side readback (message arrives in the intended
  recipient mailbox with the exact runtime canary) supports DELIVERED.

## Reconciliation

For an ambiguous SMTP send (connection lost after DATA):

1. Search the recipient mailbox by message id / runtime canary
   (real IMAP readback).
2. If found exactly once -> reconcile to Confirmed (SENT/DELIVERED at
   the observed boundary); do NOT resend.
3. If not found -> the provider never accepted; a retry is a fresh
   attempt.
4. Never resend blind; a duplicate email has real consequences.

## Auth failure

Wrong/revoked credentials map to `Authorization` with zero provider
mutation, zero SENT state, and no ledger success entry. Real provider
rejection is never mapped to fabricated success. Tokens are held ONLY
for the Authorization header and never appear in errors, audit, or
evidence.

## Token refresh / recovery

OAuth refresh is provider-owned (Gmail/Graph deployment integration).
The IMAP/SMTP connector authenticates per connection with the
configured account credentials; after a provider restart, reconnect
and re-authenticate before the next operation (restart/recovery proof
in `lf011`/`m4_restart_recovery`). A stale session never fabricates
health: readiness requires a real socket + successful auth.

## Mailbox exact-target lookup

`MailVerifier` requires the SAME message id reaching the SAME expected
state. An unrelated message NEVER verifies the target (UnrelatedChange
-> Verification error). Bind lookups with: exact recipient, provider
message id, unique runtime subject, unique body canary, sender,
arrival window.

## Attachment troubleshooting

- Every attachment passes `MailPolicy.attachment_allows` (size bound +
  ScanStatus CLEAN) BEFORE any provider mutation. Pending/Blocked/
  Quarantined attachments reject with zero provider calls.
- Attachments carry a sha256 digest; content is never stored raw in
  domain/telemetry. Materialization through ArtifactStore is a later
  node concern (digest-only until then).
- A denied/unscanned attachment generates ZERO external provider
  mutation (proven in `lf011_attachment_gate_no_mutation`).

## TLS troubleshooting

- TLS validation is NEVER disabled. Fail-closed trust: invalid cert,
  hostname mismatch, or unknown trust root -> error, no connection.
- Custom root CA: the fixture generates a per-run keystore
  (CN=localhost + SAN, end-entity). openssl `req -x509` marks certs as
  CAs (rustls rejects `CaUsedAsEndEntity`); generate end-entity certs.
- GreenMail's bundled `greenmail.p12` has no SAN; rustls does not
  CN-fallback. Use the fixture-generated keystore.

## Redacted logging

- Audit ring: bounded (256 entries), redacts secrets and body-shaped
  canaries at insert; `mail-<nanos>-<seq>` correlation on every event.
- Fixture credentials are generated per run and never committed,
  logged, or written to evidence. Redaction canary proofs:
  `m4_redaction_canary_no_leak`, `lf011_redaction_evidence_no_leak`.

## Shutdown / cleanup

    sh infra/mail/teardown.sh

Zero-orphan audit (after teardown): no `ep026-mail-*` containers, no
responder children (tcp_break_proxy.py / silent_listener.py), no
leaked test processes. Gates run this after every suite.

## Known certification boundaries

- Canonical EmailProvider: INTERNAL_CERTIFIED (M1-M4)
- Gmail connector: IMPLEMENTED; real Gmail provider NOT ASSERTED
  (external certification deferred to deployment/ship owner)
- Microsoft Graph connector: IMPLEMENTED / TRANSPORT_CERTIFIED against
  controlled Graph-shaped fixtures; real tenant NOT ASSERTED
  (deferred M5+/deployment owner)
- IMAP/SMTP connector: IMPLEMENTED / PROTOCOL_CERTIFIED /
  TRANSPORT_CERTIFIED against the controlled GreenMail server
- GreenMail: CONTROLLED_TEST_FIXTURE
- recipient delivery: recipient-side readback within the controlled
  provider boundary (LF-011); arbitrary Internet delivery NOT ASSERTED
- hostile-content authority isolation: CERTIFIED (content is data,
  never authority)
- external/public provider certification: NOT ASSERTED - certification
  debt with deployment/ship owner (SPEC-008 pattern), never claimed
  from the controlled fixture
