# EP-026 M4 Mail Fixture (CONTROLLED_TEST_FIXTURE)

Real local IMAP/SMTP server software for deterministic integration
tests. This fixture is CONTROLLED_TEST_FIXTURE: it never certifies
Gmail, Outlook, or any public provider.

## Provider record

- Software: GreenMail standalone 2.1.0 (mature SMTP/IMAP/POP3 server,
  Apache-2.0)
- Image: `greenmail/standalone:2.1.0`
- Digest: `sha256:308685b99ad840f05bd2dee43f47f7956f876adbf396523f68166f078300cd29`
- Enabled protocols: SMTP (3025), IMAP (3143) on dynamic host ports
- Auth method: SMTP AUTH PLAIN; IMAP LOGIN
- TLS mode: plaintext for protocol tests; TLS truthfulness proven
  against per-run self-signed certificates via `fixtures/tls_listener.py`
  (real TLS handshakes, never validation disabled)
- Fixture-only credentials: generated per run, stored in
  `/tmp/ep026-mail.env` (never committed)
- Account topology:
  - `tenant-a@nexus.test` (INBOX/Drafts/Sent/Trash)
  - `tenant-b@nexus.test` (INBOX/Drafts/Sent/Trash)
  - Tenant isolation is proven by the M4 suite: tenant A can never
    read/verify/mutate tenant B state.

## Fixture responders (controlled, real sockets)

- `fixtures/tcp_break_proxy.py` - relays a REAL client to the REAL
  GreenMail backend and breaks the TCP connection at a deterministic
  protocol phase (after DATA terminator -> SMTP ambiguous; after RCPT
  TO -> SMTP mid-session; after SELECT -> IMAP mid-session). The
  production client is never mocked.
- `fixtures/silent_listener.py` - accepts and holds silent (timeout
  classification: Timeout, not Unavailable).
- `fixtures/tls_listener.py` - real TLS peer with per-run self-signed
  certificate (fail-closed trust proof + custom-root positive proof).

## Usage

    sh infra/mail/provision.sh   # starts the stack, writes /tmp/ep026-mail.env
    sh infra/mail/teardown.sh    # idempotent teardown + state removal
