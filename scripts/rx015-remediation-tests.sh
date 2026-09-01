#!/usr/bin/env sh
# RX-015 remediation battery: network defense truth
# (AUD-025 immutable approval receipt; AUD-026 quarantine binds observed
#  network identity; AUD-027 AdGuard configured blocklist; AUD-028 production
#  NetworkInventory; AUD-029 owner notification; AUD-030 Suricata profile;
#  AUD-031 preauthorization truth; AUD-032 same-indicator confidence;
#  AUD-033 verifier proposal binding; AUD-034 Zeek minute truth;
#  AUD-035 osquery durable endpoint identity; AUD-036 osquery REAL TLS;
#  AUD-037 osquery observation time + collision-proof event ids)
#
# The battery runs the REAL test suites that prove each milestone plus the
# workspace gates. LF-009 live-fire runs with a fresh run_id; LF-010 evidence
# run_ids are restored by the node-verify wrapper (battery clobber pattern).
set -eu
cd "$(dirname "$0")/.."
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

pass=0
fail=0
note() { echo "ok - $1"; pass=$((pass + 1)); }
bad() { echo "FAIL - $1"; fail=$((fail + 1)); }

run_suite() {
  # $1 = label, $2 = cargo package, $3 = expected minimum pass count
  out=$(cargo test -p "$2" 2>&1 || true)
  n=$(echo "$out" | grep -Eo "test result: ok\. [0-9]+ passed" | grep -Eo "[0-9]+" | awk '{s+=$1} END{print s+0}')
  if [ "${n:-0}" -ge "$3" ] && ! echo "$out" | grep -q "FAILED\|error\["; then
    note "$1 ($n passed)"
  else
    bad "$1"
    echo "$out" | tail -30
  fi
}

# --- M1: AUD-025 immutable approval receipt ---
run_suite "nexus-sentinel (AUD-025 approval receipt)" nexus-sentinel 5
run_suite "nexus-sentinel-advanced (AUD-025 approval-binds)" nexus-sentinel-advanced 20

# --- M2: AUD-026 observed network identity binding ---
run_suite "nexus-opnsense-connector (AUD-026 observed source)" nexus-opnsense-connector 24

# --- M3: AUD-029 owner notification ---
run_suite "nexus-openwrt (AUD-029 owner notification)" nexus-openwrt-connector 10

# --- M4: AUD-027 AdGuard configured blocklist ---
run_suite "nexus-adguard-connector (AUD-027 filtering status)" nexus-adguard-connector 23

# --- M5: AUD-028 production NetworkInventory ---
run_suite "nexus-opnsense-connector inventory (AUD-028 ARP)" nexus-opnsense-connector 24

# --- M6: AUD-030 Suricata EVE profile ---
run_suite "nexus-suricata-connector (AUD-030 EVE JSON)" nexus-suricata-connector 10

# --- M7: AUD-031 preauthorization truth ---
run_suite "nexus-sentinel-advanced (AUD-031 preauthorize)" nexus-sentinel-advanced 20

# --- M8: AUD-032 same-indicator confidence ---
run_suite "nexus-sentinel-advanced (AUD-032 confidence)" nexus-sentinel-advanced 20

# --- M9: AUD-033 verifier proposal binding ---
run_suite "nexus-sentinel-advanced (AUD-033 verifier)" nexus-sentinel-advanced 20

# --- M10: AUD-034 Zeek minute truth ---
run_suite "nexus-zeek-connector (AUD-034 minute truth)" nexus-zeek-connector 8

# --- M11/M12/M13: osquery identity + TLS + observation truth ---
run_suite "nexus-osquery-connector (AUD-035/036/037)" nexus-osquery-connector 31

# --- Live-fire journeys ---
out=$(EP031_M5_RUN_ID=m13-aud037 cargo test -p nexus-sentinel-advanced-live-fire 2>&1 || true)
n=$(echo "$out" | grep -Eo "test result: ok\. [0-9]+ passed" | grep -Eo "[0-9]+" | awk '{s+=$1} END{print s+0}')
if [ "${n:-0}" -ge 6 ] && ! echo "$out" | grep -q "FAILED\|error\["; then
  note "LF-009 quarantine journey (AUD-025..037, $n passed)"
else
  bad "LF-009 quarantine journey"
  echo "$out" | tail -30
fi

out=$(cargo test -p nexus-sentinel-live-fire 2>&1 || true)
n=$(echo "$out" | grep -Eo "test result: ok\. [0-9]+ passed" | grep -Eo "[0-9]+" | awk '{s+=$1} END{print s+0}')
if [ "${n:-0}" -ge 2 ] && ! echo "$out" | grep -q "FAILED\|error\["; then
  note "LF-010 network diagnosis (AUD-027/028, $n passed)"
else
  bad "LF-010 network diagnosis"
  echo "$out" | tail -30
fi

# --- Workspace gates ---
if cargo clippy --workspace --all-targets -- -D warnings >/tmp/rx015-clippy.log 2>&1; then
  note "clippy -D warnings"
else
  bad "clippy -D warnings"
  tail -20 /tmp/rx015-clippy.log
fi
if cargo fmt --all -- --check >/tmp/rx015-fmt.log 2>&1; then
  note "fmt --check"
else
  bad "fmt --check"
  tail -10 /tmp/rx015-fmt.log
fi
if cargo check --workspace --all-targets >/tmp/rx015-ws.log 2>&1; then
  note "workspace check"
else
  bad "workspace check"
  tail -20 /tmp/rx015-ws.log
fi

echo
echo "RX-015 battery: $pass ok, $fail fail"
[ "$fail" -eq 0 ]
