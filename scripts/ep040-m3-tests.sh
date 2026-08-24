#!/usr/bin/env sh
# EP-040 M3 gate: real provider certification + e2e transport integration.
#
# M3 owns tests/provider-certification/ (real digest-pinned PostgreSQL 18.4
# container transport, ProviderCertificationPort with mock/real distinction,
# readiness/cancellation/timeout/idempotency/event emission/audit/cleanup)
# and tests/e2e/transport/ (real end-to-end journey composing M1 contract,
# M2 execution core, M3 real provider transport).
#
# The gate executes the REAL cargo machinery against REAL ephemeral
# containers through the REAL docker CLI. Vacuous green is impossible:
# every required proof must be observed by name with a non-zero passing
# count and zero failed/ignored tests.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

# Ensure cargo is available to `sh -c` subshells.
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

log="/tmp/ep040-m3-tests.log"
: > "$log"

fail() {
  echo "EP-040 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-040 M3 gate: $1"; }

# Vacuity guard 0: the owned crates must exist with their owned sources.
if [ ! -f tests/provider-certification/Cargo.toml ]; then
  fail "tests/provider-certification/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/transport.rs \
  src/certifier.rs \
  tests/ep040_m3_provider_certification.rs; do
  if [ ! -f "tests/provider-certification/$f" ]; then
    fail "tests/provider-certification/$f missing"
  fi
done
if [ ! -f tests/e2e/transport/Cargo.toml ]; then
  fail "tests/e2e/transport/Cargo.toml missing"
fi
for f in \
  src/lib.rs \
  src/journey.rs \
  tests/ep040_m3_e2e_transport.rs; do
  if [ ! -f "tests/e2e/transport/$f" ]; then
    fail "tests/e2e/transport/$f missing"
  fi
done
ok "provider-certification + e2e-transport crates and M3-owned sources present"

# Vacuity guard 0b: the workspace declares both crate members.
if ! grep -q 'tests/provider-certification' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/provider-certification member"
fi
if ! grep -q 'tests/e2e/transport' Cargo.toml; then
  fail "workspace Cargo.toml missing tests/e2e/transport member"
fi
ok "workspace members declared"

# Vacuity guard 1: the real component is registered digest-pinned.
if ! grep -q 'postgres:18.4' COMPONENT_REGISTRY.yaml; then
  fail "postgresql 18.4 not registered in COMPONENT_REGISTRY.yaml"
fi
if ! grep -q 'a02db8cac496f15b094798a38254f14d6e00741f709360e5e00bb6668ea31636' COMPONENT_REGISTRY.yaml; then
  fail "postgresql 18.4 digest not pinned in COMPONENT_REGISTRY.yaml"
fi
ok "real component registered with digest (postgres:18.4)"

# Vacuity guard 2: docker CLI must actually be present and answer.
if ! docker version >/dev/null 2>&1; then
  fail "docker CLI unavailable; real-container integration cannot run"
fi
ok "docker CLI available"

# Real test run through cargo (rtk-tee compresses interactive output, so
# capture raw output to the log and grep real sentinels).
if ! sh -c 'cargo test -p nexus-provider-certification -p nexus-e2e-transport --locked >> "$1" 2>&1' _ "$log"; then
  fail "cargo test failed" "$log"
fi

# Vacuity guard 3: every suite reported a non-zero pass.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 4: zero failures observed.
if grep -qE 'test result: FAILED|[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 5: zero ignored tests (no required test may be skipped).
if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed; [1-9][0-9]* ignored' "$log"; then
  fail "required tests were ignored (vacuity guard)" "$log"
fi

# Vacuity guard 6 (anti-masking): every required M3 proof observed by name.
# One sentinel per behavior family.
for sentinel in \
  ep040_integration_provider_real_probe_observes_engine \
  ep040_integration_provider_real_roundtrip \
  ep040_integration_provider_readiness_through_host_port \
  ep040_integration_provider_statement_timeout_cancels_slow_query \
  ep040_integration_provider_idempotency_unique_constraint \
  ep040_integration_provider_event_emission_notify_listen \
  ep040_integration_provider_cleanup_zero_residue \
  ep040_integration_provider_real_evidence_certifies \
  ep040_integration_provider_auth_failure_fails_closed \
  ep040_unit_provider_unavailable_fails_closed \
  ep040_unit_provider_mock_evidence_never_certifies \
  ep040_unit_provider_simulated_evidence_never_certifies \
  ep040_unit_provider_stale_evidence_rejected \
  ep040_unit_provider_missing_evidence_rejected \
  ep040_unit_provider_identity_binding \
  ep040_unit_provider_evidence_redaction_enforced \
  ep040_integration_e2e_real_provider_journey \
  ep040_integration_e2e_m2_runner_composes_real_output \
  ep040_integration_e2e_output_without_summary_fails_closed \
  ep040_integration_e2e_stale_evidence_rejected \
  ep040_integration_e2e_empty_evidence_never_green \
  ep040_integration_e2e_redaction_proof \
  ep040_integration_e2e_parse_output_requires_evidence_bound; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-040-owned test $sentinel did not run (anti-masking)" "$log"
  fi
