"""Resolve existing first-block values without exporting or qualifying a model."""

import copy
import re

import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper

from .first_divergence import compare_taps as compare_existing
from .first_divergence import summarize as summarize_existing
from .first_divergence import unique_norm
from .first_divergence_evidence import require

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
COUNTS = {"cases": 2, "stages": 32, "compared_elements": 55296}
SCOPE = "/decoder/transformer/h.0"


def _one(nodes: list[onnx.NodeProto], stage: str, role: str) -> onnx.NodeProto:
    require(bool(nodes), f"missing {role} for {stage}")
    require(len(nodes) == 1, f"duplicate {role} for {stage}")
    return nodes[0]


def _integers(tensor: onnx.TensorProto) -> list[int]:
    require(tensor.data_type == TensorProto.INT64, "unsupported reshape shape dtype")
    require(len(tensor.dims) <= 1 and int(np.prod(tensor.dims)) <= 3, "unsupported reshape shape rank")
    return [int(value) for value in numpy_helper.to_array(tensor).reshape(-1)]


class _Index:
    def __init__(self, model: onnx.ModelProto) -> None:
        self.nodes = list(model.graph.node)
        self.producers = {}
        for node in self.nodes:
            for name in node.output:
                require(name not in self.producers, "duplicate graph tensor producer")
                self.producers[name] = node
        self.initializers = {value.name: value for value in model.graph.initializer}

    def initializer(self, name: str) -> onnx.TensorProto:
        """Follow exporter parameter aliases, never evaluate an arbitrary operator."""
        visited = set()
        while name not in self.initializers:
            require(name not in visited, "cyclic initializer alias")
            visited.add(name)
            node = self.producers.get(name)
            require(
                node is not None and node.op_type == "Identity" and len(node.input) == 1, "unsupported initializer role"
            )
            name = node.input[0]
        return self.initializers[name]

    def shape(self, name: str, source: str, visited: set[str] | None = None) -> list[object]:
        """Read only small integer shape expressions rooted at this Conv1D input."""
        visited = set() if visited is None else visited
        require(name not in visited, "cyclic reshape shape expression")
        visited = visited | {name}
        if name in self.initializers:
            return _integers(self.initializers[name])
        node = self.producers.get(name)
        require(node is not None, "missing reshape shape producer")
        attributes = {value.name: helper.get_attribute_value(value) for value in node.attribute}
        if node.op_type == "Constant":
            require(set(attributes) == {"value"}, "unsupported reshape Constant")
            return _integers(attributes["value"])
        if node.op_type == "Shape":
            require(list(node.input) == [source] and not attributes, "unsupported reshape shape source")
            return [1, "T", "width"]
        if node.op_type == "Identity":
            require(len(node.input) == 1 and not attributes, "unsupported reshape Identity")
            return self.shape(node.input[0], source, visited)
        if node.op_type == "Unsqueeze":
            require(len(node.input) == 2 and not attributes, "unsupported reshape Unsqueeze")
            values = self.shape(node.input[0], source, visited)
            require(
                len(values) == 1 and self.shape(node.input[1], source, visited) in ([0], [-1]),
                "unsupported reshape Unsqueeze axis/value",
            )
            return values
        if node.op_type == "Concat":
            require(attributes.get("axis", 0) == 0, "unsupported reshape Concat axis")
            return [value for item in node.input for value in self.shape(item, source, visited)]
        require(node.op_type == "Gather" and attributes.get("axis", 0) == 0, "unsupported reshape shape operator")
        values = self.shape(node.input[0], source, visited)
        indices = self.shape(node.input[1], source, visited)
        require(
            all(type(index) is int and -len(values) <= index < len(values) for index in indices),
            "unsupported reshape Gather index",
        )
        return [values[index] for index in indices]


def _conv(index: _Index, stage: str, scope: str, input_width: int, output_width: int) -> tuple[str, str]:
    try:
        pattern = re.compile(re.escape(scope) + r"/Gemm(?:_\d+)?$")
        gemm = _one([n for n in index.nodes if n.op_type == "Gemm" and pattern.fullmatch(n.name)], stage, "Gemm")
        require(len(gemm.input) == 3 and len(gemm.output) == 1, "unsupported Gemm interface")
        attributes = {a.name: helper.get_attribute_value(a) for a in gemm.attribute}
        require(set(attributes) <= {"alpha", "beta", "transA", "transB"}, "unsupported Gemm attributes")
        require(
            attributes.get("alpha", 1.0) == attributes.get("beta", 1.0) == 1.0
            and attributes.get("transA", 0) == attributes.get("transB", 0) == 0,
            "unsupported Gemm transpose/scale",
        )
        weight, bias = (index.initializer(name) for name in gemm.input[1:])
        require(
            list(weight.dims) == [input_width, output_width] and list(bias.dims) == [output_width],
            "unsupported initializer shape",
        )
        require(weight.data_type == bias.data_type == TensorProto.FLOAT, "unsupported initializer dtype")
        flat = index.producers.get(gemm.input[0])
        require(
            flat is not None and flat.op_type == "Reshape" and len(flat.input) == 2,
            "unsupported flatten Reshape producer",
        )
        source = flat.input[0]
        require(
            index.shape(flat.input[1], source) in ([-1, input_width], [-1, "width"]),
            "unsupported flatten Reshape shape",
        )
        pattern = re.compile(re.escape(scope) + r"/Reshape(?:_\d+)?$")
        final = _one(
            [
                n
                for n in index.nodes
                if n.op_type == "Reshape" and pattern.fullmatch(n.name) and list(n.input[:1]) == list(gemm.output)
            ],
            stage,
            "final Reshape",
        )
        require(len(final.input) == 2 and len(final.output) == 1, "unsupported final Reshape interface")
        require(
            index.shape(final.input[1], source) in ([1, -1, output_width], [1, "T", output_width]),
            "unsupported final Reshape shape",
        )
        return source, final.output[0]
    except ValueError as error:
        raise ValueError(f"{stage}: {error}") from error


