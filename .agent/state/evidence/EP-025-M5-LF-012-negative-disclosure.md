# EP-025 M5 LF-012 governed phone call (real proof)

Real inbound governed call through real Asterisk 22.10.1, real ARI,
real RTP, real whisper.cpp STT, real Kokoro TTS.

```json
{
  "answer_http": 204,
  "asterisk_image": "andrius/asterisk:22.10.1_debian-trixie-amd64@sha256:7a22d773fe0f81adb715cd3e8df57c602726f8ef9d39deead6360e051483e280",
  "caller_number": "endpoint-v",
  "caller_recording_bytes": 34604,
  "caller_recording_wav_sha256": "3832a483e0b2627b73f60ebc3837096979c5f3375778048fffc50837b231f4eb",
  "channel_id": "1787101054.1",
  "command_recognized": true,
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
  "phrase_sha256": "15192f82452a1c525045d37e452694b031029bf8052d0187e8434a3fefb4fca8",
  "play_http": 201,
  "play_media": "sound:nexus-lf012-response",
  "recording_name": "lf012-1787101054",
  "recording_started_http": 201,
  "recording_stop_http": 404,
  "response_text": "Turning on the lights now.",
  "scenario": "negative-disclosure",
  "stt_digest": "4eca89e948eb93037200bef29449b62389fdebae490f9e21996fd42f71ac7e8c",
  "stt_seconds": 2.16,
  "stt_transcript": "Turn on the lights please.",
  "terminal_active_channels": 0,
  "timestamp": "2026-08-19T00:57:32+0000",
  "tts_duration_seconds": 1.925,
  "tts_sample_rate_hz": 24000,
  "tts_voice": "af_heart",
  "tts_wav_sha256": "3593cbf4e9ad3f5dc1eba2d4d06f2e3cd8c3bc305e4bffb5bbd724168a01d0bf"
}
```
