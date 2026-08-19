#!/usr/bin/env sh
# EP-028 M4 operations diagnostic + bounded recovery for the Hydra
# connector (SPEC-015).
#
#   sh tests/hydra/ops/hydra-diag.sh diagnose [base_url]
#   sh tests/hydra/ops/hydra-diag.sh recover   [base_url]
#
# The Hydra connector is an authenticated REST boundary over the
# versioned canonical surface. `diagnose` probes readiness and returns
# structured, redacted state (it never prints credentials). `recover`
# is bounded: it only reports the health transition and requires the
# caller to restart/point at the provider - it never fabricates a
# session or mints credentials.
set -eu
BASE="${2:-${HYDRA_BASE_URL:-http://127.0.0.1:8443}}"
DIAG_TIMEOUT="${HYDRA_DIAG_TIMEOUT:-5}"

redact() {
  # Never print bearer credentials or secret material.
  sed -E 's#(Bearer|Authorization)[=: ]+[A-Za-z0-9._~+/=-]+#\1 ***#g'
}

probe() {
  # The pipeline exit status must be curl's, not head's: a refused or
  # unreachable endpoint must fail closed (non-zero), never report
  # "reachable" with an empty body.
  out="$(curl -fsS --max-time "$DIAG_TIMEOUT" "$BASE/v1/capabilities" 2>/dev/null)" || return 1
  printf '%s' "$out" | head -c 400
}

case "${1:-diagnose}" in
  diagnose)
    echo "hydra diag: base=$BASE"
    if capabilities="$(probe)"; then
      echo "hydra diag: reachable=yes"
      echo "hydra diag: capabilities=$(printf '%s' "$capabilities" | redact)"
    else
      echo "hydra diag: reachable=no"
      echo "hydra diag: provider unavailable or unreachable (fail closed)"
      exit 3
    fi
    ;;
  recover)
    echo "hydra diag: recover (bounded)"
    if capabilities="$(probe)"; then
      echo "hydra diag: provider healthy - no recovery action needed"
      echo "hydra diag: capabilities=$(printf '%s' "$capabilities" | redact)"
    else
      echo "hydra diag: provider unavailable - restart/point the provider"
      echo "hydra diag: recovery is bounded: the adapter never fabricates a session"
      exit 3
    fi
    ;;
  *)
    echo "hydra diag: FAIL - unknown verb ${1:-}" >&2
    exit 2
    ;;
esac
echo "hydra diag: ok"
