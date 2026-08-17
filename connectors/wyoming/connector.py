#!/usr/bin/env python3
"""EP-022 M3 Wyoming protocol connector (SPEC-012 Wyoming term).

Real client for the Wyoming protocol (rhasspy/wyoming 1.10.0, MIT)
speaking to a REAL Wyoming server. The connector is the provider-neutral
adapter for the nexus-audio WyomingProvider port: connect, describe,
stream audio, and receive wake detections over the canonical protocol.

No audio fabrication: audio arrives as real 16-bit PCM WAV and every
detection is a real event from the server.

Component record (M3 milestone content 6):
  - component: rhasspy/wyoming-openwakeword (server)
  - source: https://github.com/rhasspy/wyoming-openwakeword
  - container: rhasspy/wyoming-openwakeword:latest
  - digest: sha256:52cb1168731a1849fc28cf339c935fde58746bbabc94226668a40ef6ddf5d42b
  - license: Apache-2.0 (upstream classifier); MIT LICENSE.md text
  - protocol client: wyoming==1.10.0 (MIT)
  - replacement contract: WyomingProvider port in crates/nexus-audio;
    any Wyoming-compatible server may replace this one.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import wave
from dataclasses import dataclass, field
from typing import List, Optional

from wyoming.audio import AudioChunk, AudioStart, AudioStop
from wyoming.client import AsyncClient
from wyoming.event import Event
from wyoming.info import Describe, Info
from wyoming.wake import Detect, Detection, NotDetected

DEFAULT_URI = "tcp://127.0.0.1:10400"
DEFAULT_TIMEOUT_SECONDS = 10.0


@dataclass
class WyomingDetection:
    name: str
    timestamp_ms: int


@dataclass
class WyomingSession:
    uri: str
    timeout_seconds: float
    client: Optional[AsyncClient] = field(default=None, init=False)
    _connected: bool = field(default=False, init=False)
    info: Optional[Info] = field(default=None, init=False)

    async def connect(self) -> Info:
        """Connect and perform the canonical describe/info handshake."""
        self.client = AsyncClient.from_uri(self.uri)
        await self.client.connect()
        self._connected = True
        await self.client.write_event(Describe().event())
        event: Event = await asyncio.wait_for(
            self.client.read_event(), timeout=self.timeout_seconds
        )
        if not Info.is_type(event.type):
            raise RuntimeError(f"expected info event, got {event.type}")
        self.info = Info.from_event(event)
        return self.info

    async def stream_audio(
        self,
        wav_path: str,
        wake_names: Optional[List[str]] = None,
    ) -> List[WyomingDetection]:
        """Stream a real WAV through the protocol and collect detections."""
        if self.client is None or not self._connected:
            raise RuntimeError("session is not connected")

        with wave.open(wav_path, "rb") as w:
            rate = w.getframerate()
            width = w.getsampwidth()
            channels = w.getnchannels()
            frames = w.readframes(w.getnframes())

        # Select wake models (canonical Detect request).
        if wake_names:
            await self.client.write_event(Detect(names=wake_names).event())

        # Stream real audio.
        await self.client.write_event(
            AudioStart(rate=rate, width=width, channels=channels).event()
        )
        chunk_size = rate  # 1 second per chunk
        for i in range(0, len(frames), chunk_size * width * channels):
            chunk = frames[i : i + chunk_size * width * channels]
            await self.client.write_event(
                AudioChunk(rate=rate, width=width, channels=channels, audio=chunk).event()
            )
            # Small pacing so the server can process each chunk.
            await asyncio.sleep(0.02)
        await self.client.write_event(AudioStop().event())

        # Collect detection events.
        detections: List[WyomingDetection] = []
        try:
            while True:
                event: Event = await asyncio.wait_for(
                    self.client.read_event(), timeout=self.timeout_seconds
                )
                if Detection.is_type(event.type):
                    d = Detection.from_event(event)
                    detections.append(
                        WyomingDetection(name=d.name, timestamp_ms=d.timestamp)
                    )
                elif NotDetected.is_type(event.type):
                    break
        except asyncio.TimeoutError:
            pass
        return detections

    async def close(self) -> None:
        if self.client is not None and self._connected:
            await self.client.disconnect()
        self._connected = False


async def _run(args: argparse.Namespace) -> int:
    session = WyomingSession(
        uri=args.uri,
        timeout_seconds=args.timeout,
    )
    try:
        info = await session.connect()
        result: dict = {"info": {"name": info.name, "version": info.version}}
        if args.wav:
            detections = await session.stream_audio(
                args.wav,
                wake_names=[args.model] if args.model else None,
            )
            result["detections"] = [
                {"name": d.name, "timestamp_ms": d.timestamp_ms} for d in detections
            ]
        print(json.dumps(result, indent=2))
        return 0
    finally:
        await session.close()


def main() -> int:
    ap = argparse.ArgumentParser(description="Wyoming protocol connector")
    ap.add_argument("--uri", default=DEFAULT_URI)
    ap.add_argument("--wav")
    ap.add_argument("--model")
    ap.add_argument("--timeout", type=float, default=DEFAULT_TIMEOUT_SECONDS)
    args = ap.parse_args()
    return asyncio.run(_run(args))


if __name__ == "__main__":
    raise SystemExit(main())
