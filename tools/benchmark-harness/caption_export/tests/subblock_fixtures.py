"""Small computational graph fixtures, not exports or caption-quality evidence."""

import importlib
from pathlib import Path

import numpy as np
import onnx
import pytest
from onnx import TensorProto, helper

SCOPE = "/decoder/transformer/h.0"
STAGES = (
    "block_input",
    "ln1",
    "self_qkv",
    "self_context",
    "self_projection",
    "self_residual",
    "cross_norm",
    "cross_query",
    "cross_context",
    "cross_projection",
    "cross_residual",
    "ln2",
    "mlp_fc",
    "mlp_activation",
    "mlp_projection",
    "block_output",
)
MULTIPLIERS = (1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4, 4, 1, 1)


def feature(monkeypatch, module="subblock", required=()):
    project = Path(__file__).resolve().parents[1]
    monkeypatch.syspath_prepend(str(project / "src"))
    name = f"caption_export.{module}"
    try:
        result = importlib.import_module(name)
    except ModuleNotFoundError as error:
        if error.name in {"caption_export", name}:
            pytest.fail(f"NEWFEATUREABSENCE: {name} is not implemented", pytrace=False)
        raise
    assert Path(result.__file__).resolve().parent == project / "src" / "caption_export"
    for function in required:
        if not callable(getattr(result, function, None)):
            pytest.fail(f"NEWFEATUREABSENCE: {name}.{function} is not implemented", pytrace=False)
    return result


class GraphFixture:
    """Build valid scoped Gemm/Reshape edges without pretending to implement GPT2."""

    def __init__(self):
        self.nodes = []
        self.initializers = []

    def tensor(self, name, dims, values, dtype=TensorProto.FLOAT):
        """Declare an inline initializer with an explicit role and dimensions."""
        self.initializers.append(helper.make_tensor(name, dtype, dims, values))
        return name

    def node(self, kind, inputs, output, name, **attributes):
        """Preserve exact producer edges and scoped operator names."""
        self.nodes.append(helper.make_node(kind, inputs, [output], name=name, **attributes))
        return output

    def norm(self, value, output, scope):
        """Expose a real three-input LayerNormalization boundary."""
        scale = self.tensor(f"{output}.scale", [4], [1.0] * 4)
        bias = self.tensor(f"{output}.bias", [4], [0.0] * 4)
        return self.node("LayerNormalization", [value, scale, bias], output, f"{scope}/LayerNormalization")

    def conv(self, value, output, scope, width, result_width):
        """Build the flatten/addmm/restore path used by Conv1D exports."""
        flat = self.tensor(f"{output}.flat_shape", [2], [-1, width], TensorProto.INT64)
        shape = self.tensor(f"{output}.shape", [3], [1, -1, result_width], TensorProto.INT64)
        weights = self.tensor(f"{output}.weight", [width, result_width], [0.025] * (width * result_width))
        bias = self.tensor(f"{output}.bias", [result_width], [0.01] * result_width)
        self.node("Reshape", [value, flat], f"{output}.flat", f"{scope}/Reshape")
        self.node("Gemm", [f"{output}.flat", weights, bias], f"{output}.gemm", f"{scope}/Gemm")
        return self.node("Reshape", [f"{output}.gemm", shape], output, f"{scope}/Reshape_1")


def tiny_graph():
    graph = GraphFixture()
    embedding = graph.tensor("embedding", [7, 4], [0.02 * (index % 9) for index in range(28)])
    graph.node("Gather", [embedding, "input_ids"], "tokens", "/embedding/Gather")
    graph.node("ReduceMean", ["encoder_hidden_states"], "context", "/context/ReduceMean", axes=[1], keepdims=1)
    graph.node("Add", ["tokens", "context"], "block_input", f"{SCOPE}/input/Add")
    graph.norm("block_input", "ln1", f"{SCOPE}/ln_1")
    graph.conv("ln1", "self_qkv", f"{SCOPE}/attn/c_attn", 4, 12)
    indices = [
        graph.tensor(name, [1], [value], TensorProto.INT64)
        for name, value in [("start", 0), ("end", 4), ("axis", 2), ("step", 1)]
    ]
    graph.node("Slice", ["self_qkv", *indices], "self_context", f"{SCOPE}/attn/context/Slice")
    graph.conv("self_context", "self_projection", f"{SCOPE}/attn/c_proj", 4, 4)
    graph.node("Add", ["block_input", "self_projection"], "self_residual", f"{SCOPE}/self/Add")
    graph.norm("self_residual", "cross_norm", f"{SCOPE}/ln_cross_attn")
    graph.conv("cross_norm", "cross_query", f"{SCOPE}/crossattention/q_attn", 4, 4)
    graph.node("Add", ["cross_query", "context"], "cross_context", f"{SCOPE}/crossattention/context/Add")
    graph.conv("cross_context", "cross_projection", f"{SCOPE}/crossattention/c_proj", 4, 4)
    graph.node("Add", ["self_residual", "cross_projection"], "cross_residual", f"{SCOPE}/cross/Add")
    graph.norm("cross_residual", "ln2", f"{SCOPE}/ln_2")
    graph.conv("ln2", "mlp_fc", f"{SCOPE}/mlp/c_fc", 4, 16)
    graph.node("Relu", ["mlp_fc"], "mlp_activation", f"{SCOPE}/mlp/activation/Relu")
    graph.conv("mlp_activation", "mlp_projection", f"{SCOPE}/mlp/c_proj", 16, 4)
    graph.node("Add", ["cross_residual", "mlp_projection"], "block_output", f"{SCOPE}/output/Add")
    graph.norm("block_output", "next_norm", "/decoder/transformer/h.1/ln_1")
    head = graph.tensor("head", [4, 7], [0.03] * 28)
    graph.node("MatMul", ["next_norm", head], "logits", "/decoder/lm_head/MatMul")
    graph.norm("block_input", "decoy", "/other_decoder/transformer/h.0/ln_1")
    model = helper.make_model(
        helper.make_graph(
            graph.nodes,
            "subblock-dataflow-fixture",
            [
                helper.make_tensor_value_info("input_ids", TensorProto.INT64, [1, "tokens"]),
                helper.make_tensor_value_info("encoder_hidden_states", TensorProto.FLOAT, [1, 2, 4]),
            ],
            [helper.make_tensor_value_info("logits", TensorProto.FLOAT, [1, "tokens", 7])],
            initializer=graph.initializers,
        ),
        opset_imports=[helper.make_opsetid("", 17)],
    )
    model.ir_version = 10
    onnx.checker.check_model(model)
    assert len(model.graph.node) >= 30 and len(model.graph.initializer) >= 20
    assert len([node for node in model.graph.node if node.op_type == "Gemm"]) == 6
    return model


def stage_arrays(length):
    result = {
        stage: np.full((1, length, 768 * multiplier), 0.25, dtype=np.float32)
        for stage, multiplier in zip(STAGES, MULTIPLIERS, strict=True)
    }
    assert len(result) == 16
    assert sum(value.size for value in result.values()) == length * 18432
    return result
