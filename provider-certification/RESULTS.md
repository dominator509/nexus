# PROVIDER CERTIFICATION RESULTS

Selected deployment profile: FULLY_LOCAL (release manifest nexus-1.0.0-rc1,
release_channel STABLE). Per SPEC-008 a provider row becomes release-blocking
only when the selected deployment profile requires that provider and its
registry row is blocking_for_ship=true. Source of truth for component
certification state: .agent/state/evidence/CERTIFICATION_REGISTRY.md
(machine-readable ASCII registry, updated only with ledger evidence).

## Rows (transcribed from CERTIFICATION_REGISTRY.md at ship gate)

- deepseek-reflex (required reflex provider for the FULLY_LOCAL v1 release):
  registry provider_certification PROVIDER_CERTIFIED, blocking_for_ship false.
  Evidence: crates/nexus-reflex + .agent/state/evidence/ep014-m5/EP-014-M5-live-fire.md
  (real provider route deepseek-v4-flash, 8 canonical requests, mandatory
  runtime smoke real container PASS).
- model-gateway-provider-registry: INTERNAL_CERTIFIED for bifrost (internal
  gateway 127.0.0.1:8000); deepseek-v4-flash fallback PROVIDER_CERTIFIED via
  EP-014. blocking_for_ship false.
- home-assistant-provider: PROVIDER_CERTIFIED (real container live-fire;
  evidence EP-020-M3-real-ha-provider.md, EP-020-M4-forced-failures.md,
  EP-020-M5-real-provider-livefire.md). Optional for FULLY_LOCAL;
  blocking_for_ship false.
- nexus-appliances / nexus-irrigation / nexus-vacuum (EP-024): PROVIDER_CERTIFIED
  via EP-020 composition boundary. Optional for FULLY_LOCAL; blocking_for_ship false.
- nexus-gmail (EP-026 M2): NOT ASSERTED (no Gmail OAuth credentials in the
  environment). Optional for FULLY_LOCAL; blocking_for_ship false.
- nexus-microsoft-mail (EP-026 M3): TRANSPORT_CERTIFIED against controlled
  Graph-shaped fixtures only; real tenant NOT ASSERTED. Optional; blocking_for_ship false.
- nexus-imap-smtp / EP-026 email lifecycle (EP-026 M4/M5): PROTOCOL_CERTIFIED
  against the controlled GreenMail fixture only; external/public provider NOT
  ASSERTED (no credentials exist in this environment). Optional; blocking_for_ship false.
- agent-harness-adapters (codex/claude-code/hermes/openclaw CLIs): DEFERRED
  (external coding-agent CLIs NOT installed in this environment; LF-016 proves
  the real process boundary through a CONTROLLED_TEST_FIXTURE only).
  Optional for FULLY_LOCAL; blocking_for_ship false.
- wyoming-connector / nexus-bluetooth-audio (EP-022): INTERNAL_CERTIFIED or
  NOT ASSERTED against real containers/bus; hardware-bound transport
  certification DEFERRED. Optional; blocking_for_ship false.
- nexus-frigate / nexus-roku-home (EP-023): INTERNAL_CERTIFIED for the real
  pinned container + real RTSP/media chain; physical camera/Roku hardware
  NOT ASSERTED. Optional for FULLY_LOCAL; blocking_for_ship false.
- skill-plane-external-registry: NOT ASSERTED (no external/public skill
  registry claimed). Optional; blocking_for_ship false.

## Ship-gate statement

No provider row is release-blocking for the FULLY_LOCAL profile: every row in
CERTIFICATION_REGISTRY.md carries blocking_for_ship false, and optional
providers without real credentials stay honestly NOT ASSERTED / DEFERRED until
their deployment/ship owner produces credential-bound live-fire evidence.
Ship-level signed rows are recorded only with their validated structured
verification records under .agent/state/evidence (AUD-074); no textual marker
is presented as verification.
