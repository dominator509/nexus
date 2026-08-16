"""EP-021 voice session contract (SPEC-012 required behaviors 3, 8).

``VoiceSession`` carries the principal, objective, privacy, transcript
context, and endpoint state of one interaction. A conversation may
transfer endpoints without losing context (SPEC-012 behavior 8).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .vocabulary import require_endpoint_kind

SESSION_STATES = ("IDLE", "LISTENING", "PROCESSING", "RESPONDING", "INTERRUPTED", "ENDED")


class SessionState:
    """Canonical voice session state."""

    Idle = "IDLE"
    Listening = "LISTENING"
    Processing = "PROCESSING"
    Responding = "RESPONDING"
    Interrupted = "INTERRUPTED"
    Ended = "ENDED"


def _require_state(value: str) -> str:
    if value not in SESSION_STATES:
        raise ValueError(f"unknown session state: {value}")
    return value


@dataclass
class VoiceSession:
    """One voice interaction.

    Attributes:
        session_id: stable session identity.
        principal_id: authenticated principal (never inferred from
            voice evidence).
        endpoint_kind: current audio endpoint (SPEC-012 behavior 6).
        state: canonical session state.
        objective: interaction objective text.
        transcript: ordered transcript entries (utterance -> text).
        privacy_policy_id: id of the governing privacy policy.
        tenant_id: owning tenant.
        correlation_id: preserved pipeline correlation.
    """

    session_id: str
    principal_id: str
    endpoint_kind: str
    state: str = SessionState.Idle
    objective: str = ""
    transcript: list[tuple[str, str]] = field(default_factory=list)
    privacy_policy_id: str = ""
    tenant_id: str | None = None
    correlation_id: str | None = None

    def __post_init__(self) -> None:
        object.__setattr__(self, "endpoint_kind", require_endpoint_kind(self.endpoint_kind))
        object.__setattr__(self, "state", _require_state(self.state))
        if not self.session_id:
            raise ValueError("session_id must not be empty")
        if not self.principal_id:
            raise ValueError("principal_id must not be empty")

    def append_transcript(self, speaker: str, text: str) -> None:
        """Append a transcript entry (speaker is a label, never identity)."""
        self.transcript.append((speaker, text))

    def transfer_to(self, endpoint_kind: str) -> None:
        """Transfer the session to another endpoint without losing context.

        SPEC-012 behavior 8: principal, objective, privacy, and
        transcript context are preserved.
        """
        require_endpoint_kind(endpoint_kind)
        self.endpoint_kind = endpoint_kind

    def to_dict(self) -> dict[str, object]:
        """Versioned serialization (never includes raw audio)."""
        return {
            "schema": "nexus.voice.session.v1",
            "session_id": self.session_id,
            "principal_id": self.principal_id,
            "endpoint_kind": self.endpoint_kind,
            "state": self.state,
            "objective": self.objective,
            "transcript": list(self.transcript),
            "privacy_policy_id": self.privacy_policy_id,
            "tenant_id": self.tenant_id,
            "correlation_id": self.correlation_id,
        }

    @classmethod
    def from_dict(cls, payload: dict[str, object]) -> VoiceSession:
        """Deserialize a versioned dict; unknown schema versions rejected."""
        schema = payload.get("schema")
        if schema != "nexus.voice.session.v1":
            raise ValueError(f"unknown voice session schema: {schema!r}")
        session_id = payload["session_id"]
        if not isinstance(session_id, str):
            raise ValueError("session_id must be a string")
        principal_id = payload["principal_id"]
        if not isinstance(principal_id, str):
            raise ValueError("principal_id must be a string")
        endpoint = payload["endpoint_kind"]
        if not isinstance(endpoint, str):
            raise ValueError("endpoint_kind must be a string")
        state = payload.get("state", "IDLE")
        if not isinstance(state, str):
            raise ValueError("state must be a string")
        objective = payload.get("objective", "")
        if not isinstance(objective, str):
            raise ValueError("objective must be a string")
        transcript_raw = payload.get("transcript", [])
        if not isinstance(transcript_raw, list):
            raise ValueError("transcript must be a list")
        transcript: list[tuple[str, str]] = []
        for entry in transcript_raw:
            if not isinstance(entry, (list, tuple)) or len(entry) != 2:
                raise ValueError("transcript entries must be [speaker, text] pairs")
            speaker, text = entry
            if not isinstance(speaker, str) or not isinstance(text, str):
                raise ValueError("transcript entries must be strings")
            transcript.append((speaker, text))
        privacy = payload.get("privacy_policy_id", "")
        if not isinstance(privacy, str):
            raise ValueError("privacy_policy_id must be a string")
        tenant = payload.get("tenant_id")
        if tenant is not None and not isinstance(tenant, str):
            raise ValueError("tenant_id must be a string or null")
        correlation = payload.get("correlation_id")
        if correlation is not None and not isinstance(correlation, str):
            raise ValueError("correlation_id must be a string or null")
        return cls(
            session_id=session_id,
            principal_id=principal_id,
            endpoint_kind=endpoint,
            state=state,
            objective=objective,
            transcript=transcript,
            privacy_policy_id=privacy,
            tenant_id=tenant,
            correlation_id=correlation,
        )
