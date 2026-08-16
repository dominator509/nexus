"""EP-021 audio privacy policy contract (SPEC-012 required behaviors 4, 9).

Raw room audio is ephemeral by default and never continuously streamed
to cloud (SPEC-012 behavior 4). Hardware mute and shared-room privacy
states propagate to policy (SPEC-012 behavior 9; node contract).
"""

from __future__ import annotations

from dataclasses import dataclass

PRIVACY_ZONES = ("PRIVATE", "SHARED_ROOM", "PUBLIC")


class PrivacyZone:
    """Canonical room privacy zone."""

    Private = "PRIVATE"
    SharedRoom = "SHARED_ROOM"
    Public = "PUBLIC"


def _require_zone(value: str) -> str:
    if value not in PRIVACY_ZONES:
        raise ValueError(f"unknown privacy zone: {value}")
    return value


@dataclass(frozen=True)
class AudioPrivacyPolicy:
    """Audio privacy policy governing one session.

    Attributes:
        policy_id: stable policy identity.
        ephemeral_by_default: raw audio is ephemeral unless explicitly
            retained (SPEC-012 behavior 4).
        allow_cloud_streaming: whether any audio may leave the local
            edge (always False when the provider is local-first).
        hardware_mute_enforced: a hardware mute signal must reach the
            policy and suppress capture (SPEC-012 behavior 9).
        shared_room: shared-room privacy state (SPEC-012 behavior 9).
        retention_seconds: retention window for retained audio; 0 when
            ephemeral only.
    """

    policy_id: str
    ephemeral_by_default: bool = True
    allow_cloud_streaming: bool = False
    hardware_mute_enforced: bool = True
    shared_room: bool = False
    retention_seconds: int = 0
    zone: str = PrivacyZone.Private

    def __post_init__(self) -> None:
        if not self.policy_id:
            raise ValueError("policy_id must not be empty")
        if self.retention_seconds < 0:
            raise ValueError("retention_seconds must be non-negative")
        object.__setattr__(self, "zone", _require_zone(self.zone))
        if self.ephemeral_by_default and self.retention_seconds != 0:
            raise ValueError("ephemeral policy must not carry retention")

    def apply_hardware_mute(self, muted: bool) -> AudioPrivacyPolicy:
        """Propagate a hardware mute state into the policy.

        A hardware mute always yields the most restrictive policy:
        capture suppressed, no cloud streaming, no retention. Mute is
        authoritative regardless of room zone (SPEC-012 behavior 9).
        """
        if not muted:
            return self
        return AudioPrivacyPolicy(
            policy_id=self.policy_id,
            ephemeral_by_default=True,
            allow_cloud_streaming=False,
            hardware_mute_enforced=True,
            shared_room=False,
            retention_seconds=0,
            zone=PrivacyZone.Private,
        )

    def apply_shared_room(self, shared: bool) -> AudioPrivacyPolicy:
        """Propagate a shared-room privacy state into the policy.

        Shared-room sessions forbid cloud streaming and any persistent
        retention unless an explicit policy override exists.
        """
        if shared:
            return AudioPrivacyPolicy(
                policy_id=self.policy_id,
                ephemeral_by_default=True,
                allow_cloud_streaming=False,
                hardware_mute_enforced=self.hardware_mute_enforced,
                shared_room=True,
                retention_seconds=0,
                zone=PrivacyZone.SharedRoom,
            )
        return self

    def to_dict(self) -> dict[str, object]:
        """Versioned serialization."""
        return {
            "schema": "nexus.voice.privacy_policy.v1",
            "policy_id": self.policy_id,
            "ephemeral_by_default": self.ephemeral_by_default,
            "allow_cloud_streaming": self.allow_cloud_streaming,
            "hardware_mute_enforced": self.hardware_mute_enforced,
            "shared_room": self.shared_room,
            "retention_seconds": self.retention_seconds,
            "zone": self.zone,
        }

    @classmethod
    def from_dict(cls, payload: dict[str, object]) -> AudioPrivacyPolicy:
        """Deserialize a versioned dict; unknown schema versions rejected."""
        schema = payload.get("schema")
        if schema != "nexus.voice.privacy_policy.v1":
            raise ValueError(f"unknown privacy policy schema: {schema!r}")
        policy_id = payload["policy_id"]
        if not isinstance(policy_id, str):
            raise ValueError("policy_id must be a string")
        ephemeral = payload.get("ephemeral_by_default", True)
        if not isinstance(ephemeral, bool):
            raise ValueError("ephemeral_by_default must be a boolean")
        cloud = payload.get("allow_cloud_streaming", False)
        if not isinstance(cloud, bool):
            raise ValueError("allow_cloud_streaming must be a boolean")
        mute = payload.get("hardware_mute_enforced", True)
        if not isinstance(mute, bool):
            raise ValueError("hardware_mute_enforced must be a boolean")
        shared = payload.get("shared_room", False)
        if not isinstance(shared, bool):
            raise ValueError("shared_room must be a boolean")
        retention = payload.get("retention_seconds", 0)
        if not isinstance(retention, int):
            raise ValueError("retention_seconds must be an integer")
        zone = payload.get("zone", "PRIVATE")
        if not isinstance(zone, str):
            raise ValueError("zone must be a string")
        return cls(
            policy_id=policy_id,
            ephemeral_by_default=ephemeral,
            allow_cloud_streaming=cloud,
            hardware_mute_enforced=mute,
            shared_room=shared,
            retention_seconds=retention,
            zone=zone,
        )
