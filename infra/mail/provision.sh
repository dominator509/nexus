#!/usr/bin/env sh
# EP-026 M4 mail fixture: real GreenMail SMTP+IMAP server (pinned
# digest) + real TLS with a fixture-generated keystore.
# CONTROLLED_TEST_FIXTURE.
#
# Starts the GreenMail standalone container with two tenant accounts,
# AUTH ENABLED (the image default disables auth; auth-failure proofs
# require it), 0.0.0.0 bind, and a per-run self-signed TLS keystore
# with SAN=localhost injected into GreenMail's TLS endpoints. Writes
# per-run connection facts to /tmp/ep026-mail.env and registers the
# stack for teardown.
set -eu

GREENMAIL_IMAGE="greenmail/standalone:2.1.0@sha256:308685b99ad840f05bd2dee43f47f7956f876adbf396523f68166f078300cd29"
NAME="ep026-mail-$(date +%s)"
ENVFILE="/tmp/ep026-mail.env"
STATE="/tmp/ep026-mail-stack-state.json"
WORK="/tmp/ep026-mail-work"
mkdir -p "$WORK"

# Per-run fixture credentials (never committed; fixture-only).
A_PASS="m4-pass-a-$(head -c 6 /dev/urandom | od -An -tx1 | tr -d ' \n')"
B_PASS="m4-pass-b-$(head -c 6 /dev/urandom | od -An -tx1 | tr -d ' \n')"

# Per-run self-signed TLS identity with SAN=localhost so hostname
# validation is exercised properly (CN-only certs fail rustls, which
# does not CN-fallback). PKCS12 keystore for GreenMail.
CERT="$WORK/fixture-cert.pem"
KEY="$WORK/fixture-key.pem"
P12="$WORK/greenmail.p12"
openssl req -x509 -newkey rsa:2048 -keyout "$KEY" -out "$CERT" -days 1 -nodes \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost" \
  -addext "basicConstraints=critical,CA:FALSE" \
  -addext "keyUsage=critical,digitalSignature,keyEncipherment" >/dev/null 2>&1
openssl pkcs12 -export -in "$CERT" -inkey "$KEY" -out "$P12" \
  -passout pass:changeit -name greenmail >/dev/null 2>&1

# Hygiene: never leave a previous stack behind.
for OLD in $(docker ps -aq --filter "name=ep026-mail-" || true); do
  docker rm -f "$OLD" >/dev/null 2>&1 || true
done

# FIXED host ports (not 127.0.0.1:: ephemeral): docker restart
# re-randomizes ephemeral host bindings, so a restart test would
# lose its port and every later test would hit a dead fixture.
# Fixed ports survive restarts and keep /tmp/ep026-mail.env valid.
SMTP_PORT=39525
IMAP_PORT=39526
SMTPS_PORT=39527
IMAPS_PORT=39528
for P in "$SMTP_PORT" "$IMAP_PORT" "$SMTPS_PORT" "$IMAPS_PORT"; do
  if (echo > /dev/tcp/127.0.0.1/$P) 2>/dev/null; then
    echo "port $P already in use; refusing to start fixture" >&2
    exit 1
  fi
done

# Start GreenMail with two tenants, all protocols, AUTH ENFORCED,
# 0.0.0.0 bind, and the fixture keystore for the real TLS endpoints.
docker run -d --name "$NAME" \
  -p "127.0.0.1:$SMTP_PORT:3025" \
  -p "127.0.0.1:$IMAP_PORT:3143" \
  -p "127.0.0.1:$SMTPS_PORT:3465" \
  -p "127.0.0.1:$IMAPS_PORT:3993" \
  -v "$P12:/home/greenmail/greenmail.p12:ro" \
  -e "GREENMAIL_OPTS=-Dgreenmail.setup.test.all -Dgreenmail.hostname=0.0.0.0 -Dgreenmail.tls.keystore.file=/home/greenmail/greenmail.p12 -Dgreenmail.tls.keystore.password=changeit -Dgreenmail.users=tenant-a:$A_PASS@nexus.test,tenant-b:$B_PASS@nexus.test -Dgreenmail.verbose" \
  "$GREENMAIL_IMAGE" >/dev/null

