# EP-014 M3 reflex transport integration suite

This directory is the EP-014 M3 milestone manifest root. The integration
tests that prove the REAL `DeepSeekReflexTransport` boundary live with
the reflex crate:

- `crates/nexus-reflex/tests/ep014_integration_transport.rs` - real HTTP
  against a controlled provider sandbox speaking the OpenAI-compatible
  chat completions surface (gate selector:
  `cargo test --locked -p nexus-reflex ep014_integration`).

The transport under proof is production code: `DeepSeekReflexTransport`
wraps EP-013's real `OpenAiCompatibleTransport` (pinned ureq) and
normalizes the canonical `NexusControlObject`. The sandbox is a protocol
simulator under TESTING.md's integration layer; it never certifies the
DeepSeek commercial API, but it proves the reflex transport contract
across a real socket: canonical request assembly, response
normalization, typed SPEC-006 error classification (timeout, malformed
response, unavailable), and the full `DeepSeekFlashProvider` ->
`DeepSeekReflexTransport` -> HTTP path.

Provider identity is the canonical DeepSeek V4 Flash manifest
(`deepseek-v4-flash`, base `https://api.deepseek.com/v1`, credential ref
`secret/model/deepseek`) from `config/models/providers/providers.json`
and `COMPONENT_REGISTRY.yaml id=deepseek-v4-flash`.
