#!/usr/bin/env python3
"""LF-028 shared-room private response live-fire (real proof).

Scenario: a sensitive response is requested while the room is occupied
(shared-room privacy state). The proof shows Nexus routes the response
privately instead of speaking it aloud, using the REAL stack:

  1. the real Kokoro engine synthesizes the would-be response audio
     (proving the text is real and synthesizable);
  2. the real AudioPrivacyPolicy carries the shared-room state
     (SPEC-012 behavior 9);
  3. the real router (infra/voice/adapters/pipeline.route_response)
     decides the route from the policy;
  4. the audible channel is suppressed for the shared room and the
     response is delivered on the PRIVATE channel;
  5. a private-zone control proves the suppression is zone-driven.

Emits machine-readable evidence to .agent/state/evidence/.
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
for _root in (REPO_ROOT, PYTHON_ROOT):
    if str(_root) not in sys.path:
        sys.path.insert(0, str(_root))

from nexus_voice import AudioPrivacyPolicy, PrivacyZone  # noqa: E402

from infra.voice.adapters import run_engine  # noqa: E402
from infra.voice.adapters.pipeline import route_response  # noqa: E402

SENSITIVE_RESPONSE = (
    "Your account password is a private secret; it is shown only on your private channel."
)


def main() -> None:
    evidence: dict[str, object] = {}

    # 1. Real synthesis of the would-be spoken response (real Kokoro).
    with tempfile.TemporaryDirectory() as td:
        out = str(Path(td) / "response.wav")
        tts = run_engine(
            "kokoro_worker.py",
            "--text",
            SENSITIVE_RESPONSE,
            "--out",
            out,
        )
        evidence["tts_synthesized"] = {
            "sample_rate_hz": tts["sample_rate_hz"],
            "duration_seconds": tts["duration_seconds"],
            "rms": tts["rms"],
        }

        # 2. Real privacy policy carrying the shared-room state. The
        # room is occupied (shared) with no active hardware mute.
        private_policy = AudioPrivacyPolicy(
            policy_id="lf028-control",
            zone=PrivacyZone.Private,
            hardware_mute_enforced=False,
        )
        shared_policy = private_policy.apply_shared_room(True)
        assert shared_policy.shared_room is True
        assert shared_policy.zone == PrivacyZone.SharedRoom
        assert shared_policy.allow_cloud_streaming is False
        assert shared_policy.retention_seconds == 0

        # 3+4. Real router: shared room + sensitive -> PRIVATE, not spoken.
        shared_route = route_response(SENSITIVE_RESPONSE, shared_policy, sensitive=True)
        private_route = route_response(SENSITIVE_RESPONSE, private_policy, sensitive=True)
        muted_policy = private_policy.apply_hardware_mute(True)
        muted_route = route_response(SENSITIVE_RESPONSE, muted_policy, sensitive=True)

        # 5. Zone-driven control: private room may speak locally.
        evidence["shared_room_route"] = shared_route
        evidence["private_room_route"] = private_route
        evidence["hardware_mute_route"] = muted_route

        if shared_route["channel"] != "PRIVATE" or shared_route["audible"] is not False:
            raise SystemExit(
                f"LF-028 FAIL: shared room response not routed privately: {shared_route}"
            )
        if private_route["channel"] != "SPOKEN" or private_route["audible"] is not True:
            raise SystemExit(
                f"LF-028 FAIL: private-room control did not allow audible route: {private_route}"
            )
        if muted_route["channel"] != "SUPPRESSED" or muted_route["audible"] is not False:
            raise SystemExit(f"LF-028 FAIL: hardware mute did not suppress: {muted_route}")

    evidence_dir = REPO_ROOT / ".agent/state/evidence"
    evidence_dir.mkdir(parents=True, exist_ok=True)
    evidence_path = evidence_dir / "EP-021-M5-LF-028-shared-room-private-response.md"
    with open(evidence_path, "w", encoding="utf-8") as f:
        f.write(
            "# LF-028 shared-room private response (EP-021 M5)\n\n"
            "Real proof: sensitive response in an occupied room is routed privately, "
            "never spoken aloud on the room speaker.\n\n"
            f"```json\n{json.dumps(evidence, indent=2, sort_keys=True)}\n```\n"
        )
    print("LF-028 shared-room private response: PASS")
    print(f"evidence: {evidence_path}")
    print(json.dumps(evidence, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
