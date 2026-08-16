"""EP-021 STT contract (SPEC-012 required behaviors 1-3).

``SttProvider`` is the provider-neutral speech-to-text port. The local
default is whisper.cpp (SPEC-012 behavior 1). Cloud fallbacks
(Deepgram, OpenAI) fit the same contract and are never certified
operational until real provider evidence exists (SPEC-012 behavior 2;
Reality rule). Raw audio is ephemeral and never continuously streamed
to cloud (SPEC-012 behavior 4).
"""

from __future__ import annotations

from dataclasses import dataclass

from .audio import AudioFrame
from .error import VoiceError, VoiceErrorCode


@dataclass(frozen=True)
class SttResult:
    """Speech-to-text result.

    Attributes:
        transcript: recognized text (empty for silence/no-speech).
        confidence: recognition confidence in [0, 1], or ``None`` when
            the provider does not expose confidence.
        language: BCP-47 language tag when known, else ``None``.
        frames: number of audio frames consumed.
    """

    transcript: str
    confidence: float | None = None
    language: str | None = None
    frames: int = 0

    def __post_init__(self) -> None:
        if self.confidence is not None and not (0.0 <= self.confidence <= 1.0):
            raise ValueError("confidence must be in [0, 1]")
        if self.frames < 0:
            raise ValueError("frames must be non-negative")


class SttProvider:
    """Speech-to-text port (SPEC-012 behaviors 1-3).

    Implementations transcribe real audio and return the recognized
    transcript. An unavailable provider raises ``VoiceError`` with code
    UNAVAILABLE or EXTERNAL_PROVIDER; it never fabricates a transcript.
    """

    def transcribe(self, frames: list[AudioFrame]) -> SttResult:
        """Transcribe a sequence of audio frames.

        A port without a bound provider fails closed with a typed
        UNAVAILABLE error; it never fabricates a transcript.
        """
        raise VoiceError(
            VoiceErrorCode.Unavailable,
            "stt provider has no implementation bound",
        )
