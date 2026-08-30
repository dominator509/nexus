# EP-025 M5 LF-012 governed phone call (real proof)

Real inbound governed call through real Asterisk 22.10.1, real ARI,
real RTP, real whisper.cpp STT, real Kokoro TTS.

```json
{
  "answer_http": 204,
  "asterisk_image": "andrius/asterisk:22.10.1_debian-trixie-amd64@sha256:7a22d773fe0f81adb715cd3e8df57c602726f8ef9d39deead6360e051483e280",
  "caller_number": "endpoint-v",
  "caller_recording_bytes": 34604,
  "caller_recording_wav_sha256": "512e9277cecd5f02c97bc9179386d147206ae2d3045edfc47d2bd8dfe384a442",
  "channel_id": "1788115208.0",
  "command_recognized": true,
  "container": "nexus-ep025-ast",
  "dialplan": [],
  "disclosure_policy": {
    "ai_disclosure_required": true,
    "jurisdiction": "US",
    "recording_consented": true,
    "retention_seconds": 0
  },
  "disclosure_satisfied": true,
  "governed_transcript_created": true,
  "hangup_http": 204,
  "hostile_content": false,
  "phrase_sha256": "3c33347762bd0be0ce742fafae2f2dd74deec06530b14c1505fa4c07a7f2a556",
  "play_http": 201,
  "play_media": "sound:nexus-lf012-response",
  "recording_name": "lf012-1788115209",
  "recording_started_http": 201,
  "recording_stop_http": 404,
  "response_text": "Turning on the lights now.",
  "scenario": "positive",
  "stt_digest": "4eca89e948eb93037200bef29449b62389fdebae490f9e21996fd42f71ac7e8c",
  "stt_seconds": 2.16,
  "stt_transcript": "Turn on the lights please.",
  "terminal_active_channels": 0,
  "timestamp": "2026-08-30T18:40:06+0000",
  "tts_duration_seconds": 1.925,
  "tts_sample_rate_hz": 24000,
  "tts_voice": "af_heart",
  "tts_wav_sha256": "07a3fd366352c7776e3832b2c9d56473e974d2129908f1fc0fd96de1ddde4fa8"
}
```
