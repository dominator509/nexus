# EP-025 M5 LF-012 governed phone call (real proof)

Real inbound governed call through real Asterisk 22.10.1, real ARI,
real RTP, real whisper.cpp STT, real Kokoro TTS.

```json
{
  "answer_http": 204,
  "asterisk_image": "andrius/asterisk:22.10.1_debian-trixie-amd64@sha256:7a22d773fe0f81adb715cd3e8df57c602726f8ef9d39deead6360e051483e280",
  "caller_number": "endpoint-v",
  "caller_recording_bytes": 0,
  "channel_id": "1788392819.1",
  "command_recognized": false,
  "container": "nexus-ep025-ast",
  "dialplan": [],
  "disclosure_policy": {
    "ai_disclosure_required": true,
    "jurisdiction": "US",
    "recording_consented": false,
    "retention_seconds": 0
  },
  "disclosure_satisfied": false,
  "governed_transcript_created": false,
  "hangup_http": 204,
  "hostile_content": false,
  "phrase_sha256": "c8fafd57049a46baff714f4c751537c7ab8152fdf68579a1cf30669eed0eca49",
  "play_http": 201,
  "play_media": "sound:nexus-lf012-response",
  "recording_denied_reason": "recording not consented",
  "recording_started": false,
  "recording_started_http": "skipped (consent denied)",
  "response_text": "Recording is not enabled for this call.",
  "scenario": "negative-disclosure",
  "stt_skipped": "recording not consented",
  "terminal_active_channels": 0,
  "timestamp": "2026-09-02T23:46:57+0000",
  "tts_duration_seconds": 2.85,
  "tts_sample_rate_hz": 24000,
  "tts_voice": "af_heart",
  "tts_wav_sha256": "448b014e798189cb47b6b2af20db54c7808e29ebb171629bccbcf64ad0fd3c4b"
}
```
