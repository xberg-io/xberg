"""Exercise the observed Conv1D shape topology without real model payloads."""

import copy

import numpy as np
import onnx
import onnxruntime as ort
import pytest
from onnx import TensorProto, helper

from subblock_fixtures import MULTIPLIERS, SCOPE, STAGES, feature, tiny_graph

ROLES = (
    ("attn/c_attn", 12),
    ("attn/c_proj", 4),
    ("crossattention/q_attn", 4),
    ("crossattention/c_proj", 4),
    ("mlp/c_fc", 16),
    ("mlp/c_proj", 4),
)


def shape_nodes(scope, source, width, explicit_step):
    """Encode Conv1D view(-1, x.size(-1)) and the original size_out tuple."""
    nodes = []

    def node(kind, inputs, label, **attributes):
        name = f"{scope}/shape/{label}"
        nodes.append(helper.make_node(kind, inputs, [name], name=name, **attributes))
        return name

    def constant(label, values, scalar=False):
        value = helper.make_tensor(label, TensorProto.INT64, [] if scalar else [len(values)], values)
        return node("Constant", [], label, value=value)

    axes = constant("axes", [0])
    shape = node("Shape", [source], "width_shape")
    starts = constant("starts", [-1])
    ends = constant("ends", [9223372036854775807])
    inputs = [shape, starts, ends, axes]
    if explicit_step:
        inputs.append(constant("steps", [1]))
    last = node("Slice", inputs, "slice")
    scalar = node("Squeeze", [last, axes], "squeeze")
    vector = node("Unsqueeze", [scalar, axes], "unsqueeze_width")
    minus = constant("minus", [-1])
    flatten = node("Concat", [minus, vector], "flatten", axis=0)
    parts = []
    for index, label in enumerate(("batch", "tokens")):
        size = node("Shape", [source], f"{label}_shape")
        selected = node("Gather", [size, constant(f"{label}_index", [index], scalar=True)], label, axis=0)
        parts.append(node("Unsqueeze", [selected, axes], f"unsqueeze_{label}"))
    restore = node("Concat", [*parts, constant("output_width", [width])], "restore", axis=0)
    return nodes, flatten, restore


def dynamic_shape_model(explicit_step=False):
    model = tiny_graph()
    for role, width in ROLES:
        scope = f"{SCOPE}/{role}"
        flat = next(node for node in model.graph.node if node.name == f"{scope}/Reshape")
        final = next(node for node in model.graph.node if node.name == f"{scope}/Reshape_1")
        nodes, flatten, restore = shape_nodes(scope, flat.input[0], width, explicit_step)
        flat.input[1] = flatten
        final.input[1] = restore
        ordered = []
        for current in model.graph.node:
            if current.name == flat.name:
                ordered.extend(nodes)
            ordered.append(current)
        del model.graph.node[:]
        model.graph.node.extend(ordered)
    onnx.checker.check_model(model)
    assert sum(node.name.endswith("/shape/slice") for node in model.graph.node) == 6
    assert sum(node.name.endswith("/shape/squeeze") for node in model.graph.node) == 6
    return model


def find(model, label):
    matches = [node for node in model.graph.node if node.name == f"{SCOPE}/attn/c_attn/shape/{label}"]
    assert len(matches) == 1
    return matches[0]


def replace_constant(model, label, values):
    node = find(model, label)
    node.attribute[0].t.CopyFrom(helper.make_tensor(label, TensorProto.INT64, [len(values)], values))