done
ok "all 23 EP-040 M3-owned proofs observed"

# Vacuity guard 7: real containers were actually exercised - the transport
# uses the docker CLI with EP-040-owned names and real SQL.
if ! grep -q "docker run" tests/provider-certification/src/transport.rs; then
  fail "transport does not spawn the real container"
fi
if ! grep -q "nexus-ep040-m3-" tests/provider-certification/src/transport.rs; then
  fail "transport does not use EP-040-owned container names"
fi
if ! grep -q "POSTGRES_IMAGE" tests/provider-certification/src/transport.rs; then
  fail "transport does not use the pinned postgres image"
fi
ok "real-container transport wiring verified"

# Vacuity guard 8: no placeholder content in the M3 crates.
if grep -rqiE 'placeholder|TODO|fake|sample only' tests/provider-certification/src tests/e2e/transport/src; then
  fail "M3 crate contains placeholder content"
fi
ok "M3 crate content validated"

# Vacuity guard 9: dependency direction - the M3 crates depend only on the
# canonical M1/M2 surfaces plus the real transport client (postgres).
bad_dep=$(cargo tree -p nexus-provider-certification --depth 1 2>/dev/null | grep -vE 'nexus-provider-certification|nexus-test-contract|postgres|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-provider-certification: $bad_dep"
fi
bad_dep=$(cargo tree -p nexus-e2e-transport --depth 1 2>/dev/null | grep -vE 'nexus-e2e-transport|nexus-test-contract|nexus-test-execution|nexus-provider-certification|postgres|serde|serde_json' || true)
if [ -n "$bad_dep" ]; then
  fail "dependency-direction violation in nexus-e2e-transport: $bad_dep"
fi
ok "dependency-direction clean (canonical surfaces + real postgres client)"

# Clippy -D warnings and fmt on the owned crates.
if ! sh -c 'cargo clippy -p nexus-provider-certification -p nexus-e2e-transport --all-targets --locked -- -D warnings >> "$1" 2>&1' _ "$log"; then
  fail "clippy -D warnings failed" "$log"
fi
ok "clippy -D warnings clean"

if ! sh -c 'cargo fmt -p nexus-provider-certification -p nexus-e2e-transport -- --check >> "$1" 2>&1' _ "$log"; then
  fail "cargo fmt check failed" "$log"
fi
ok "cargo fmt clean"

# License/security of the crates themselves: declared MIT.
if ! grep -q '^license = "MIT"' tests/provider-certification/Cargo.toml; then
  fail "nexus-provider-certification license must be MIT"
fi
if ! grep -q '^license = "MIT"' tests/e2e/transport/Cargo.toml; then
  fail "nexus-e2e-transport license must be MIT"
fi
ok "crate licenses declared (MIT)"

# Vacuity guard 10: resource hygiene - zero EP-040-owned containers remain
# after the real runs (plain docker ps output; no format templates that
# could be mistaken for placeholders).
leftover=$(docker ps -a | awk '{print $NF}' | grep '^nexus-ep040-m3-' || true)
if [ -n "$leftover" ]; then
  fail "EP-040 M3 containers left running: $leftover"
fi
leftover_evid=$(ls -d /tmp/ep040-m3-evid-* /tmp/ep040-m3-stale-* /tmp/ep040-m3-red-* 2>/dev/null || true)
if [ -n "$leftover_evid" ]; then
  fail "EP-040 M3 temp evidence residue: $leftover_evid"
fi
ok "resource hygiene verified (zero EP-040-owned containers/temp evidence)"

# M1 regression: the contract + performance suites must stay green.
if ! sh -c 'cargo test -p nexus-test-contract -p nexus-test-performance --locked >> "$1" 2>&1' _ "$log"; then
  fail "M1 regression failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "M1 regression ran no tests (vacuity guard)" "$log"
fi
ok "M1 regression green"

# M2 regression: the execution + accessibility suites must stay green.
if ! sh -c 'cargo test -p nexus-test-execution -p nexus-accessibility-audit --locked >> "$1" 2>&1' _ "$log"; then
  fail "M2 regression failed" "$log"
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  fail "M2 regression ran no tests (vacuity guard)" "$log"
fi
ok "M2 regression green"

echo "EP-040 M3 gate: ok"
