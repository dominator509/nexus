# hardware/voice - EP-022 hardware conformance profiles (SPEC-012)

Every hardware class in the SPEC-012 top-ten satellite matrix plus the
software endpoints has a conformance profile in `profiles.yaml`
(acceptance obligation 1: "Top ten hardware classes have conformance
profiles").

`conformance: DEFINED` means the class profile exists and the provider-
neutral contract (crates/nexus-audio) accepts the canonical class.
`certified: NOT_ASSERTED` means no physical hardware of that class was
exercised in this node: no microphone, speaker, or satellite hardware
certification is claimed (Reality rule). Hardware certification is a
deferred physical/field activity owned by its future certification
owner; the fallback per the node contract certifies Home Assistant
Voice Preview Edition, Linux satellite, Android, and iOS first.
