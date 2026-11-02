"""Prospective subblock graph contracts; no real model or qualification assets."""

import copy

import numpy as np
import onnx
import onnxruntime as ort
import pytest
from onnx import TensorProto, helper

from subblock_fixtures import MULTIPLIERS, SCOPE, STAGES, feature, tiny_graph


def resolve(api, model):
    return api.resolve_taps(model, hidden_size=4)


def test_resolves_sixteen_distinct_scoped_stages_in_prospective_order(monkeypatch):
    model = tiny_graph()
    api = feature(monkeypatch, required=("resolve_taps",))
    taps = resolve(api, model)
    assert [tap["stage"] for tap in taps] == list(STAGES)
    assert [tap["tensor"] for tap in taps] == list(STAGES)
    assert [tap["width"] for tap in taps] == [4 * width for width in MULTIPLIERS]
    assert len({tap["tensor"] for tap in taps}) == 16


@pytest.mark.parametrize(
    ("stage", "scope"),
    [
        ("ln1", "ln_1/LayerNormalization"),
        ("cross_norm", "ln_cross_attn/LayerNormalization"),
        ("mlp_fc", "mlp/c_fc/Gemm"),
        ("self_projection", "attn/c_proj/Gemm"),
    ],
)
def test_missing_scope_fails_at_its_named_stage(monkeypatch, stage, scope):
    model = tiny_graph()
    matches = [node for node in model.graph.node if node.name == f"{SCOPE}/{scope}"]
    assert len(matches) == 1
    matches[0].name = f"/unrelated/{scope}"
    onnx.checker.check_model(model)
    api = feature(monkeypatch, required=("resolve_taps",))
    with pytest.raises(ValueError, match=r"(?i)missing") as error:
        resolve(api, model)
    assert stage in str(error.value)


@pytest.mark.parametrize(
    ("scope", "stage"),
    [
        ("ln_1/LayerNormalization", "ln1"),
        ("attn/c_attn/Gemm", "self_qkv"),
    ],
)
def test_duplicate_scoped_operator_is_not_resolved_by_first_match(monkeypatch, scope, stage):
    model = tiny_graph()
    matches = [node for node in model.graph.node if node.name == f"{SCOPE}/{scope}"]
    assert len(matches) == 1
    duplicate = copy.deepcopy(matches[0])
    duplicate.name += "_1"
    duplicate.output[0] += ".duplicate"
    model.graph.node.append(duplicate)
    onnx.checker.check_model(model)
    api = feature(monkeypatch, required=("resolve_taps",))
    with pytest.raises(ValueError, match=r"(?i)(duplicate|ambiguous)") as error:
        resolve(api, model)
    assert stage in str(error.value)


@pytest.mark.parametrize("broken", ["transpose", "weight_shape", "flatten_producer", "disconnected_reshape"])
def test_conv1d_resolution_rejects_wrong_roles_or_dataflow(monkeypatch, broken):
    model = tiny_graph()
    gemm = next(node for node in model.graph.node if node.name == f"{SCOPE}/attn/c_proj/Gemm")
    final = next(node for node in model.graph.node if node.name == f"{SCOPE}/attn/c_proj/Reshape_1")
    if broken == "transpose":
        gemm.attribute.append(helper.make_attribute("transB", 1))
    elif broken == "weight_shape":
        weight = next(value for value in model.graph.initializer if value.name == gemm.input[1])
        weight.dims[:] = [2, 8]
    elif broken == "flatten_producer":
        gemm.input[0] = "ln1"
    else:
        final.input[0] = "self_qkv.gemm"
    onnx.checker.check_model(model)
    api = feature(monkeypatch, required=("resolve_taps",))
    with pytest.raises(ValueError, match=r"(?i)(unsupported|shape|transpose|reshape|dataflow|initializer)") as error:
        resolve(api, model)
    assert "self_projection" in str(error.value)


def test_seventeen_typed_outlets_preserve_whole_original_graph_and_ort_logits(monkeypatch, tmp_path):
    model = tiny_graph()
    source = tmp_path / "original.onnx"
    onnx.save_model(model, source)
    original_bytes = source.read_bytes()
    api = feature(monkeypatch, required=("resolve_taps", "instrument_graph"))
    taps = resolve(api, model)
    tapped = api.instrument_graph(model, taps)
    onnx.checker.check_model(tapped)
    assert len(tapped.graph.output) == 17
    assert [value.name for value in tapped.graph.output] == [*STAGES, "logits"]
    for output, multiplier in zip(tapped.graph.output[:16], MULTIPLIERS, strict=True):
        tensor = output.type.tensor_type
        assert tensor.elem_type == TensorProto.FLOAT
        assert tensor.shape.dim[0].dim_value == 1
        assert tensor.shape.dim[1].dim_param
        assert tensor.shape.dim[2].dim_value == 4 * multiplier
    restored = copy.deepcopy(tapped)
    del restored.graph.output[:]
    restored.graph.output.extend(model.graph.output)
    assert restored.SerializeToString() == model.SerializeToString() == original_bytes
    assert source.read_bytes() == original_bytes
    options = ort.SessionOptions()
    options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_DISABLE_ALL
    options.execution_mode = ort.ExecutionMode.ORT_SEQUENTIAL
    options.intra_op_num_threads = options.inter_op_num_threads = 1
    baseline = ort.InferenceSession(original_bytes, options, providers=["CPUExecutionProvider"])
    observed = ort.InferenceSession(tapped.SerializeToString(), options, providers=["CPUExecutionProvider"])
    for length in (1, 2):
        feed = {
            "input_ids": np.zeros((1, length), dtype=np.int64),
            "encoder_hidden_states": np.full((1, 2, 4), 0.25, dtype=np.float32),
        }
        expected = baseline.run(["logits"], feed)[0]
        actual = observed.run(["logits"], feed)[0]
        assert expected.shape == actual.shape == (1, length, 7) and expected.size > 0
        assert expected.tobytes() == actual.tobytes()
    assert source.read_bytes() == original_bytes
