#!/usr/bin/env sh
# RX-016 remediation battery: desktop/mobile client truth
# (AUD-038 actual React PWA entry; AUD-039 desktop high-risk authorization
#  resolves approval class from the registered capability profile, never the
#  wire; AUD-040 real native mobile security channel Android/iOS;
#  AUD-041 step-up enforcement + identity/tenant binding VERIFIED_FIXED)
#
# The battery runs the REAL test suites that prove each milestone plus the
# workspace gates. LF-005 evidence is refreshed by the ep033 M5 gate.
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

# --- M1: AUD-039 registered capability profile gate ---
out=$(pnpm --filter @nexus/desktop test:unit 2>&1 || true)
if echo "$out" | grep -q "Tests.*passed" && ! echo "$out" | grep -q "failed"; then
  note "desktop dispatcher (AUD-039 registered profile gate)"
else
  bad "desktop dispatcher (AUD-039)"
  echo "$out" | tail -20
fi

out=$(pnpm --filter @nexus/web test:unit 2>&1 || true)
if echo "$out" | grep -q "Tests.*passed" && ! echo "$out" | grep -q "failed"; then
  note "web contracts + PWA entry (AUD-038/039)"
else
  bad "web contracts + PWA entry"
  echo "$out" | tail -20
fi

out=$(pnpm --filter @nexus/web-e2e test:unit 2>&1 || true)
if echo "$out" | grep -q "Tests.*passed" && ! echo "$out" | grep -q "failed"; then
  note "web e2e forced-failure (AUD-039)"
else
  bad "web e2e forced-failure"
  echo "$out" | tail -20
fi

# --- M2/M3/M4: mobile contracts + native channel ---
if timeout 400 sh scripts/ep034-m1-tests.sh >/tmp/rx016-ep034-m1.log 2>&1; then
  note "ep034 M1 gate (mobile contract suite)"
else
  bad "ep034 M1 gate"
  tail -15 /tmp/rx016-ep034-m1.log
fi
if timeout 400 sh scripts/ep034-m2-tests.sh >/tmp/rx016-ep034-m2.log 2>&1; then
  note "ep034 M2 gate (AUD-041 approval binding)"
else
  bad "ep034 M2 gate"
  tail -15 /tmp/rx016-ep034-m2.log
fi
if timeout 400 sh scripts/ep034-m3-tests.sh >/tmp/rx016-ep034-m3.log 2>&1; then
  note "ep034 M3 gate (e2e transport)"
else
  bad "ep034 M3 gate"
  tail -15 /tmp/rx016-ep034-m3.log
fi
if timeout 400 sh scripts/ep034-m4-tests.sh >/tmp/rx016-ep034-m4.log 2>&1; then
  note "ep034 M4 gate (failure suite)"
else
  bad "ep034 M4 gate"
  tail -15 /tmp/rx016-ep034-m4.log
fi
if timeout 400 sh scripts/ep034-m5-tests.sh >/tmp/rx016-ep034-m5.log 2>&1; then
  note "ep034 M5 gate (live-fire LF-004 + LF-022)"
else
  bad "ep034 M5 gate"
  tail -15 /tmp/rx016-ep034-m5.log
fi

# --- M4: EP-033 M5 gate (PWA + a11y + LF-005) ---
if timeout 400 sh scripts/ep033-m5-tests.sh >/tmp/rx016-ep033-m5.log 2>&1; then
  note "ep033 M5 gate (PWA + a11y axe + LF-005)"
else
  bad "ep033 M5 gate"
  tail -15 /tmp/rx016-ep033-m5.log
fi

echo
echo "RX-016 battery: $pass ok, $fail fail"
[ "$fail" -eq 0 ]
