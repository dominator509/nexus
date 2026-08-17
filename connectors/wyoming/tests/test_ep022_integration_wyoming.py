#!/usr/bin/env python3
"""EP-022 M3 Wyoming protocol integration tests (SPEC-012).

These tests run against the REAL rhasspy/wyoming-openwakeword container
(digest sha256:52cb1168731a1849fc28cf339c935fde58746bbabc94226668a40ef6ddf5d42b)
over the real Wyoming protocol (client wyoming==1.10.0). No mocks of the
server: every detection is a real event from the real container. Audio
fixtures are real Kokoro-generated WAV files under tests/fixtures/.

The tests require the container to be running on 127.0.0.1:10400. The
gate script (scripts/ep022-m3-tests.sh) starts/waits for the container
and tears it down; the test module itself only talks to the wire.
"""

import asyncio
import os
import sys
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from connector import WyomingDetection, WyomingSession  # noqa: E402

HOST = os.environ.get("NEXUS_WYOMING_HOST", "127.0.0.1")
PORT = int(os.environ.get("NEXUS_WYOMING_PORT", "10400"))
URI = f"tcp://{HOST}:{PORT}"
FIXTURES = Path(__file__).resolve().parent / "fixtures"


def _fixture(name: str) -> Path:
    return FIXTURES / name


class Ep022WyomingIntegration(unittest.TestCase):
    """Real protocol integration against the real container.

    Every test runs the full session lifecycle (connect -> operation ->
    close) inside ONE asyncio loop so transports are never crossed
    between loops.
    """

    def test_ep022_integration_describe_handshake(self) -> None:
        """Canonical describe/info handshake returns the real server."""
        async def scenario() -> None:
            session = WyomingSession(uri=URI, timeout_seconds=5.0)
            try:
                info = await session.connect()
                self.assertTrue(info.wake, "server advertises wake programs")
                names = {p.name for p in info.wake}
                self.assertIn("openwakeword", names)
                self.assertTrue(
                    any(m.installed for p in info.wake for m in p.models),
                    "at least one installed wake model",
                )
            finally:
                await session.close()

        asyncio.run(scenario())

    def test_ep022_integration_real_wake_detection(self) -> None:
        """Real audio -> real protocol -> real Detection event."""
        async def scenario() -> None:
            session = WyomingSession(uri=URI, timeout_seconds=5.0)
            try:
                info = await session.connect()
                # Wake model names (e.g. hey_jarvis), not program names.
                wake_names = [m.name for p in info.wake for m in p.models]
                detections: list[WyomingDetection] = await session.stream_audio(
                    str(_fixture("hey-jarvis.wav")), wake_names=wake_names
                )
                self.assertTrue(detections, "expected at least one real detection")
                self.assertIn("hey_jarvis", {d.name for d in detections})
            finally:
                await session.close()

        asyncio.run(scenario())

    def test_ep022_integration_not_detected_negative(self) -> None:
        """Silence produces no detection (real negative, never fabricated)."""
        async def scenario() -> None:
            session = WyomingSession(uri=URI, timeout_seconds=5.0)
            try:
                await session.connect()
                detections: list[WyomingDetection] = await session.stream_audio(
                    str(_fixture("silence.wav"))
                )
                self.assertEqual(detections, [])
            finally:
                await session.close()

        asyncio.run(scenario())

    def test_ep022_integration_connect_refused_fails_fast(self) -> None:
        """A dead server fails closed quickly (connection refused)."""
        async def scenario() -> None:
            dead = WyomingSession(uri="tcp://127.0.0.1:1", timeout_seconds=2.0)
            start = time.monotonic()
            with self.assertRaises(Exception):
                await dead.connect()
            self.assertLess(time.monotonic() - start, 10.0)

        asyncio.run(scenario())


if __name__ == "__main__":
    unittest.main(verbosity=2)