# Real readiness: SMTP and IMAP ports must accept connections AND the
# TLS ports must complete a handshake against the fixture cert.
python3 - "$SMTP_PORT" "$IMAP_PORT" "$SMTPS_PORT" "$IMAPS_PORT" "$CERT" <<'PYEOF'
import socket, ssl, sys, time
smtp_port, imap_port, smtps_port, imaps_port = (int(x) for x in sys.argv[1:5])
cert = sys.argv[5]
deadline = time.time() + 60
for port in (smtp_port, imap_port):
    while True:
        try:
            s = socket.create_connection(("127.0.0.1", port), timeout=2)
            s.close()
            break
        except OSError:
            if time.time() > deadline:
                raise SystemExit(f"fixture port {port} never became ready")
            time.sleep(0.5)
ctx = ssl.create_default_context(cafile=cert)
for port in (smtps_port, imaps_port):
    while True:
        try:
            s = socket.create_connection(("localhost", port), timeout=2)
            tls = ctx.wrap_socket(s, server_hostname="localhost")
            tls.close()
            break
        except (OSError, ssl.SSLError):
            if time.time() > deadline:
                raise SystemExit(f"tls port {port} never completed handshake")
            time.sleep(0.5)
print("fixture ready")
PYEOF

# Provision the standard mailbox topology (directive E): GreenMail
# creates INBOX only; Drafts/Sent/Trash are created via real IMAP so
# the adapter's save_draft/send flows have their real folders.
python3 - "$IMAP_PORT" "tenant-a" "$A_PASS" "tenant-b" "$B_PASS" <<'PYEOF'
import imaplib, sys, time
imap_port = int(sys.argv[1])
accounts = [(sys.argv[2], sys.argv[3]), (sys.argv[4], sys.argv[5])]
deadline = time.time() + 60
for login, password in accounts:
    while True:
        try:
            imap = imaplib.IMAP4("127.0.0.1", imap_port)
            imap.login(login, password)
            break
        except Exception:
            if time.time() > deadline:
                raise SystemExit(f"tenant {login} login failed during provisioning")
            time.sleep(0.5)
    for folder in ("Drafts", "Sent", "Trash"):
        typ, _ = imap.create(folder)
        if typ != "OK":
            # Already exists is fine; anything else is a real failure.
            typ2, data2 = imap.select(folder)
            if typ2 != "OK":
                raise SystemExit(f"could not create folder {folder}: {data2}")
    imap.logout()
print("mailbox topology provisioned")
PYEOF

cat > "$ENVFILE" <<EOF
EP026_SMTP_HOST=127.0.0.1
EP026_SMTP_PORT=$SMTP_PORT
EP026_IMAP_HOST=127.0.0.1
EP026_IMAP_PORT=$IMAP_PORT
EP026_SMTPS_HOST=localhost
EP026_SMTPS_PORT=$SMTPS_PORT
EP026_IMAPS_HOST=localhost
EP026_IMAPS_PORT=$IMAPS_PORT
EP026_MAIL_ACCOUNT_A=tenant-a@nexus.test
EP026_MAIL_LOGIN_A=tenant-a
EP026_MAIL_PASS_A=$A_PASS
EP026_MAIL_ACCOUNT_B=tenant-b@nexus.test
EP026_MAIL_LOGIN_B=tenant-b
EP026_MAIL_PASS_B=$B_PASS
EP026_MAIL_TLS_CERT=$CERT
EP026_MAIL_STACK_NAME=$NAME
EOF

cat > "$STATE" <<EOF
{"name": "$NAME", "env": "$ENVFILE", "work": "$WORK"}
EOF

echo "ep026 mail fixture: ok"
echo "SMTP=$SMTP_PORT IMAP=$IMAP_PORT SMTPS=$SMTPS_PORT IMAPS=$IMAPS_PORT"
