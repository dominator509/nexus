# infra/voice - EP-021 M3 real voice engine integration

Real local voice engines for the Nexus voice core (SPEC-012), integrated
as an isolated sidecar so the main Nexus Python environment stays frozen.

## Engines

| Capability | Engine                                   | Version | License    | Proof                         |
| ---------- | ---------------------------------------- | ------- | ---------- | ----------------------------- |
| VAD        | Silero VAD (ONNX)                        | v5.1    | MIT        | speech 0.737 vs silence 0.004 |
| Wake       | openWakeWord runtime + Nexus-owned model | 0.4.0   | Apache-2.0 | 26/26 pos @ 1.0, neg <= 0.089 |
| STT        | whisper.cpp                              | v1.7.4  | MIT        | exact phrase transcription    |
| TTS        | Kokoro (torch CPU)                       | 0.9.4   | Apache-2.0 | new waveform per text         |

See `manifests/engines.yaml`, `manifests/models.yaml`, and
`manifests/certification.yaml` for digests, licenses, and certification
status. Noncommercial openwakeword pretrained weights are never used
(SPEC-019).

## Layout

- `workers/` - real engine workers (run under the sidecar venv, one
  canonical JSON object on stdout):
  - `silero_worker.py` - real Silero ONNX inference -> VAD decision/segments
  - `wake_worker.py` - real openwakeword streaming detection
  - `whisper_worker.py` - real whisper-cli transcription
  - `kokoro_worker.py` - real Kokoro synthesis -> WAV
  - `crop_worker.py` - real signal crop for utterance capture
  - `fixture_gen.py` - regenerable controlled fixtures (real Kokoro TTS)
  - `train_wake_model.py` - trains the Nexus-owned wake model and asserts
    real engine separation
- `adapters/` - nexus_voice provider adapters (project interpreter,
  stdlib-only; subprocess to workers):
  - `silero_vad_adapter.py` (VadProvider)
  - `wake_word_adapter.py` (WakeWordProvider)
  - `stt_adapter.py` (SttProvider)
  - `tts_adapter.py` (TtsProvider)
  - `pipeline.py` - composed chain (VAD -> wake -> utterance -> STT) and
    text -> TTS
- `engine_env.py` - sidecar resolver + worker runner
- `manifests/` - engines, models, certification

## Runtime artifact roots (never committed)

- sidecar venv: `/opt/nexus-voice-engines` (`NEXUS_VOICE_ENGINE_VENV`)
- models: `/opt/nexus-voice-models` (`NEXUS_VOICE_MODELS`)
- fixtures: `/opt/nexus-voice-fixtures` (`NEXUS_VOICE_FIXTURES`)

## Setup and regeneration

```sh
# engine venv (Python 3.12): onnxruntime, openwakeword, kokoro,
# torch (CPU index), scikit-learn, scipy, numpy, skl2onnx, onnx,
# en-core-web-sm (spaCy wheel)
uv venv --python 3.12 /opt/nexus-voice-engines
uv pip install --python /opt/nexus-voice-engines \
  numpy onnxruntime scipy scikit-learn openwakeword skl2onnx onnx \
  "https://download.pytorch.org/whl/cpu"  # torch via --index-url in practice

# fixtures (real Kokoro TTS; deterministic)
/opt/nexus-voice-engines/bin/python infra/voice/workers/fixture_gen.py

# wake model (real features, Nexus-owned weights; asserts engine separation)
/opt/nexus-voice-engines/bin/python infra/voice/workers/train_wake_model.py

# individual engine proofs
/opt/nexus-voice-engines/bin/python infra/voice/workers/silero_worker.py --wav <wav>
/opt/nexus-voice-engines/bin/python infra/voice/workers/wake_worker.py --wav <wav>
/opt/nexus-voice-engines/bin/python infra/voice/workers/whisper_worker.py --wav <wav>
/opt/nexus-voice-engines/bin/python infra/voice/workers/kokoro_worker.py --text 'hi' --out /tmp/hi.wav
```

The ambient `VIRTUAL_ENV` is stripped by `engine_env.run_worker` so the
sidecar interpreter is never shadowed (spaCy resolution).

## Gate

`sh scripts/nodes/EP-021.sh M3` runs the `ep021_integration` suite under
`uv run --frozen pytest tests/voice/core`.
