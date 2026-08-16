#!/usr/bin/env python3
"""EP-021 M3 deterministic fixture generator (real Kokoro TTS).

Generates the controlled-test fixtures used by the real engine proofs:
wake positives ("hey nexus" across voices/speeds), wake negatives (other
phrases), digital silence, deterministic noise, the STT phrase, and the
composed chain clip (ambient lead + wake + gap + utterance).

Fixtures are real synthesized speech (never prerecorded engine output)
and are regenerable; the printed JSON manifest records SHA-256 of each.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import wave

import numpy as np
from kokoro import KPipeline

SR = 24000

POSITIVE_TEXTS: list[tuple[str, float]] = [
    ("hey nexus", s)
    for s in (
        1.0,
        1.0,
        0.95,
        1.1,
        0.95,
        1.0,
        1.15,
        0.9,
        0.85,
        1.05,
        1.2,
        1.0,
        1.0,
        1.0,
        1.0,
        1.0,
        1.0,
        1.0,
    )
]
POSITIVE_VOICES = [
    "af_heart",
    "am_michael",
    "af_heart",
    "am_michael",
    "af_bella",
    "am_fenrir",
    "af_heart",
    "am_michael",
    "af_heart",
    "af_bella",
    "am_michael",
    "am_fenrir",
    "af_nicole",
    "am_onyx",
    "af_heart",
    "am_michael",
    "af_bella",
    "am_fenrir",
] + [
    "af_heart",
    "am_michael",
    "af_bella",
    "am_fenrir",
    "af_nicole",
    "am_onyx",
    "af_heart",
    "am_michael",
]
NEGATIVE_TEXTS = [
    "good morning",
    "what time is it",
    "turn on the lights",
    "play some music",
    "open the garage door",
    "stop the music",
    "set a timer for ten minutes",
    "what is the weather",
    "call mom",
    "good morning everyone",
    "what time is it now",
    "please open the window",
    "turn off the television",
    "start the coffee maker",
    "navigate to the nearest gas station",
    "send a message to my wife",
    "what song is this",
    "remind me to buy milk",
    "how is the traffic today",
]


def synth(pipeline: KPipeline, text: str, voice: str, speed: float) -> np.ndarray:
    return np.concatenate([a for _, _, a in pipeline(text, voice=voice, speed=speed)]).astype(
        np.float32
    )


def write_wav(path: str, audio: np.ndarray) -> None:
    pcm = (np.clip(audio, -1.0, 1.0) * 32767).astype(np.int16)
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())


def sha256(path: str) -> str:
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--fixtures", default=os.environ.get("NEXUS_VOICE_FIXTURES", "/opt/nexus-voice-fixtures")
    )
    args = ap.parse_args()
    os.makedirs(args.fixtures, exist_ok=True)
    pipeline = KPipeline(lang_code="a", repo_id="hexgrad/Kokoro-82M")
    manifest: dict[str, object] = {}

    for i, (text, speed) in enumerate(POSITIVE_TEXTS):
        voice = POSITIVE_VOICES[i % len(POSITIVE_VOICES)]
        out = os.path.join(args.fixtures, f"wake_pos_{i}.wav")
        write_wav(out, synth(pipeline, text, voice, speed))
        manifest[f"wake_pos_{i}.wav"] = sha256(out)
    for i, text in enumerate(NEGATIVE_TEXTS):
        voice = POSITIVE_VOICES[(i * 3) % len(POSITIVE_VOICES)]
        out = os.path.join(args.fixtures, f"wake_neg_{i}.wav")
        write_wav(out, synth(pipeline, text, voice, 1.0))
        manifest[f"wake_neg_{i}.wav"] = sha256(out)

    silence = np.zeros(SR * 2, dtype=np.float32)
    write_wav(os.path.join(args.fixtures, "silence.wav"), silence)
    manifest["silence.wav"] = sha256(os.path.join(args.fixtures, "silence.wav"))

    rng = np.random.default_rng(42)
    noise = (rng.uniform(-1, 1, SR * 2) * 0.05).astype(np.float32)
    write_wav(os.path.join(args.fixtures, "noise.wav"), noise)
    manifest["noise.wav"] = sha256(os.path.join(args.fixtures, "noise.wav"))

    write_wav(
        os.path.join(args.fixtures, "stt_phrase.wav"),
        synth(pipeline, "the quick brown fox jumps over the lazy dog", "af_heart", 1.0),
    )
    manifest["stt_phrase.wav"] = sha256(os.path.join(args.fixtures, "stt_phrase.wav"))
    write_wav(
        os.path.join(args.fixtures, "chain_utterance.wav"),
        synth(pipeline, "turn on the lights", "af_heart", 1.0),
    )
    manifest["chain_utterance.wav"] = sha256(os.path.join(args.fixtures, "chain_utterance.wav"))

    chain = np.concatenate(
        [
            np.zeros(int(SR * 1.0), dtype=np.float32),
            synth(pipeline, "hey nexus", "af_heart", 1.0),
            np.zeros(int(SR * 0.5), dtype=np.float32),
            synth(pipeline, "turn on the lights", "af_heart", 1.0),
        ]
    )
    write_wav(os.path.join(args.fixtures, "chain_full.wav"), chain)
    manifest["chain_full.wav"] = sha256(os.path.join(args.fixtures, "chain_full.wav"))

    print(json.dumps({"fixtures": args.fixtures, "files": manifest}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
