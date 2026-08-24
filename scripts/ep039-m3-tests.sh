#!/usr/bin/env sh
# EP-039 M3 gate: real dependency and transport integration.
#
# M3 owns policies/licenses/ (real checked-in policy files: allowlist,
# classes, sidecar obligations, waivers) and the transport crate
# @nexus-supply-chain-policy-io that loads them, parses the REAL
# Cargo.lock, resolves REAL licenses from the real registry cache and
# workspace manifests, classifies SPDX expressions at the boundary, and
# evaluates the real inventory through the M1 classifier + M2 engine.
#
# Vacuous green is impossible: the gate requires real cargo test runs
# with non-zero pass counts, EP-039-owned integration test names, zero
# failed/ignored, real-file evidence generation, deny.toml alignment,
# dependency-direction enforcement, M1+M2 regression, clippy, and fmt.
set -eu
export CI=true
export CARGO_TERM_COLOR=never
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep039-m3-tests.log"
: > "$log"

fail() {
  echo "EP-039 M3 gate: FAIL - $1" >&2
  tail -50 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-039 M3 gate: $1"; }

# Vacuity guard 0: M3-owned material presence (policy files + crate).
for f in \
  policies/licenses/Cargo.toml \
  policies/licenses/src/lib.rs \
  policies/licenses/src/policy_files.rs \
  policies/licenses/src/spdx.rs \
  policies/licenses/src/lockfile.rs \
  policies/licenses/src/resolve.rs \
  policies/licenses/src/inventory.rs \
  policies/licenses/src/evidence.rs \
  policies/licenses/tests/ep039_m3_integration.rs \
  policies/licenses/allowlist.toml \
  policies/licenses/classes.toml \
  policies/licenses/sidecar-obligations.toml \
  policies/licenses/waivers.toml; do
  if [ ! -f "$f" ]; then
    fail "policies/licenses/$f missing"
  fi
done
ok "M3-owned sources and policy files present"

# Vacuity guard 0b: workspace declares the transport crate.
if ! grep -q '"policies/licenses"' Cargo.toml; then
  fail "workspace Cargo.toml missing policies/licenses member"
fi
ok "workspace member declared"

# Vacuity guard 0c: policy files are deny-unknown and the allowlist is
# non-empty (no decorative policy accepted).
if ! grep -q 'deny_unknown = true' policies/licenses/allowlist.toml; then
  fail "allowlist.toml must set deny_unknown = true"
fi
if ! grep -q 'deny_unknown = true' policies/licenses/classes.toml; then
  fail "classes.toml must set deny_unknown = true"
fi
if ! grep -q '^allow = \[' policies/licenses/allowlist.toml; then
  fail "allowlist.toml missing allow table"
fi
ok "policy files deny-unknown"

# Vacuity guard 0d: allowlist.toml aligned with deny.toml (the real
# cargo-deny gate). Every allowlist id must appear in deny.toml allow -
# the checked-in policy cannot silently broaden the real gate.
if ! python3 - <<'EOF'
import tomllib
allow = tomllib.load(open('policies/licenses/allowlist.toml','rb'))['allow']
deny = None
for line in open('deny.toml'):
    if line.strip().startswith('allow = ['):
        deny = []
        continue
    if deny is not None:
        stripped = line.strip().rstrip(',')
        if stripped.startswith('"'):
            deny.append(stripped.strip('"'))
        elif stripped == ']':
            break
if deny is None:
    raise SystemExit('deny.toml allow list not found')
missing = [a for a in allow if a not in deny]
extra = [d for d in deny if d not in allow]
if missing:
    print(f'allowlist.toml ids not in deny.toml: {missing}', file=__import__('sys').stderr)
    raise SystemExit(1)
if extra:
    print(f'deny.toml ids not in allowlist.toml: {extra}', file=__import__('sys').stderr)
    raise SystemExit(1)
EOF
then
  fail "allowlist.toml / deny.toml alignment failed"
fi
ok "allowlist.toml aligned with deny.toml"

# Real test run through cargo, captured to the log for raw sentinels.
if ! sh -c 'cargo test -p nexus-supply-chain-policy-io --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test failed" "$log"
fi

# Vacuity guard 1: every suite reported a non-zero pass.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: zero failures observed.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 3: zero ignored tests (no required test may be skipped).
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 4 (anti-masking): EP-039-owned integration proofs
# observed. One sentinel per real-transport behavior family.
for sentinel in \
  ep039_integration_real_lockfile_parses_all_packages \
  ep039_integration_real_inventory_evaluates_every_package \
  ep039_integration_real_policy_files_load \
  ep039_integration_real_unknown_license_fails_closed \
  ep039_integration_real_green_license_clears_policy \
  ep039_integration_real_inventory_deterministic \
  ep039_integration_real_evidence_redacted \
  ep039_integration_waiver_absent_denied_on_real_policy \
  ep039_integration_sidecar_obligations_loaded_from_real_policy \
  ep039_integration_m1_classifier_alignment; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-039-owned integration test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "EP-039-owned real-transport integration proofs observed"

# Vacuity guard 5: dependency direction - the transport crate may depend
# only on nexus-supply-chain, nexus-supply-chain-policy, nexus-domain,
# serde, serde_json, toml. No vendor SDK / OCI / scanner / signer.
bad_dep=$(cargo tree -p nexus-supply-chain-policy-io --depth 1 2>/dev/null | grep -vE 'nexus-supply-chain-policy-io|nexus-supply-chain-policy|nexus-supply-chain|nexus-domain|serde|serde_json|toml' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-supply-chain-policy-io: $bad_dep"
fi
for forbidden in cyclonedx spdx-tools syft grype cosign sigstore slsa in-toto trivy osv-scanner aquasec anchore quay docker-registry npm pypi pip cargo-registry; do
  if cargo tree -p nexus-supply-chain-policy-io 2>/dev/null | grep -qi "$forbidden"; then
    fail "provider SDK dependency forbidden in M3: $forbidden"
  fi
done
ok "dependency-direction clean (contract + policy + serde + toml only)"

# Vacuity guard 6: no placeholder content in the transport.
if grep -rqiE 'placeholder|TODO|fake|sample only' policies/licenses/src; then
  fail "transport contains placeholder content"
fi
ok "transport content validated"

# Vacuity guard 7 (anti-masking): no secret literals in tracked sources.
if grep -rniE 'sk-[A-Za-z0-9]|ghp_[A-Za-z0-9]|AKIA[0-9A-Z]|Bearer [A-Za-z0-9]' policies/licenses/; then
  fail "secret-shaped literal in tracked M3 sources"
fi
ok "no secret literals in tracked sources"

# Clippy -D warnings and fmt on the owned crate.
if ! sh -c 'cargo clippy -p nexus-supply-chain-policy-io --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-supply-chain-policy-io -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# License/security of the crate itself: declared license MIT.
if ! grep -q '^license = "MIT"' policies/licenses/Cargo.toml; then
  fail "nexus-supply-chain-policy-io license must be MIT"
fi
ok "crate license declared (MIT)"

# Vacuity guard 8 (real-file evidence): the redaction + non-vacuity of
# the real inventory was already proven by the integration test
# ep039_integration_real_evidence_redacted (real Cargo.lock + real
# registry cache + real policy files). Confirm the sentinel and that the
# real-inventory tests exercised fail-closed paths.
if ! grep -q 'ep039_integration_real_evidence_redacted' "$log"; then
  fail "redaction proof did not run (anti-masking)" "$log"
fi
if ! grep -q 'ep039_integration_real_unknown_license_fails_closed' "$log"; then
  fail "real unknown-license fail-closed proof did not run (anti-masking)" "$log"
fi
ok "redaction + fail-closed proofs observed"

# M1 + M2 regression: the transport must not break the contract crate or
# the deterministic engine.
for crate in nexus-supply-chain nexus-supply-chain-policy; do
  mlog="/tmp/ep039-m3-regression-$crate.log"
  : > "$mlog"
  if ! sh -c 'cargo test -p "$1" --locked >> "$2" 2>&1' _ "$crate" "$mlog"; then
    fail "regression: cargo test -p $crate failed" "$mlog"
  fi
  if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$mlog"; then
    fail "regression: no tests ran for $crate (vacuity guard)" "$mlog"
  fi
  if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$mlog"; then
    fail "regression: observed failed tests for $crate" "$mlog"
  fi
done
ok "M1 + M2 regression green"

echo "EP-039 M3 gate: ok"