@pytest.mark.parametrize("explicit_step", [False, True], ids=["implicit-unit-step", "explicit-unit-step"])
def test_dynamic_slice_squeeze_shape_program_resolves_all_six_conv_roles(monkeypatch, explicit_step):
    original = tiny_graph()
    model = dynamic_shape_model(explicit_step)
    options = ort.SessionOptions()
    options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_DISABLE_ALL
    options.intra_op_num_threads = options.inter_op_num_threads = 1
    baseline = ort.InferenceSession(original.SerializeToString(), options, providers=["CPUExecutionProvider"])
    dynamic = ort.InferenceSession(model.SerializeToString(), options, providers=["CPUExecutionProvider"])
    outputs = 0
    for length in (1, 2):
        feed = {
            "input_ids": np.zeros((1, length), dtype=np.int64),
            "encoder_hidden_states": np.full((1, 2, 4), 0.25, dtype=np.float32),
        }
        expected = baseline.run(["logits"], feed)[0]
        actual = dynamic.run(["logits"], feed)[0]
        assert expected.shape == actual.shape == (1, length, 7) and actual.size == length * 7
        assert expected.tobytes() == actual.tobytes(), "shape topology fixture changed computation"
        outputs += 2
    assert outputs == 4
    api = feature(monkeypatch, required=("resolve_taps",))
    taps = api.resolve_taps(model, hidden_size=4)
    assert [tap["stage"] for tap in taps] == list(STAGES)
    assert [tap["tensor"] for tap in taps] == list(STAGES)
    assert [tap["width"] for tap in taps] == [4 * width for width in MULTIPLIERS]


@pytest.mark.parametrize(
    ("broken", "reason"),
    [
        ("slice_interface", r"(?i)Slice.*(input|interface)"),
        ("slice_axis", r"(?i)Slice.*axis"),
        ("slice_step", r"(?i)Slice.*step"),
        ("squeeze_axis", r"(?i)Squeeze.*axis"),
        ("non_singleton", r"(?i)(Slice|Squeeze).*(single|length|size)"),
    ],
)
def test_dynamic_shape_program_rejects_malformed_or_unsupported_roles(monkeypatch, broken, reason):
    model = dynamic_shape_model(explicit_step=True)
    if broken == "slice_interface":
        del find(model, "slice").input[2:]
    elif broken == "slice_axis":
        name = find(model, "slice").input[3]
        value = helper.make_tensor("bad-axis", TensorProto.INT64, [1], [1])
        model.graph.node.insert(0, helper.make_node("Constant", [], [name + ".bad"], value=value))
        find(model, "slice").input[3] = name + ".bad"
    elif broken == "slice_step":
        replace_constant(model, "steps", [2])
    elif broken == "squeeze_axis":
        name = find(model, "squeeze").input[1]
        value = helper.make_tensor("bad-squeeze-axis", TensorProto.INT64, [1], [1])
        model.graph.node.insert(0, helper.make_node("Constant", [], [name + ".bad"], value=value))
        find(model, "squeeze").input[1] = name + ".bad"
    else:
        replace_constant(model, "starts", [0])
        replace_constant(model, "ends", [2])
    api = feature(monkeypatch, required=("resolve_taps",))
    with pytest.raises(ValueError, match=reason) as error:
        api.resolve_taps(model, hidden_size=4)
    assert "self_qkv" in str(error.value)


def test_dynamic_shape_program_rejects_a_foreign_tensor_shape_source(monkeypatch):
    model = dynamic_shape_model()
    find(model, "width_shape").input[0] = "encoder_hidden_states"
    onnx.checker.check_model(model)
    api = feature(monkeypatch, required=("resolve_taps",))
    with pytest.raises(ValueError, match=r"(?i)shape source") as error:
        api.resolve_taps(model, hidden_size=4)
    assert "self_qkv" in str(error.value)


def test_dynamic_shape_program_rejects_ambiguous_producer_without_first_match(monkeypatch):
    model = dynamic_shape_model()
    duplicate = copy.deepcopy(find(model, "slice"))
    duplicate.name += ".duplicate"
    model.graph.node.append(duplicate)
    assert sum(list(node.output) == list(duplicate.output) for node in model.graph.node) == 2
    api = feature(monkeypatch, required=("resolve_taps",))
    with pytest.raises(ValueError, match=r"(?i)duplicate.*producer"):
        api.resolve_taps(model, hidden_size=4)
