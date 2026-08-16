"""EP-021 TTS contract (SPEC-012 required behaviors 1-3).

``TtsProvider`` is the provider-neutral text-to-speech port. The local
default is Kokoro (SPEC-012 behavior 1). Cloud fallbacks (ElevenLabs,
Azure Speech) fit the same contract and are never certified operational
until real provider evidence exists (SPEC-012 behavior 2).
"""

from __future__ import annotations

from dataclasses import dataclass

from .audio import AudioFrame


@dataclass(frozen=True)
class TtsResult:
    """Text-to-speech result.

    Attributes:
        audio: synthesized audio frame(s) in the provider's canonical
            format. Never logged or placed in errors.
        frames: number of audio frames produced.
    """

    audio: tuple[AudioFrame, ...]

    def __post_init__(self) -> None:
        if not self.audio:
            raise ValueError("tts result must contain at least one frame")

    @property
    def frames(self) -> int:
        return len(self.audio)


class TtsProvider:
    """Text-to-speech port (SPEC-012 behaviors 1-3).

    Implementations synthesize real audio from text. An unavailable
    provider raises ``VoiceError`` (UNAVAILABLE/EXTERNAL_PROVIDER); it
    never fabricates audio.
    """

    def synthesize(self, text: str) -> TtsResult:
        """Synthesize speech audio for text."""
        raise NotImplementedError
