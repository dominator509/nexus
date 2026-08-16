# EP-021 M5 voice engine benchmarks (real measurements)

Wall-clock latency per engine on the controlled fixtures (CPU VPS).

```json
{
  "kokoro_tts": {
    "max_seconds": 31.3776,
    "mean_seconds": 27.2342,
    "runs": [
      31.3776,
      24.3917,
      25.9332
    ],
    "sample": {
      "duration_seconds": 1.825,
      "rms": 0.047672,
      "sample_rate_hz": 24000,
      "sha256": "04b5a2cae6cff117eaa0d24d55749c9ed1dd435e97bb8508668d54cd41e3d1e3",
      "voice": "af_heart",
      "wav": "/tmp/bench_tts_2.wav"
    }
  },
  "silero_vad": {
    "max_seconds": 0.8709,
    "mean_seconds": 0.6487,
    "runs": [
      0.8709,
      0.5217,
      0.5536
    ],
    "sample": {
      "decision": "SPEECH",
      "max_prob": 0.999721,
      "mean_prob": 0.740761,
      "seconds": 3.275,
      "segments": [
        [
          0.32,
          2.912
        ]
      ],
      "speech_window_count": 78,
      "window_count": 102
    }
  },
  "wake_detection": {
    "max_seconds": 3.4506,
    "mean_seconds": 3.2289,
    "runs": [
      3.2538,
      2.9823,
      3.4506
    ],
    "sample": {
      "detected": true,
      "frames": 19,
      "score": 1.0,
      "threshold": 0.7,
      "trigger_frame": 6400,
      "trigger_seconds": 0.4
    }
  },
  "whisper_stt": {
    "max_seconds": 3.3905,
    "mean_seconds": 3.2669,
    "runs": [
      3.3905,
      3.1897,
      3.2206
    ],
    "sample": {
      "language": "en",
      "returncode": 0,
      "seconds": 3.275,
      "transcript": "The quick brown fox jumps over the lazy dog."
    }
  }
}
```
