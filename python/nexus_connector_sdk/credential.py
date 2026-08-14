"""Credential broker boundary (SPEC-022 behavior 7, SPEC-020) -
Python binding.

Connectors never receive generic credentials and never embed secrets
in prompts or manifests. The ``CredentialBroker`` port hands out
references and resolves references to values only inside the sandbox
at execution time; values never enter logs, prompts, manifests, or
model context.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from .error import NOT_FOUND, UNAVAILABLE, VALIDATION, SdkError


@dataclass
class CredentialReference:
    """A reference to a broker-held credential.

    The reference is the only thing that travels in manifests,
    requests, and telemetry. The value lives in the broker.
    """

    reference: str
    version: str
    fingerprint: str

    def __post_init__(self) -> None:
        if not self.reference or not (
            self.reference.startswith("vault:") or self.reference.startswith("broker:")
        ):
            raise SdkError(
                VALIDATION,
                "credential reference must be namespaced (vault: or broker:)",
                resource=self.reference,
            )
        if not self.version:
            raise SdkError(VALIDATION, "credential version must not be empty")
        if not self.fingerprint:
            raise SdkError(VALIDATION, "credential fingerprint must not be empty")

    def key(self) -> str:
        return self.reference

    def to_dict(self) -> dict[str, Any]:
        return {
            "reference": self.reference,
            "version": self.version,
            "fingerprint": self.fingerprint,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> CredentialReference:
        return cls(
            reference=str(data["reference"]),
            version=str(data["version"]),
            fingerprint=str(data["fingerprint"]),
        )


@dataclass
class TemporaryCredential:
    """Scoped, expiring credential material bound to one invocation.

    The value exists only inside the sandbox for the invocation's
    lifetime; it is cleared after use and never observed through any
    observation surface.
    """

    reference: CredentialReference
    value: str
    scope: str
    expires_at_unix_ms: int

    def is_expired(self, now_unix_ms: int) -> bool:
        return now_unix_ms >= self.expires_at_unix_ms


class CredentialBroker:
    """Port for the credential broker.

    Implementations resolve references to values only inside the
    sandbox at execution time, with least privilege, and never expose
    values through any observation surface. A port without an
    implementation fails closed with a typed UNAVAILABLE error; it
    never returns a fabricated value.
    """

    def resolve(self, reference: CredentialReference) -> str:
        raise SdkError(
            UNAVAILABLE,
            "credential broker port has no provider bound",
            resource=reference.reference,
        )


class InMemoryCredentialBroker(CredentialBroker):
    """Deterministic broker for conformance tests.

    Test/verification zone: values live in a process-local map keyed
    by reference; resolution is scoped to an invocation and the value
    is returned by value (the caller decides how to scope/clear it).
    Never logs values.
    """

    def __init__(self) -> None:
        self._values: dict[str, str] = {}

    def put(self, reference: CredentialReference, value: str) -> None:
        self._values[reference.key()] = value

    def resolve(self, reference: CredentialReference) -> str:
        value = self._values.get(reference.key())
        if value is None:
            raise SdkError(
                NOT_FOUND,
                "credential reference not found",
                resource=reference.reference,
            )
        return value


__all__ = [
    "CredentialBroker",
    "CredentialReference",
    "InMemoryCredentialBroker",
    "TemporaryCredential",
]