def resolve_taps(model: onnx.ModelProto, *, hidden_size: int = 768) -> list[dict]:
    """Resolve the fixed sixteen seams from unique operators and linked edges."""
    require(type(hidden_size) is int and hidden_size > 0, "positive hidden width required")
    require([output.name for output in model.graph.output] == ["logits"], "original outlets must be exactly logits")
    index = _Index(model)
    norms = {
        stage: unique_norm(model, f"{SCOPE}/{name}", stage)
        for stage, name in (("ln1", "ln_1"), ("cross_norm", "ln_cross_attn"), ("ln2", "ln_2"))
    }
    next_norm = unique_norm(model, "/decoder/transformer/h.1/ln_1", "block_output")
    tensors = {
        "block_input": norms["ln1"].input[0],
        "ln1": norms["ln1"].output[0],
        "self_residual": norms["cross_norm"].input[0],
        "cross_norm": norms["cross_norm"].output[0],
        "cross_residual": norms["ln2"].input[0],
        "ln2": norms["ln2"].output[0],
        "block_output": next_norm.input[0],
    }
    operations = [
        ("self_qkv", "attn/c_attn", 1, 3, None),
        ("self_projection", "attn/c_proj", 1, 1, "self_context"),
        ("cross_query", "crossattention/q_attn", 1, 1, None),
        ("cross_projection", "crossattention/c_proj", 1, 1, "cross_context"),
        ("mlp_fc", "mlp/c_fc", 1, 4, None),
        ("mlp_projection", "mlp/c_proj", 4, 1, "mlp_activation"),
    ]
    for stage, path, incoming, outgoing, pre in operations:
        source, tensors[stage] = _conv(index, stage, f"{SCOPE}/{path}", hidden_size * incoming, hidden_size * outgoing)
        if pre:
            tensors[pre] = source
    taps = [
        {"stage": stage, "tensor": tensors[stage], "width": hidden_size * width}
        for stage, width in zip(STAGES, MULTIPLIERS, strict=True)
    ]
    names = [tap["tensor"] for tap in taps]
    require(
        len(set(names)) == 16 and all(name in index.producers for name in names),
        "duplicate or missing produced subblock tensors",
    )
    return taps


def instrument_graph(model: onnx.ModelProto, taps: list[dict]) -> onnx.ModelProto:
    """Append only typed existing-value outlets, preserving the entire protobuf."""
    require([tap["stage"] for tap in taps] == list(STAGES), "subblock stage inventory changed")
    require([value.name for value in model.graph.output] == ["logits"], "original outlets must be exactly logits")
    names = [tap["tensor"] for tap in taps]
    produced = {name for node in model.graph.node for name in node.output}
    require(
        len(set(names)) == 16 and "logits" not in names and set(names).issubset(produced),
        "duplicate or missing subblock outlet",
    )
    result = copy.deepcopy(model)
    del result.graph.output[:]
    for tap in taps:
        require(type(tap["width"]) is int and tap["width"] > 0, "invalid subblock outlet width")
        result.graph.output.append(
            helper.make_tensor_value_info(tap["tensor"], TensorProto.FLOAT, [1, "tokens", tap["width"]])
        )
    result.graph.output.extend(model.graph.output)
    return result


def compare_taps(expected: dict, actual: dict, *, prefix_length: int) -> list[dict]:
    """Validate heterogeneous stage widths before reusing the original comparator."""
    require(type(prefix_length) is int and prefix_length in (1, 2), "unsupported prefix length")
    require(set(expected) == set(actual) == set(STAGES), "missing or additional subblock stage")
    for stage, width in zip(STAGES, MULTIPLIERS, strict=True):
        require(
            expected[stage].shape == actual[stage].shape == (1, prefix_length, width * 768),
            f"subblock capture shape mismatch: {stage}",
        )
    return compare_existing(expected, actual, order=STAGES)


def summarize(cases: list[dict], *, original_result: dict) -> dict:
    """Count only subblock pairs; the immutable original qualification stays failed."""
    require([case["case"] for case in cases] == ["zeros_t1", "zeros_t2"], "subblock case inventory changed")
    result = summarize_existing(cases, original_result=original_result, expected_cases=2, expected_stages=STAGES)
    require(result["actual_counts"] == COUNTS, "subblock element counts differ from frozen protocol")
    return result


def validate_controls(controls: dict) -> dict:
    """Require two literal bit-equal comparisons for every separate observer class."""
    names = ("mask", "hf_observer", "ort_observer")
    require(set(controls) == set(names), "observer control classes changed")
    for name in names:
        require(len(controls[name]) == 2, f"{name}: observer control count mismatch")
        for row in controls[name]:
            require(
                type(row.get("compared_elements")) is int
                and row["compared_elements"] > 0
                and row.get("failed_elements") == 0
                and row.get("passed") is True
                and row.get("bitwise_equal") is True,
                f"{name}: observer is not bit equal",
            )
    return dict.fromkeys(names, 2)
