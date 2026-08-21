#!/usr/bin/env sh
# EP-035 M4 operations diagnostic + bounded recovery for the onboarding
# providers (PostgreSQL 18.4 + NATS 2.14.3; SPEC-004/SPEC-016; M4 CHANGE
# tests/onboarding/ops/).
#
#   sh tests/onboarding/ops/onboarding-diag.sh diagnose [pg_port] [nats_port]
#   sh tests/onboarding/ops/onboarding-diag.sh recover   [pg_port] [nats_port]
#
# `diagnose` probes both provider boundaries through the published host
# port and prints structured, redacted state (never credentials).
# `recover` is bounded: it only reports the health transition and the
# exact restart command - it never fabricates a session or mints
# credentials. On an unreachable provider the diagnostic fails closed
# with a non-zero exit (3), matching the EP-028/EP-027 diag convention.
set -eu

MODE="${1:-diagnose}"
PG_PORT="${2:-5432}"
NATS_PORT="${3:-4222}"
PG_USER="${NEXUS_ONBOARDING_PG_USER:-nexus}"
PG_DB="${NEXUS_ONBOARDING_PG_DB:-nexus}"

redact() {
  # Never print credentials or secret material.
  sed -E 's#(password|secret|token|authorization)[=: ]+[A-Za-z0-9._~+/=-]+#\1 ***#Ig'
}

DEFAULT_PW="nexus-test"

pg_ready() {
  PG_PW="${NEXUS_ONBOARDING_PG_PASSWORD:-$DEFAULT_PW}"
  PGPASSWORD="$PG_PW" psql \
    -h 127.0.0.1 -p "$PG_PORT" -U "$PG_USER" -d "$PG_DB" \
    -Atc 'SELECT 1' >/dev/null 2>&1
}

nats_ready() {
  # PING/PONG through the published host port; read >= 4096B so the
  # INFO banner cannot swallow the PONG (M3 NATS framing lesson).
  bash -c "exec 3<>/dev/tcp/127.0.0.1/$NATS_PORT && printf 'PING\r\n' >&3 && timeout 3 dd bs=4096 count=1 <&3 2>/dev/null | grep -q PONG" >/dev/null 2>&1
}

case "$MODE" in
  diagnose)
    echo "onboarding diag: pg_port=$PG_PORT nats_port=$NATS_PORT"
    if pg_ready; then
      echo "onboarding diag: postgres reachable=yes"
    else
      echo "onboarding diag: postgres reachable=no (fail closed)"
      exit 3
    fi
    if nats_ready; then
      echo "onboarding diag: nats reachable=yes"
    else
      echo "onboarding diag: nats reachable=no (fail closed)"
      exit 3
    fi
    echo "onboarding diag: providers healthy"
    ;;
  recover)
    echo "onboarding diag: bounded recovery report"
    if pg_ready && nats_ready; then
      echo "onboarding diag: both providers healthy - no action"
      exit 0
    fi
    if ! pg_ready; then
      echo "onboarding diag: postgres unhealthy - restart the digest-pinned container (docker run --rm -d --name nexus-ep035-pg -e POSTGRES_USER=$PG_USER -p 127.0.0.1::5432 postgres:18.4@sha256:a02db8cac496f15b094798a38254f14d6e00741f709360e5e00bb6668ea31636)"
    fi
    if ! nats_ready; then
      echo "onboarding diag: nats unhealthy - restart the digest-pinned container (docker run --rm -d --name nexus-ep035-nats -p 127.0.0.1::4222 nats:2.14.3@sha256:67ac7866d010e8d83302dd30332eeae1a2b7a8ee051155e2eb5a5485b720cd4b -js)"
    fi
    echo "onboarding diag: recovery requires operator restart (bounded; never fabricated)"
    exit 4
    ;;
  *)
    echo "onboarding diag: unknown mode $MODE (diagnose|recover)" >&2
    exit 2
    ;;
esac
