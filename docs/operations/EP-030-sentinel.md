# EP-030 Sentinel Core (Network and DNS) - Operations

Owned components: `crates/nexus-sentinel` (contract), `connectors/opnsense`
(OPNsense firewall adapter), `connectors/openwrt` (OpenWrt ubus adapter),
`connectors/adguard-home` (AdGuard Home DNS adapter), `infra/sentinel/core`
(live-fire evidence), `tests/sentinel/core` (contract composition).

## Health

- Contract: `cargo test -p nexus-sentinel`
- OPNsense adapter: `cargo test -p nexus-opnsense-connector`
- OpenWrt adapter: `cargo test -p nexus-openwrt-connector`
- AdGuard adapter: `cargo test -p nexus-adguard-connector`
- Live-fire: `cargo test -p nexus-sentinel-live-fire`

## Diagnostics

### OPNsense diagnostic status

`OpnsenseFirewallProvider::capabilities()` advertises
`ReadFirewallTelemetry`/`Containment`/`ProposeQuarantine` ONLY when the
documented searchRule transport answers (reality rule). An unreachable or
unbound transport advertises nothing and `read_telemetry` returns
`Unavailable` - never a fabricated healthy firewall.

### OpenWrt diagnostic status

`OpenWrtFirewallProvider::capabilities()` performs the documented ubus
`session/login`; capability advertisement requires the login to answer. A
refused port is `Unavailable`; ubus status 6 (PERMISSION_DENIED) is
`Authorization`; a silent kept-open peer is `Timeout`.

### AdGuard Home diagnostic status

`AdGuardDnsSecurityProvider::capabilities()` advertises
`ReadDnsTelemetry`/`ReadDnsBlocklist` ONLY when `GET /control/status`
answers. Telemetry is OBSERVED data: totals/blocked are counted from the
documented query log. An empty log is an empty window, never a fabricated
baseline.

## Connectivity failure classification (SPEC-006)

- refused socket -> `Unavailable`
- silent accepted socket -> `Timeout`
- malformed provider response -> `External` (fail closed)
- HTTP 401/403 -> `Authorization`
- HTTP 404 -> `NotFound`
- HTTP 5xx -> `Unavailable`

## Auth failure interpretation

Bad credentials fail closed: the operation returns `Authorization` (or the
provider's documented error), an audit entry with correlation is recorded,
and NO success is fabricated. Credentials are registered as redaction
secrets and never appear in errors, telemetry, or the audit ring.

## Network diagnosis workflow (LF-010)

1. Collect provider signals through the production connectors:
   - OPNsense `read_telemetry` (firewall/router state)
   - OpenWrt `read_telemetry` (router/config state)
   - AdGuard `read_telemetry` + `read_blocklist` (DNS/filter telemetry)
2. Normalize to `NetworkFinding`/`DnsTelemetry`/`DnsBlocklistEntry` with
   provenance (source/provider, observation timestamp, resource/segment
   identity, confidence).
3. Derive a bounded diagnosis from OBSERVED facts only - never invent a
   root cause from missing telemetry. Partial data is reported as
   `UNAVAILABLE` evidence, never as a healthy provider.
4. Recommended remediation is a quarantine PROPOSAL (data, not
   containment).
5. Governed containment: proposal -> APPROVED (preauthorized AND
   reversible) -> apply -> provider acceptance (rule_ref) -> independent
   exact-target readback -> VERIFIED.

## Reversible containment workflow

- `propose_containment` captures the device label as source network
  (DATA; zero provider mutation calls).
- `apply_containment` requires state APPROVED + `is_auto_applicable()`
  (preauthorized AND reversible) BEFORE any transport call. Denial makes
  ZERO provider calls and records a POLICY audit entry.
- OPNsense: `addRule` (canonical block rule) + `apply`; rule recorded as
  `rule_ref`.
- OpenWrt: ubus login + `uci add`/`set`/`commit` + `rc init firewall
reload`; section recorded as `rule_ref`.

## Rollback

`revoke_containment` requires state APPLIED + rule_ref, then:

- OPNsense: `toggleRule/{uuid}/0` + `apply` (rule remains, disabled).
- OpenWrt: `uci set enabled 0` + `commit` + `rc init firewall reload`.
  Verification after revoke must FAIL (the rule no longer verifies).

## Exact-target verification

`verify_containment` reads back the provider state by the canonical
description/section (nexus-quarantine-<proposal_id>) and requires the
rule_ref to match AND the rule to be enabled AND the action to be
block/DROP. An unrelated rule never satisfies verification.

## Redacted evidence/logging

`SentinelObservability` is a bounded 256-entry redacted audit ring:
credentials are registered as redaction secrets and replaced with `***`
at insert (poison-safe). The audit ring records operation,
provider/result class, correlation, and bounded safe diagnostics - never
secrets. Evidence files under `.agent/state/evidence/` must pass the
redaction canary scan.

## Provider recovery

Failures fail closed and are recorded; the next request succeeds
truthfully once the provider answers again (bounded recovery is proven by
the M4 failure suite over real sockets). Capability advertisement returns
as soon as the transport answers.

## Zero-orphan cleanup

Live-fire fixtures are in-process threads over real sockets with bounded
accept deadlines; they terminate when the test joins them. No containers,
no child processes, no scratch files are created by the sentinel core
proofs. Verify with `git status --short` (clean tree) and
`docker ps -a` (no nexus-owned leaks) after a node run.

## Certification boundary

- nexus-sentinel contract: INTERNAL CONTRACT CERTIFIED.
- OPNsense/OpenWrt/AdGuard connectors: IMPLEMENTED / TRANSPORT_CERTIFIED
  against controlled real-socket fixtures.
- LF-010: PROVEN over canonical production Sentinel surfaces.
- Real physical OPNsense appliance / OpenWrt router / production AdGuard
  instance: NOT ASSERTED (certification debt owned by deployment/ship
  review).
