#!/usr/bin/env python3
"""EP-021 M3 wake model training: real features -> real commercial-safe weights.

Trains a wake detector for the phrase "hey nexus" on real openwakeword
feature embeddings computed from the Kokoro-synthesized fixtures. The
weights are Nexus-owned (Apache-2.0) so SPEC-019 is preserved: no
noncommercial upstream openwakeword weights are used or shipped.

The exported ONNX model matches the openwakeword engine contract:
input [1, 16, 96] float32 embeddings, output [1, 1] float32 score.
The real engine is then exercised in streaming mode over every fixture
and the separation is asserted (pos >= 0.7, neg < 0.3).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import wave

# Pin BLAS/OpenMP threads so LogisticRegression training and the ONNX
# export are byte-reproducible across runs (multi-threaded BLAS reduces
# float non-deterministically on multi-core hosts).
os.environ.setdefault("OMP_NUM_THREADS", "1")
os.environ.setdefault("OPENBLAS_NUM_THREADS", "1")
os.environ.setdefault("MKL_NUM_THREADS", "1")

import numpy as np  # noqa: E402
from openwakeword.utils import AudioFeatures  # noqa: E402

N_FRAMES = 16


def load_int16(path: str) -> np.ndarray:
    with wave.open(path, "rb") as w:
        sr = w.getframerate()
        raw = w.readframes(w.getnframes())
    x = np.frombuffer(raw, dtype=np.int16)
    if sr != 16000:
        n = int(len(x) * 16000 / sr)
        x = np.interp(
            np.linspace(0, len(x) - 1, n), np.arange(len(x)), x.astype(np.float64)
        ).astype(np.int16)
    return x


def phrase_region(x: np.ndarray, pad: int = 1600) -> tuple[int, int]:
    idx = np.where(np.abs(x.astype(np.int32)) > 500)[0]
    if len(idx) == 0:
        return (0, len(x))
    return (max(0, idx[0] - pad), min(len(x), idx[-1] + pad))


def features_for(path: str, positive: bool) -> tuple[list[np.ndarray], list[int]]:
    x = load_int16(path)
    region = phrase_region(x) if positive else None
    af = AudioFeatures()
    feats: list[np.ndarray] = []
    labs: list[int] = []
    step = 1280
    for i in range(0, len(x) - step + 1, step):
        af(x[i : i + step])
        feats.append(af.get_features(N_FRAMES))
        if positive:
            assert region is not None
            wstart = max(0, i + step - 2 * step)
            wend = i + step
            labs.append(1 if (wend > region[0] and wstart < region[1]) else 0)
        else:
            labs.append(0)
    return feats, labs


def build_onnx(clf, X) -> bytes:
    from onnx import TensorProto, helper
    from skl2onnx import convert_sklearn
    from skl2onnx.common.data_types import FloatTensorType

    model = convert_sklearn(
        clf,
        initial_types=[("input", FloatTensorType([None, X.shape[1]]))],
        options={id(clf): {"zipmap": False}},
        target_opset=17,
    )
    nodes = list(model.graph.node)
    first = nodes[0]
    for k in range(len(first.input)):
        if first.input[k] == "input":
            first.input[k] = "flat_in"
    nodes.insert(0, helper.make_node("Flatten", ["input"], ["flat_in"], axis=1))
    proba_name = None
    for o in model.graph.output:
        if o.type.tensor_type.elem_type == TensorProto.FLOAT:
            proba_name = o.name
    assert proba_name, "no float proba output"
    nodes.append(helper.make_node("Slice", [proba_name, "st", "en", "ax"], ["nexus_wake_score"]))
    g = model.graph
    del g.node[:]
    g.node.extend(nodes)
    del g.input[0].type.tensor_type.shape.dim[:]
    d0 = g.input[0].type.tensor_type.shape.dim.add()
    d0.dim_param = "None"
    d1 = g.input[0].type.tensor_type.shape.dim.add()
    d1.dim_value = N_FRAMES
    d2 = g.input[0].type.tensor_type.shape.dim.add()
    d2.dim_value = 96
    del g.output[:]
    g.output.append(helper.make_tensor_value_info("nexus_wake_score", TensorProto.FLOAT, [1, 1]))
    g.initializer.extend(
        [
            helper.make_tensor("st", TensorProto.INT64, [1], [1]),
            helper.make_tensor("en", TensorProto.INT64, [1], [2]),
            helper.make_tensor("ax", TensorProto.INT64, [1], [1]),
        ]
    )
    return model.SerializeToString()


def engine_stream_score(path: str, model_path: str) -> float:
    from openwakeword.model import Model as WakeModel

    x = load_int16(path)
    engine = WakeModel(
        wakeword_model_paths=[model_path],
        class_mapping_dicts=[{"0": "negative", "1": "nexus"}],
    )
    step = 1280
    best = 0.0
    for i in range(0, len(x) - step + 1, step):
        pred = engine.predict(x[i : i + step])
        best = max(best, max(pred.values()))
    return float(best)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--fixtures", default=os.environ.get("NEXUS_VOICE_FIXTURES", "/opt/nexus-voice-fixtures")
    )
    ap.add_argument(
        "--models", default=os.environ.get("NEXUS_VOICE_MODELS", "/opt/nexus-voice-models")
    )
    ap.add_argument("--out", default="nexus_wake_hey_nexus_v1.onnx")
    args = ap.parse_args()
    os.makedirs(args.models, exist_ok=True)

    pos = sorted(
        p for p in os.listdir(args.fixtures) if p.startswith("wake_pos_") and p.endswith(".wav")
    )
    neg = sorted(
        p for p in os.listdir(args.fixtures) if p.startswith("wake_neg_") and p.endswith(".wav")
    )
    assert len(pos) >= 4 and len(neg) >= 4, f"fixtures missing: pos={len(pos)} neg={len(neg)}"

    X_parts: list[np.ndarray] = []
    y_parts: list[int] = []
    for p in pos:
        feats, labs = features_for(os.path.join(args.fixtures, p), True)
        X_parts += feats
        y_parts += labs
    for p in neg:
        feats, labs = features_for(os.path.join(args.fixtures, p), False)
        X_parts += feats
        y_parts += labs
    for p in ("silence.wav", "noise.wav"):
        feats, labs = features_for(os.path.join(args.fixtures, p), False)
        X_parts += feats
        y_parts += labs
    X = np.concatenate(X_parts).reshape(len(X_parts), -1).astype(np.float32)
    y = np.asarray(y_parts, dtype=np.int64)

    from sklearn.linear_model import LogisticRegression

    clf = LogisticRegression(max_iter=3000, C=1.0)
    clf.fit(X, y)
    train_acc = float(clf.score(X, y))

    out_path = os.path.join(args.models, args.out)
    with open(out_path, "wb") as f:
        f.write(build_onnx(clf, X))
    with open(out_path, "rb") as f:
        digest = hashlib.sha256(f.read()).hexdigest()

    pos_scores = [engine_stream_score(os.path.join(args.fixtures, p), out_path) for p in pos]
    neg_scores = [engine_stream_score(os.path.join(args.fixtures, p), out_path) for p in neg]
    neg_scores += [
        engine_stream_score(os.path.join(args.fixtures, p), out_path)
        for p in ("silence.wav", "noise.wav")
    ]
    pos_min = min(pos_scores)
    neg_max = max(neg_scores)
    if not (pos_min >= 0.7 and neg_max < 0.3):
        raise SystemExit(f"wake separation failed: pos_min={pos_min:.3f} neg_max={neg_max:.3f}")
    print(
        json.dumps(
            {
                "model": out_path,
                "sha256": digest,
                "train_acc": round(train_acc, 4),
                "train_samples": int(len(y)),
                "pos_clips": len(pos),
                "neg_clips": len(neg) + 2,
                "pos_min": round(pos_min, 4),
                "neg_max": round(neg_max, 4),
                "license": ("Apache-2.0 (Nexus-owned weights; commercial use permitted)"),
                "provenance": (
                    "trained from Kokoro-synthesized controlled fixtures via "
                    "openwakeword feature frontend"
                ),
                "engine_contract": {"input": "[1, 16, 96]", "output": "[1, 1]"},
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
