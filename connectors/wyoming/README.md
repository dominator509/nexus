# EP-022 M3: Wyoming protocol connector

Real client for the [Wyoming protocol](https://github.com/rhasspy/wyoming)
used by Home Assistant assist satellites. The connector implements the
provider-neutral `WyomingProvider` port surface from `crates/nexus-audio`:
connect, describe, stream audio, and receive wake detections over the
canonical protocol. No audio is fabricated: every detection is a real
event from a real Wyoming server.

## Component record (M3 milestone content 6)

| Field                | Value                                                                                   |
| -------------------- | --------------------------------------------------------------------------------------- |
| component            | rhasspy/wyoming-openwakeword (Wyoming protocol server)                                  |
| source               | https://github.com/rhasspy/wyoming-openwakeword                                         |
| container            | `rhasspy/wyoming-openwakeword:latest`                                                   |
| digest               | sha256:52cb1168731a1849fc28cf339c935fde58746bbabc94226668a40ef6ddf5d42b                 |
| license              | Apache-2.0 (upstream classifier); MIT LICENSE.md text                                   |
| protocol client      | `wyoming==1.10.0` (MIT) in the engine venv                                              |
| replacement contract | `WyomingProvider` port in crates/nexus-audio; any Wyoming-compatible server may replace |
| integration_mode     | sidecar (ephemeral container)                                                           |

## Real proof (observed)

- Real container boots and serves `tcp://0.0.0.0:10400`; canonical
  `describe` handshake returns the real `info` event advertising
  `openwakeword` wake program with installed models.
- Real Kokoro-generated `hey-jarvis.wav` (24 kHz -> converter to 16 kHz)
  streamed through the real protocol produces a real `Detection` event:
  `hey_jarvis` at timestamp 1000 ms.
- Real silence produces `NotDetected` (real negative; never fabricated).
- A dead server fails closed with connection refused (typed, fast).

## Operations

- Start the server: the gate script
  `scripts/ep022-m3-tests.sh` starts the pinned container on
  127.0.0.1:10400, waits for readiness, runs the suite, and tears the
  container down (zero orphans).
- Manual start:
  `docker run --rm -d --name ep022-wyoming-m3 -p 127.0.0.1:10400:10400 rhasspy/wyoming-openwakeword:latest`
- Run the connector CLI:
  `/opt/nexus-voice-engines/bin/python connectors/wyoming/connector.py --wav connectors/wyoming/tests/fixtures/hey-jarvis.wav --model hey_jarvis`
- Run the suite:
  `/opt/nexus-voice-engines/bin/python -m unittest discover -s connectors/wyoming/tests -p 'test_ep022_integration*.py' -v`

## Certification boundary

- The Wyoming protocol transport and the real container are proven here
  (transport integration).
- The container's bundled wake models (`okay_nabu`, `hey_jarvis`, etc.)
  are upstream test fixtures. The Nexus-owned wake model remains
  Nexus-owned per SPEC-019; production wake-model certification remains
  DEFERRED (EP-021 M3 recorded graph gap).
- No physical satellite/microphone/speaker hardware is exercised in this
  node (hardware/voice/profiles.yaml stays NOT_ASSERTED).
