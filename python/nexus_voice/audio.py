"""EP-021 audio frame contract (SPEC-012; provider-neutral).

An ``AudioFrame`` is an immutable, versioned unit of audio. It is the
only audio type that crosses the voice core boundary. Raw room audio
is ephemeral by default (SPEC-012 required behavior 4) and an
``AudioFrame`` never implies consent to record or transmit.
"""

from __future__ import annotations

from dataclasses import dataclass

from .vocabulary import require_endpoint_kind

# Canonical audio formats for the voice core.
AUDIO_FORMATS = ("PCM_S16LE", "PCM_F32LE", "OPUS", "MP3", "WAV")


class AudioFormat:
    """Canonical audio wire format."""

    PcmS16LE = "PCM_S16LE"
    PcmF32LE = "PCM_F32LE"
    Opus = "OPUS"
    Mp3 = "MP3"
    Wav = "WAV"


def _require_format(value: str) -> str:
    if value not in AUDIO_FORMATS:
        raise ValueError(f"unknown audio format: {value}")
    return value


@dataclass(frozen=True)
class AudioFrame:
    """Immutable audio frame crossing the voice core boundary.

    Attributes:
        format: canonical audio format (see ``AudioFormat``).
        sample_rate_hz: positive sample rate in hertz.
        channels: positive channel count.
        data: immutable audio bytes. Never logged or placed in errors.
        endpoint_kind: canonical endpoint kind that produced the frame
            (SPEC-012 top-ten satellite matrix), or ``None`` when the
            source is not an endpoint (e.g. synthetic corpus input).
        correlation_id: optional correlation id preserved through the
            pipeline.
        sequence: monotonic frame sequence number within a session.
    """

    format: str
    sample_rate_hz: int
    channels: int
    data: bytes
    endpoint_kind: str | None = None
    correlation_id: str | None = None
    sequence: int = 0

    def __post_init__(self) -> None:
        object.__setattr__(self, "format", _require_format(self.format))
        if self.endpoint_kind is not None:
            object.__setattr__(self, "endpoint_kind", require_endpoint_kind(self.endpoint_kind))
        if self.sample_rate_hz <= 0:
            raise ValueError("sample_rate_hz must be positive")
        if self.channels <= 0:
            raise ValueError("channels must be positive")
        if self.sequence < 0:
            raise ValueError("sequence must be non-negative")

    def duration_seconds(self) -> float:
        """Duration of this frame in seconds."""
        return len(self.data) / max(
            1, self.sample_rate_hz * self.channels * self._bytes_per_sample()
        )

    def _bytes_per_sample(self) -> int:
        if self.format in (AudioFormat.PcmS16LE,):
            return 2
        if self.format in (AudioFormat.PcmF32LE,):
            return 4
        # Compressed formats have no fixed bytes-per-sample; duration
        # is not derivable from byte count alone. Callers must not rely
        # on duration for compressed frames.
        return 1

    def to_dict(self) -> dict[str, object]:
        """Versioned serialization (never includes the audio payload)."""
        return {
            "schema": "nexus.voice.audio_frame.v1",
            "format": self.format,
            "sample_rate_hz": self.sample_rate_hz,
            "channels": self.channels,
            "endpoint_kind": self.endpoint_kind,
            "correlation_id": self.correlation_id,
            "sequence": self.sequence,
            "payload_bytes": len(self.data),
        }

    @classmethod
    def from_dict(cls, payload: dict[str, object]) -> AudioFrame:
        """Deserialize a versioned dict; unknown schema versions rejected."""
        schema = payload.get("schema")
        if schema != "nexus.voice.audio_frame.v1":
            raise ValueError(f"unknown audio frame schema: {schema!r}")
        data = payload.get("payload_bytes")
        if not isinstance(data, int) or data < 0:
            raise ValueError("payload_bytes must be a non-negative integer")
        fmt = payload["format"]
        if not isinstance(fmt, str):
            raise ValueError("format must be a string")
        rate = payload["sample_rate_hz"]
        if not isinstance(rate, int):
            raise ValueError("sample_rate_hz must be an integer")
        channels = payload["channels"]
        if not isinstance(channels, int):
            raise ValueError("channels must be an integer")
        endpoint = payload.get("endpoint_kind")
        if endpoint is not None and not isinstance(endpoint, str):
            raise ValueError("endpoint_kind must be a string or null")
        correlation = payload.get("correlation_id")
        if correlation is not None and not isinstance(correlation, str):
            raise ValueError("correlation_id must be a string or null")
        sequence = payload.get("sequence", 0)
        if not isinstance(sequence, int):
            raise ValueError("sequence must be an integer")
        return cls(
            format=fmt,
            sample_rate_hz=rate,
            channels=channels,
            data=b"\x00" * data,
            endpoint_kind=endpoint,
            correlation_id=correlation,
            sequence=sequence,
        )
