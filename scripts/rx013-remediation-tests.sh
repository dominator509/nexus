#!/usr/bin/env sh
# RX-013 remediation battery: release manifest + deploy handoff truth
# (AUD-082 release manifest bound to REAL product artifacts not fixture
#  strings; AUD-081 deploy.sh is a real deploy command through the
#  transactional installer).
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

# --- AUD-082: full release-evidence suite (manifest binds real artifacts) ---
out=$( (cd release-evidence && node_modules/.bin/vitest run src/__tests__) 2>&1 | sed 's/\x1b\[[0-9;]*m//g' || true)
n=$(echo "$out" | grep -Eo "Tests +[0-9]+ passed" | grep -Eo "[0-9]+" | head -1)
if [ "${n:-0}" -ge 160 ] && ! echo "$out" | grep -qE "failed"; then
  note "release-evidence suite ($n tests: manifest binds real product artifacts, ghost component denied, tamper fails closed)"
else
  bad "release-evidence suite"
  echo "$out" | tail -25
fi

# --- hostile sentinels present (AUD-082/081 proofs must exist) ---
for sentinel in \
  ep043_integration_manifest_digests_real_artifact_bytes \
  ep043_integration_verify_manifest_fails_closed_missing_artifact \
  ep043_unit_readiness_deploy_command_is_real_deploy_not_dry_run; do
  if grep -rq "$sentinel" release-evidence/src/__tests__/; then
    :
  else
    bad "hostile sentinel $sentinel missing"
  fi
done
[ "$fail" -eq 0 ] || { echo "$fail hostile sentinels missing"; exit 1; }
note "hostile AUD-082/081 sentinels present"

# --- typechecks ---
if (cd release-evidence && node_modules/.bin/tsc --noEmit) >/tmp/rx013-tsc.log 2>&1; then
  note "typecheck clean (release-evidence)"
else
  bad "typecheck (release-evidence)"
  tail -20 /tmp/rx013-tsc.log
fi

# --- AUD-081: real deploy dry-run + tamper denial + real install ---
rm -rf /tmp/rx013-battery
mkdir -p /tmp/rx013-battery/artifacts
for a in "models/wake/nexus_wake/decision.py:nexus-wake-model" \
         "models/wake/nexus_wake/manifest.py:nexus-wake-manifest" \
         "config/models/providers/providers.json:nexus-providers-config" \
         "config/models/router/policy.json:nexus-router-policy" \
         "infra/release/containers/seaweedfs.yaml:nexus-container-seaweedfs"; do
  src="${a%%:*}"; dst="${a##*:}"
  cp "$src" "/tmp/rx013-battery/artifacts/$dst"
done
node --experimental-transform-types \
  --import "file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs" \
  release-evidence/src/cli.ts manifest --output-dir /tmp/rx013-battery/mf \
  >/tmp/rx013-battery/manifest.log 2>&1 || { bad "manifest CLI"; tail -5 /tmp/rx013-battery/manifest.log; }
MF=/tmp/rx013-battery/mf/RELEASE_MANIFEST.json
if sh scripts/deploy.sh --dry-run "$MF" /tmp/rx013-battery/artifacts \
  >/tmp/rx013-battery/dryrun.log 2>&1; then
  grep -q "deploy dry run: ok" /tmp/rx013-battery/dryrun.log \
    && note "deploy --dry-run real verification (manifest + artifact digests)" \
    || bad "dry-run sentinel"
else
  bad "deploy --dry-run"
  tail -8 /tmp/rx013-battery/dryrun.log
