# SPEC-012 - Voice, Speech, Wake Word, Speaker Evidence, Satellites, Bluetooth, and Audio Routing

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define local voice pipeline, fallbacks, custom wake models, multi-room satellites, endpoint mobility, privacy, and latency.

## Canonical terms

AudioEndpoint, VoiceSession, VAD, WakeWord, STTProvider, TTSProvider, SpeakerEvidence, Diarization, AEC, Wyoming, Assist Satellite. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Local defaults are Silero VAD, custom commercially compatible openWakeWord models, whisper.cpp STT, and Kokoro TTS.
2. Fallback providers include Deepgram and OpenAI for STT and ElevenLabs and Azure Speech for TTS.
3. The pipeline performs local wake detection, VAD, denoise and AEC, streaming STT, interaction resolution, response, streaming TTS, and interruption handling.
4. Raw room audio is ephemeral by default and is never continuously streamed to cloud.
5. Speaker verification and diarization are local evidence services with explicit confidence and unknown-speaker states.
6. The top-ten satellite matrix includes Voice Preview, ESP32-S3-BOX-3, Atom Echo, generic ESP32-S3 I2S, Pi 5, Pi 4, Pi Zero 2 W, x86 Linux, Android, and iOS.
7. BlueZ and PipeWire support Linux Bluetooth endpoints; iOS and Android use native audio and BLE APIs.
8. A conversation may transfer endpoints without losing principal, objective, privacy, or transcript context.
9. Every fixed microphone has hardware mute, visible state, local wake, signed firmware path, and isolated network profile.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Voice-only high-risk approval
- Shipping noncommercial wake weights
- Continuous cloud audio
- Assuming Bluetooth microphone quality

## Required tests

- Recorded corpus accuracy and latency
- Wake false accept and reject
- Speaker unknown
- Shared-room privacy
- Endpoint transfer live-fire
- AEC barge-in
- Cloud fallback disclosure

## Acceptance

Certified satellites deliver responsive private voice with local defaults, clear fallback behavior, and no elevation of voice evidence to authority.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
