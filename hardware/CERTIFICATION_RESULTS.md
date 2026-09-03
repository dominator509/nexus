# HARDWARE CERTIFICATION RESULTS

Selected deployment profile: FULLY_LOCAL (release manifest nexus-1.0.0-rc1,
release_channel STABLE). Per SPEC-008 and the EP-040/EP-043 contract, physical
hardware rows become release-blocking only for the deployment profile that
requires them. A core server-only release marks hardware capabilities
unavailable without claiming certification. Source of truth for component
certification state: .agent/state/evidence/CERTIFICATION_REGISTRY.md
(machine-readable ASCII registry, updated only with ledger evidence).

## Rows (transcribed from CERTIFICATION_REGISTRY.md at ship gate)

- nexus-audio (EP-022 M1): hardware_certification NOT ASSERTED
  (hardware/voice/profiles.yaml conformance DEFINED only; physical classes
  never upgraded from YAML). blocking_for_ship false.
- nexus-assist-satellite (EP-022 M2): hardware_certification NOT ASSERTED.
  blocking_for_ship false.
- wyoming-connector (EP-022 M3): hardware_certification NOT ASSERTED; Nexus
  wake model certification DEFERRED per SPEC-019. blocking_for_ship false.
- nexus-bluetooth-audio (EP-022 M4): hardware_certification NOT ASSERTED
  (BlueZ absence proven by real GetNameOwner NameHasNoOwner; connector fails
  closed UNAVAILABLE). blocking_for_ship false.
- nexus-vision (EP-023 M1): hardware_certification NOT ASSERTED
  (hardware/cameras/profiles.yaml conformance DEFINED only). blocking_for_ship false.
- nexus-frigate (EP-023 M2/M3/M4): hardware_certification NOT ASSERTED
  (no physical camera; stream refs stay Unverified). blocking_for_ship false.
- nexus-roku-home (EP-023 M5): hardware_certification NOT ASSERTED
  (no Roku hardware/credentials bound on this host). blocking_for_ship false.
- nexus-appliances / nexus-irrigation / nexus-vacuum (EP-024):
  hardware_certification NOT ASSERTED (physical appliance/irrigation/vacuum
  hardware DEFERRED; fixtures never become hardware certification).
  blocking_for_ship false.
- nexus-vision-e2e (EP-023 M5): two-way audio live certification NOT ASSERTED
  (requires real speaker/media path). blocking_for_ship false.

## Ship-gate statement

No hardware row is release-blocking for the FULLY_LOCAL core server release:
no physical lab class in LAB_INVENTORY.yaml is required by this profile, and
every hardware-capability row in CERTIFICATION_REGISTRY.md honestly records
NOT ASSERTED / DEFERRED with blocking_for_ship false. No physical hardware
certification is claimed anywhere. Ship-level signed rows are recorded only
with their validated structured verification records under .agent/state/
evidence (AUD-074); no textual marker is presented as verification.
