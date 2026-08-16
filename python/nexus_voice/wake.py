"""EP-021 wake word contract (SPEC-012 required behaviors 1, 3).

``WakeWordProvider`` is the provider-neutral wake word port. The local
default runtime is openWakeWord with custom commercially compatible
weights (SPEC-012 behavior 1; SPEC-019: noncommercial upstream weights
must not ship). Wake word accuracy/AEC certification gates whether the
always-on path may be advertised; until then push-to-talk is the
certified fallback (node contract).
"""

from __future__ import annotations

from dataclasses import dataclass

from .audio import AudioFrame
from .error import VoiceError, VoiceErrorCode
from .vocabulary import require_wake_word_state

WAKE_WORD_STATES = ("ARMED", "TRIGGERED", "DISARMED", "UNCERTIFIED")


@dataclass(frozen=True)
class WakeWordResult:
    """Wake word detection result for a frame.

    Attributes:
        state: canonical wake word state (ARMED/TRIGGERED/DISARMED/
            UNCERTIFIED).
        word: matched wake word label, or ``None`` when not triggered.
        confidence: detection confidence in [0, 1].
        frame: the frame that was evaluated.
    """

    state: str
    word: str | None
    confidence: float
    frame: AudioFrame

    def __post_init__(self) -> None:
        object.__setattr__(self, "state", require_wake_word_state(self.state))
        if self.state == "TRIGGERED" and not self.word:
            raise ValueError("triggered wake word must carry a word label")
        if not (0.0 <= self.confidence <= 1.0):
            raise ValueError("confidence must be in [0, 1]")


class WakeWordProvider:
    """Wake word detection port.

    Implementations must return TRIGGERED only on a real match with the
    certified model. An uncertified or unavailable model reports
    UNCERTIFIED, never a fabricated trigger.
    """

    def detect(self, frame: AudioFrame) -> WakeWordResult:
        """Return the wake word detection result for a frame.

        A port without a bound provider fails closed with a typed
        UNAVAILABLE error; it never fabricates a trigger.
        """
        raise VoiceError(
            VoiceErrorCode.Unavailable,
            "wake word provider has no implementation bound",
        )
