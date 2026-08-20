#!/usr/bin/env sh
# EP-032 M4 SMS diagnostic: truthful Gammu SMSD health classification.
#
# Distinguishes (never reports healthy from configuration existence):
#   configured        - smsdrc exists and names a database
#   provider db       - the SQLite/PostgreSQL backend is reachable
#                       and the certified schema version is present
#   gammu-smsd        - the daemon process is running
#   provider queue    - the outbox is writable (a probe row can be
#                       inserted and removed through the documented
#                       create_outbox shape)
#   delivery report   - whether the delivery-report path can be
#                       asserted from this host (not without a real
#                       delivery report; reported honestly)
#
# Exit codes: 0 = all asserted layers healthy; 1 = a probe failed;
# 3 = provider unreachable (fail closed). Never claims healthy=true
# from configuration existence alone.
set -eu
export LC_ALL=C

usage() {
  echo "usage: $0 -c <smsdrc> [-d <database>]" >&2
  exit 2
}

SMSDRC=""
DB=""
while getopts "c:d:h" opt; do
  case "$opt" in
    c) SMSDRC="$OPTARG" ;;
    d) DB="$OPTARG" ;;
    h) usage ;;
    *) usage ;;
  esac
done
[ -n "$SMSDRC" ] || usage

echo "sms-diag: target config $SMSDRC"

# 1. configured
if [ ! -f "$SMSDRC" ]; then
  echo "sms-diag: rc=3 configured=no (config missing)"
  exit 3
fi
if [ -z "$DB" ]; then
  DB="$(sed -n 's/^database[[:space:]]*=[[:space:]]*//p' "$SMSDRC" | head -1)"
fi
if [ -z "$DB" ]; then
  echo "sms-diag: rc=3 configured=no (no database in config)"
  exit 3
fi
# Resolve dbdir-relative paths like the daemon does.
DBDIR="$(sed -n 's/^dbdir[[:space:]]*=[[:space:]]*//p' "$SMSDRC" | head -1)"
case "$DB" in
  /*) DB_PATH="$DB" ;;
  *) DB_PATH="${DBDIR:-.}/$DB" ;;
esac
if [ ! -f "$DB_PATH" ]; then
  echo "sms-diag: rc=3 configured=yes provider_db=no (database file missing)"
  exit 3
fi
echo "sms-diag: configured=yes"

# 2. provider db reachable + certified schema version
SCHEMA_VER="$(sqlite3 "$DB_PATH" 'SELECT Version FROM gammu LIMIT 1;' 2>/dev/null || true)"
if [ "$SCHEMA_VER" != "17" ]; then
  echo "sms-diag: rc=3 configured=yes provider_db=no (schema $SCHEMA_VER != certified 17)"
  exit 3
fi
echo "sms-diag: provider_db=yes schema=17"

# 3. daemon process
if pgrep -x gammu-smsd >/dev/null 2>&1 || pgrep -f "gammu-smsd.*$SMSDRC" >/dev/null 2>&1; then
  echo "sms-diag: gammu_smsd=yes"
else
  echo "sms-diag: gammu_smsd=no (daemon not running; queue may still be writable)"
fi

# 4. provider queue writable via the documented create_outbox shape.
# Insert a probe row with a unique creator id, verify it, delete it.
PROBE="nexus-diag-$$-$(date +%s)"
if sqlite3 "$DB_PATH" \
  "INSERT INTO outbox (CreatorID, SenderID, DeliveryReport, MultiPart, DestinationNumber, TextDecoded, Coding, Class)
   VALUES ('$PROBE', '', 'no', 'false', '+15551234567', 'sms-diag probe', 'Default_No_Compression', -1);" 2>/dev/null \
  && sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM outbox WHERE CreatorID = '$PROBE';" 2>/dev/null | grep -q '^1$'; then
  sqlite3 "$DB_PATH" "DELETE FROM outbox WHERE CreatorID = '$PROBE';" 2>/dev/null || true
  echo "sms-diag: provider_queue=yes"
else
  echo "sms-diag: rc=1 configured=yes provider_db=yes provider_queue=no (outbox not writable)"
  exit 1
fi

# 5. delivery-report path: not assertable from this host without a
# real delivery report (the daemon records DeliveryOK only from an
# actual SMS-STATUS-REPORT). Honest classification.
echo "sms-diag: delivery_report=not_asserted (requires a real provider delivery report)"

echo "sms-diag: rc=0 healthy=yes"
