"""EP-021 deterministic wake decision engine (SPEC-012 behavior 3).

Owns the armed/triggered/disarmed/uncertified state machine and the
deterministic score-to-decision mapping. The real openWakeWord runtime
inference plugs in behind the score port (``WakeModelScore``); this
core never fabricates a trigger, never invents a model, and reports
UNCERTIFIED when no certified model is armed.
"""

from __future__ import annotations

from dataclasses import dataclass

from nexus_voice import AudioFrame, WakeWordResult, WakeWordState
from nexus_voice.error import VoiceError, VoiceErrorCode

WAKE_DECISION_STATES = ("ARMED", "TRIGGERED", "DISARMED", "UNCERTIFIED")


@dataclass(frozen=True)
class WakeModelScore:
    """Real score from a wake model for one audio frame.

    Attributes:
        model_id: model that produced the score.
        word: matched wake word label.
        score: raw model score in [0, 1].
    """

    model_id: str
    word: str
    score: float

    def __post_init__(self) -> None:
        if not self.model_id:
            raise ValueError("model_id must not be empty")
        if not self.word:
            raise ValueError("word must not be empty")
        if not (0.0 <= self.score <= 1.0):
            raise ValueError("score must be in [0, 1]")


class WakeDecisionEngine:
    """Deterministic wake word decision state machine.

    A model is armed explicitly (idempotent). A frame decision is
    computed only from the armed model's real score against the
    configured threshold:

    - no certified model armed -> UNCERTIFIED (never fabricated)
    - score >= threshold -> TRIGGERED with the real word label
    - score < threshold -> ARMED (no trigger)

    The engine is pure state; the frame is passed through to the result
    for pipeline correlation and is never retained or logged.
    """

    def __init__(self, threshold: float = 0.5) -> None:
        if not (0.0 <= threshold <= 1.0):
            raise ValueError("threshold must be in [0, 1]")
        self.threshold = threshold
        self._armed_model: str | None = None

    @property
    def armed_model(self) -> str | None:
        return self._armed_model

    def arm(self, model_id: str) -> None:
        """Arm a certified model id (idempotent)."""
        if not model_id:
            raise ValueError("model_id must not be empty")
        self._armed_model = model_id

    def disarm(self) -> None:
        """Disarm (idempotent)."""
        self._armed_model = None

    def decide(self, score: WakeModelScore, frame: AudioFrame) -> WakeWordResult:
        """Return the wake decision for a real model score and its frame.

        Raises ``VoiceError`` UNAVAILABLE when no model is armed or the
        score is for a different model than the armed one (the caller
        must never mix models silently).
        """
        if self._armed_model is None:
            raise VoiceError(
                VoiceErrorCode.Unavailable,
                "no wake model armed",
                detail={"state": "UNCERTIFIED"},
            )
        if score.model_id != self._armed_model:
            raise VoiceError(
                VoiceErrorCode.Unavailable,
                "wake score from a different model than the armed model",
                detail={"armed": self._armed_model, "scored": score.model_id},
            )
        if score.score >= self.threshold:
            return WakeWordResult(
                state=WakeWordState.Triggered,
                word=score.word,
                confidence=score.score,
                frame=frame,
            )
        return WakeWordResult(
            state=WakeWordState.Armed,
            word=None,
            confidence=score.score,
            frame=frame,
        )