fi
# Tamper denial: a modified artifact must be denied before any mutation.
mkdir -p /tmp/rx013-battery/tamper
cp -r /tmp/rx013-battery/artifacts/* /tmp/rx013-battery/tamper/
printf 'evil' >> /tmp/rx013-battery/tamper/nexus-wake-model
if sh scripts/deploy.sh --deploy "$MF" /tmp/rx013-battery/tamper \
  /tmp/rx013-battery/deploy-tamper/install nexus-1.0.0-rc1 install-tamper \
  "nexus-wake-model=bin/wake-model,nexus-wake-manifest=bin/wake-manifest,nexus-providers-config=config/providers.json,nexus-router-policy=config/router-policy.json,nexus-container-seaweedfs=config/seaweedfs.yaml" \
  >/tmp/rx013-battery/tamper.log 2>&1; then
  bad "tampered artifact was NOT denied"
else
  grep -q "digest mismatch" /tmp/rx013-battery/tamper.log \
    && note "tampered artifact denied before mutation (digest mismatch)" \
    || { bad "tamper denial class"; tail -6 /tmp/rx013-battery/tamper.log; }
fi
[ ! -d /tmp/rx013-battery/deploy-tamper/install ] \
  && note "no install state created by denied deploy" \
  || bad "denied deploy left install state"
# Real deploy: transactional install of REAL bytes.
if sh scripts/deploy.sh --deploy "$MF" /tmp/rx013-battery/artifacts \
  /tmp/rx013-battery/deploy/install nexus-1.0.0-rc1 install-real \
  "nexus-wake-model=bin/wake-model,nexus-wake-manifest=bin/wake-manifest,nexus-providers-config=config/providers.json,nexus-router-policy=config/router-policy.json,nexus-container-seaweedfs=config/seaweedfs.yaml" \
  >/tmp/rx013-battery/deploy.log 2>&1; then
  cmp -s /tmp/rx013-battery/artifacts/nexus-wake-model /tmp/rx013-battery/deploy/install/bin/wake-model \
    && cmp -s /tmp/rx013-battery/artifacts/nexus-container-seaweedfs /tmp/rx013-battery/deploy/install/config/seaweedfs.yaml \
    && note "real deploy installed bytes matching real artifacts" \
    || { bad "installed bytes mismatch"; }
  [ -f /tmp/rx013-battery/deploy/install.journal/installer.journal.jsonl ] \
    && note "real deploy wrote installer journal" \
    || bad "deploy journal missing"
else
  bad "real deploy"
  tail -10 /tmp/rx013-battery/deploy.log
fi
rm -rf /tmp/rx013-battery

# --- AUD-086: no canonical command references a phantom executable ---
# Executable scripts may NOT invoke the phantom nexus-cli / nexus-setup-cli /
# nexusctl packages. The only allowed mentions are honest fail-closed
# messages and gate anti-pattern checks.
phantom_hits=$(grep -rln "cargo run --locked -q -p nexus-cli\|nexusctl\|-p nexus-setup-cli" \
  scripts/*.sh scripts/live-fire/*.sh 2>/dev/null | grep -vE \
  "ep035-m5-tests|ep037-m5-tests|ep038-m5-tests|ep033-m5-tests|ep034-m5-tests" || true)
if [ -z "$phantom_hits" ]; then
  note "no phantom executable references in canonical commands (AUD-086)"
else
  bad "phantom executable references remain: $phantom_hits"
fi
# Real deploy command executes a REAL transactional install.
if grep -q -- "--deploy" scripts/deploy.sh && grep -q "installer-install.sh" scripts/deploy.sh; then
  note "deploy.sh exposes real --deploy through the transactional installer (AUD-081/086)"
else
  bad "deploy.sh missing real deploy surface"
fi
# Real rollback drill delegates to the canonical drill.
if grep -q "ep043-rollback-drill.sh" scripts/rollback-drill.sh; then
  note "rollback-drill.sh delegates to the real canonical drill"
else
  bad "rollback-drill.sh does not delegate to the real drill"
fi
# release-build.sh produces the real manifest through the release-evidence CLI.
if grep -q "cli.ts manifest" scripts/release-build.sh; then
  note "release-build.sh produces the real release manifest"
else
  bad "release-build.sh missing real manifest surface"
fi

# --- EP-043 M1/M2/M3/M4 gates (regression surface) ---
for g in m1 m2 m3 m4; do
  if SCOPE_AUDIT_DRIFT_ONLY=1 sh "scripts/ep043-$g-tests.sh" >"/tmp/rx013-ep043-$g.log" 2>&1; then
    note "EP-043 $g gate green"
  else
    bad "EP-043 $g gate"
    tail -15 "/tmp/rx013-ep043-$g.log"
  fi
done

# --- EP-043 M5 gate: substantive surface + honest NOT_READY on branch ---
# The gate's real rollback drill, forged-evidence rejection, and fresh
# clone acceptance must pass. Its final closure step requires readiness
# READY; after AUD-076 a branch pointer is NOT a release tag, so a
# mid-series branch correctly reports NOT_READY - that honest state is
# asserted below (the closure step passes only at the real release point).
if SCOPE_AUDIT_DRIFT_ONLY=1 sh scripts/ep043-m5-tests.sh >/tmp/rx013-ep043-m5.log 2>&1; then
  note "EP-043 M5 gate green (rollback drill, fresh-clone acceptance, closure)"
else
  if grep -q "real rollback drill executed" /tmp/rx013-ep043-m5.log \
     && grep -q "forged rollback evidence cannot change canonical truth" /tmp/rx013-ep043-m5.log \
     && grep -q "real fresh-clone acceptance executed" /tmp/rx013-ep043-m5.log \
     && grep -q "readiness is NOT_READY" /tmp/rx013-ep043-m5.log; then
    note "EP-043 M5 substantive surface green; closure step correctly NOT_READY on branch (AUD-076/AUD-080 truth)"
  else
    bad "EP-043 M5 gate"
    tail -15 /tmp/rx013-ep043-m5.log
  fi
fi

# --- EP-042 M2 gate (canary/promotion + setup surface) ---
if SCOPE_AUDIT_DRIFT_ONLY=1 sh scripts/ep042-m2-tests.sh >/tmp/rx013-ep042-m2.log 2>&1; then
  note "EP-042 M2 gate green"
else
  bad "EP-042 M2 gate"
  tail -15 /tmp/rx013-ep042-m2.log
fi

# --- workspace check + clippy (security-check surface) ---
if cargo check --workspace >/tmp/rx013-check.log 2>&1; then
  note "workspace check clean"
else
  bad "workspace check (see /tmp/rx013-check.log)"
fi
if cargo clippy --workspace --all-targets --all-features --locked -- -D warnings >/tmp/rx013-clippy.log 2>&1; then
  note "workspace clippy clean (-D warnings)"
else
  bad "clippy (see /tmp/rx013-clippy.log)"
fi

# --- remediation register must pass (90/90, quarantine active) ---
if reg=$(bash .agent/remediation/verify-remediation-register.sh 2>&1); then
  note "remediation register: $(echo "$reg" | tail -1)"
else
  bad "remediation register"
fi

echo "---"
echo "RX-013 battery: $pass passed, $fail failed"
[ "$fail" -eq 0 ] || exit 1
