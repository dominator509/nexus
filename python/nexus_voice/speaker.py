"""EP-021 speaker evidence contract (SPEC-012 required behavior 5).

Speaker verification and diarization are LOCAL evidence services with
explicit confidence and unknown-speaker states. Voice is evidence, not
cryptographic authentication (INV-003). ``SpeakerEvidence`` never
elevates to authority.
"""

from __future__ import annotations

from dataclasses import dataclass

from .error import VoiceError, VoiceErrorCode

SPEAKER_VERDICTS = ("MATCH", "NOMATCH", "UNKNOWN", "UNSUPPORTED")


class SpeakerVerdict:
    """Canonical speaker verification verdict."""

    Match = "MATCH"
    NoMatch = "NOMATCH"
    Unknown = "UNKNOWN"
    Unsupported = "UNSUPPORTED"


def _require_verdict(value: str) -> str:
    if value not in SPEAKER_VERDICTS:
        raise ValueError(f"unknown speaker verdict: {value}")
    return value


@dataclass(frozen=True)
class SpeakerEvidence:
    """Speaker verification evidence (SPEC-012 behavior 5).

    Attributes:
        verdict: MATCH/NOMATCH/UNKNOWN/UNSUPPORTED.
        confidence: confidence in [0, 1] (0 when UNSUPPORTED).
        speaker_id: matched speaker identity when MATCH, else ``None``.
        utterance_id: correlation to the verified utterance.
    """

    verdict: str
    confidence: float
    speaker_id: str | None = None
    utterance_id: str | None = None

    def __post_init__(self) -> None:
        object.__setattr__(self, "verdict", _require_verdict(self.verdict))
        if not (0.0 <= self.confidence <= 1.0):
            raise ValueError("confidence must be in [0, 1]")
        if self.verdict == "MATCH" and not self.speaker_id:
            raise ValueError("match verdict must carry a speaker id")
        if self.verdict == "UNSUPPORTED" and self.confidence != 0.0:
            raise ValueError("unsupported verdict must carry zero confidence")

    def as_evidence(self) -> dict[str, object]:
        """Evidence surface (never authority)."""
        return {
            "schema": "nexus.voice.speaker_evidence.v1",
            "verdict": self.verdict,
            "confidence": self.confidence,
            "speaker_id": self.speaker_id,
            "utterance_id": self.utterance_id,
        }


class SpeakerEvidenceProvider:
    """Speaker evidence port (SPEC-012 behavior 5; INV-003).

    Implementations verify or diarize locally and always return an
    explicit UNKNOWN or UNSUPPORTED verdict when they cannot decide.
    Evidence never authenticates; callers must treat it as a soft
    signal only.
    """

    def verify(self, utterance_id: str, expected_speaker_id: str) -> SpeakerEvidence:
        """Verify an utterance against an expected speaker identity.

        A port without a bound provider fails closed with a typed
        UNAVAILABLE error; it never fabricates a verdict.
        """
        raise VoiceError(
            VoiceErrorCode.Unavailable,
            "speaker evidence provider has no implementation bound",
        )
