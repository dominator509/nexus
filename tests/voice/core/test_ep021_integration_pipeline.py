"""EP-021 M3 composed pipeline tests (directive K).

Proves the bounded real chains:
  audio -> Silero VAD -> wake detection -> captured utterance
        -> whisper.cpp transcription
  text -> Kokoro -> generated audio
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_ROOT = REPO_ROOT / "python"
for _root in (REPO_ROOT, PYTHON_ROOT):
    if str(_root) not in sys.path:
        sys.path.insert(0, str(_root))

import pytest  # noqa: E402

from infra.voice.adapters import pipeline  # noqa: E402
from infra.voice.engine_env import FIXTURES_DIR, VoiceEngineError, require_models  # noqa: E402


@pytest.fixture(autouse=True)
def _engines_present() -> None:
    require_models()


def test_ep021_integration_chain_wake_to_transcription() -> None:
    chain = FIXTURES_DIR / "chain_full.wav"
    if not chain.exists():
        raise VoiceEngineError(f"chain fixture missing: {chain}")
    result = pipeline.run_chain(str(chain))
    assert result["wake_detected"] is True
    assert result["wake_score"] >= 0.7
    assert len(result["vad_segments"]) >= 2
    assert result["wake_trigger_seconds"] is not None
    assert result["utterance_span"] is not None
    assert "turn on the lights" in result["transcript"].lower()


def test_ep021_integration_route_shared_room_private() -> None:
    # SPEC-012 behavior 9: shared-room privacy propagates to the real
    # policy and routes sensitive responses privately (not spoken).
    from nexus_voice import AudioPrivacyPolicy, PrivacyZone

    from infra.voice.adapters.pipeline import route_response

    private = AudioPrivacyPolicy(
        policy_id="route-test",
        zone=PrivacyZone.Private,
        hardware_mute_enforced=False,
    )
    shared = private.apply_shared_room(True)
    assert shared.shared_room is True
    assert shared.zone == PrivacyZone.SharedRoom
    assert shared.allow_cloud_streaming is False

    shared_route = route_response("sensitive answer", shared, sensitive=True)
    assert shared_route["channel"] == "PRIVATE"
    assert shared_route["audible"] is False
    assert shared_route["reason"] == "shared_room_sensitive"

    private_route = route_response("sensitive answer", private, sensitive=True)
    assert private_route["channel"] == "SPOKEN"
    assert private_route["audible"] is True

    muted = private.apply_hardware_mute(True)
    assert muted.zone == PrivacyZone.Private
    muted_route = route_response("anything", muted, sensitive=True)
    assert muted_route["channel"] == "SUPPRESSED"
    assert muted_route["audible"] is False


def test_ep021_integration_chain_text_to_generated_audio() -> None:
    with tempfile.TemporaryDirectory() as td:
        out = str(Path(td) / "generated.wav")
        result = pipeline.synthesize("hello from nexus", out)
        assert result["sample_rate_hz"] == 24000
        assert result["duration_seconds"] >= 1.0
        assert result["rms"] > 0.01
        props = pipeline.wav_properties(out)
        assert props["sample_rate_hz"] == 24000
        assert props["seconds"] >= 1.0
