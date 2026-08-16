"""EP-021 VAD contract (SPEC-012 required behavior 3; Silero default).

``VadProvider`` is the provider-neutral voice activity detection port.
The local default is Silero VAD (SPEC-012 required behavior 1).
Decisions are deterministic per frame and never fabricate speech.
"""

from __future__ import annotations

from dataclasses import dataclass

from .audio import AudioFrame

VAD_DECISIONS = ("SPEECH", "SILENCE")


class VadDecision:
    """Canonical VAD decision for a frame."""

    Speech = "SPEECH"
    Silence = "SILENCE"


def _require_decision(value: str) -> str:
    if value not in VAD_DECISIONS:
        raise ValueError(f"unknown vad decision: {value}")
    return value


@dataclass(frozen=True)
class VadResult:
    """VAD decision for one audio frame.

    Attributes:
        decision: ``SPEECH`` or ``SILENCE``.
        confidence: probability in [0, 1] that the frame is speech.
        frame: the frame that was evaluated (metadata never leaked).
    """

    decision: str
    confidence: float
    frame: AudioFrame

    def __post_init__(self) -> None:
        object.__setattr__(self, "decision", _require_decision(self.decision))
        if not (0.0 <= self.confidence <= 1.0):
            raise ValueError("confidence must be in [0, 1]")


class VadProvider:
    """Voice activity detection port (SPEC-012 behavior 3).

    Implementations must be deterministic for identical frames and must
    never report speech for silence or vice versa. Real providers are
    certified in M3/M5; a provider that cannot run is unavailable, never
    a fabricated speech decision.
    """

    def detect(self, frame: AudioFrame) -> VadResult:
        """Return the VAD decision for a frame."""
        raise NotImplementedError
