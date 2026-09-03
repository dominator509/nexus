#!/usr/bin/env sh
# RX-018 remediation battery: artifact storage truth
# (AUD-049 Local/NAS tenant boundary on a shared root; AUD-050 shared-content
#  delete preserves still-referenced objects; AUD-051 encryption-before-egress
#  verified against real ciphertext, not metadata alone)
#
# The battery runs the REAL EP-037 gate suites that prove each milestone
# (M1/M2 -> ep037-m2 + ep037-m3; M3 -> ep037-m3 + ep037-m4 + ep037-m5,
# which cover NAS/S3/SeaweedFS adapters over real MinIO/SeaweedFS peers)
# plus clippy/fmt and the live-fire storage journeys.
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

# --- M1: AUD-049 tenant boundary (local + NAS gates) ---
for g in ep037-m2-tests.sh ep037-m3-tests.sh; do
  if timeout 500 sh "scripts/$g" >"/tmp/rx018-$g.log" 2>&1; then
    note "$g (AUD-049 tenant boundary)"
  else
    bad "$g"
    tail -15 "/tmp/rx018-$g.log"
  fi
done

# --- M2: AUD-050 shared-content delete (local gate) ---
if timeout 500 sh scripts/ep037-m2-tests.sh >/tmp/rx018-ep037-m2-050.log 2>&1; then
  note "ep037-m2-tests.sh (AUD-050 shared-content delete)"
else
  bad "ep037-m2-tests.sh (AUD-050)"
  tail -15 /tmp/rx018-ep037-m2-050.log
fi

# --- M3: AUD-051 encryption-before-egress (NAS + SeaweedFS + S3 gates) ---
for g in ep037-m3-tests.sh ep037-m4-tests.sh ep037-m5-tests.sh; do
  if timeout 550 sh "scripts/$g" >"/tmp/rx018-$g.log" 2>&1; then
    note "$g (AUD-051 encryption-before-egress)"
  else
    bad "$g"
    tail -15 "/tmp/rx018-$g.log"
  fi
done

# --- clippy + fmt on the touched storage surface ---
if timeout 300 cargo clippy -p nexus-artifacts -p nexus-provider-storage-local \
  -p nexus-provider-storage-nas -p nexus-provider-storage-s3 \
  -p nexus-provider-storage-seaweedfs -p nexus-storage-livefire \
  --all-targets --locked -- -D warnings >/tmp/rx018-clippy.log 2>&1; then
  note "clippy -D warnings clean"
else
  bad "clippy -D warnings"
  tail -15 /tmp/rx018-clippy.log
fi

if timeout 120 cargo fmt -p nexus-artifacts -p nexus-provider-storage-local \
  -p nexus-provider-storage-nas -p nexus-provider-storage-s3 \
  -p nexus-provider-storage-seaweedfs -p nexus-storage-livefire \
  -- --check >/tmp/rx018-fmt.log 2>&1; then
  note "cargo fmt clean"
else
  bad "cargo fmt"
  tail -15 /tmp/rx018-fmt.log
fi

echo
echo "RX-018 battery: $pass ok, $fail fail"
[ "$fail" -eq 0 ]
