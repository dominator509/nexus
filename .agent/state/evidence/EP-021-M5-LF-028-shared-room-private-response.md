# LF-028 shared-room private response (EP-021 M5)

Real proof: sensitive response in an occupied room is routed privately, never spoken aloud on the room speaker.

```json
{
  "hardware_mute_route": {
    "audible": false,
    "channel": "SUPPRESSED",
    "policy_zone": "PRIVATE",
    "reason": "hardware_mute"
  },
  "private_room_route": {
    "audible": true,
    "channel": "SPOKEN",
    "policy_zone": "PRIVATE",
    "reason": "local_audio"
  },
  "shared_room_route": {
    "audible": false,
    "channel": "PRIVATE",
    "policy_zone": "SHARED_ROOM",
    "reason": "shared_room_sensitive"
  },
  "tts_synthesized": {
    "duration_seconds": 5.275,
    "rms": 0.050822,
    "sample_rate_hz": 24000
  }
}
```
