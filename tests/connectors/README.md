# EP-011 connector tests

M3 live-fire suite for the connector SDKs and sidecar runtime (SPEC-022).

## Layout

- `fixture_sidecar.py` - REAL sandboxed Connector Sidecar process: an
  HTTP server on 127.0.0.1 with an ephemeral port implementing the
  canonical sidecar REST transport. Test/verification zone.
- `golden/` - canonical wire fixtures generated from the authoritative
  Rust types (run `cargo run -p nexus-connector-sdk --example
generate_golden` to regenerate).
- `test_ep011_m3_live.py` - live transport, webhook normalizer, legacy
  poller, credential broker, transport security, error parity.
- `test_ep011_m3_failures.py` - real failure matrix (directive O).
- `test_ep011_m3_parity.py` - cross-language golden wire parity.

Test names: `ep011_integration_*` / `ep011_failure_*` (M3 gate selects
these with `-o python_functions=...` plus the vacuity guard).
