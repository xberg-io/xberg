"""Observe existing decoder tensors without changing export qualification."""

import copy
import re
from collections.abc import Sequence

import numpy as np
import onnx
from onnx import TensorProto, helper

ATOL = 1e-5
RTOL = 1e-4
LAYERS = 12
STAGES = ("embedding", *(f"block.{index}" for index in range(LAYERS)), "final_norm", "logits")
SCOPE = "/decoder/transformer"


def unique_norm(model: onnx.ModelProto, scope: str, stage: str) -> onnx.NodeProto:
    """Require one exact scoped norm, including numbered exporter duplicates."""
    expression = re.compile(re.escape(scope) + r"/LayerNormalization(?:_\d+)?$")
    nodes = [
        node for node in model.graph.node if node.op_type == "LayerNormalization" and expression.fullmatch(node.name)
    ]
    if not nodes:
        raise ValueError(f"missing scoped normalization for {stage}")
    if len(nodes) != 1:
        raise ValueError(f"duplicate scoped normalization for {stage}")
    node = nodes[0]
    if len(node.input) != 3 or len(node.output) != 1:
        raise ValueError(f"unsupported normalization interface for {stage}")
    return node


def resolve_taps(model: onnx.ModelProto, *, layers: int = LAYERS, scope: str = SCOPE) -> list[dict]:
    """Resolve block outputs from the next norm's input, never a guessed Add."""
    if type(layers) is not int or layers <= 0 or not scope:
        raise ValueError("positive layer count and scoped graph names are required")
    norms = [unique_norm(model, f"{scope}/h.{index}/ln_1", f"block.{index}") for index in range(layers)]
    final = unique_norm(model, f"{scope}/ln_f", "final_norm")
    if [output.name for output in model.graph.output] != ["logits"]:
        raise ValueError("original decoder must expose exactly logits")
    block_outputs = [node.input[0] for node in norms[1:]] + [final.input[0]]
    taps = [{"stage": "embedding", "tensor": norms[0].input[0]}]
    taps.extend({"stage": f"block.{index}", "tensor": name} for index, name in enumerate(block_outputs))
    taps.extend([{"stage": "final_norm", "tensor": final.output[0]}, {"stage": "logits", "tensor": "logits"}])
    names = [tap["tensor"] for tap in taps]
    if any(not name for name in names) or len(set(names)) != layers + 3:
        raise ValueError("duplicate or empty diagnostic tap tensors")
    return taps


def instrument_graph(model: onnx.ModelProto, taps: list[dict], *, hidden_size: int) -> onnx.ModelProto:
    """Copy the protobuf and expose existing values; preserve nodes and weights."""
    if type(hidden_size) is not int or hidden_size <= 0 or not taps:
        raise ValueError("positive hidden size and nonempty taps are required")
    names = [tap["tensor"] for tap in taps]
    existing = {value.name: value for value in model.graph.output}
    produced = {name for node in model.graph.node for name in node.output} | {value.name for value in model.graph.input}
    if len(names) != len(set(names)) or not set(names).issubset(produced) or not set(existing).issubset(names):
        raise ValueError("duplicate, missing or incomplete diagnostic outlets")
    result = copy.deepcopy(model)
    del result.graph.output[:]
    for name in names:
        value = existing.get(name)
        if value is None:
            value = helper.make_tensor_value_info(name, TensorProto.FLOAT, [1, "tokens", hidden_size])
        result.graph.output.append(value)
    return result


def tensor_pair(expected: np.ndarray, actual: np.ndarray) -> None:
    """Reject invalid captures before counting any numerical comparison."""
    if not isinstance(expected, np.ndarray) or not isinstance(actual, np.ndarray):
        raise ValueError("captures must be float32 arrays")
    if expected.dtype != np.float32 or actual.dtype != np.float32:
        raise ValueError("captures must have float32 dtype")
    if expected.shape != actual.shape or expected.ndim != 3 or expected.shape[0] != 1 or not expected.size:
        raise ValueError("capture shape must be equal, nonempty and batch-one rank three")
    if not np.isfinite(expected).all() or not np.isfinite(actual).all():
        raise ValueError("captures must be finite")


def compare_tensor(expected: np.ndarray, actual: np.ndarray, *, atol: float, rtol: float) -> dict:
    """Report bounded numerical examples separately from literal FP32 bytes."""
    tensor_pair(expected, actual)
    close = np.isclose(actual, expected, atol=atol, rtol=rtol, equal_nan=False)
    failed = np.argwhere(~close)
    difference = np.abs(actual.astype(np.float64) - expected.astype(np.float64))
    examples = []
    for index in failed[:5]:
        coordinate = tuple(int(value) for value in index)
        reference, observed = float(expected[coordinate]), float(actual[coordinate])
        examples.append(
            {
                "index": list(coordinate),
                "expected": reference,
                "actual": observed,
                "absolute_error": abs(observed - reference),
                "bound": atol + rtol * abs(reference),
            }
        )
    return {
        "passed": len(failed) == 0,
        "bitwise_equal": expected.tobytes() == actual.tobytes(),
        "compared_elements": int(expected.size),
        "failed_elements": len(failed),
        "first_failed_index": failed[0].tolist() if len(failed) else None,
        "max_absolute_error": float(difference.max()),
        "mean_absolute_error": float(difference.mean()),
        "first_failed_elements": examples,
    }


def compare_taps(
    expected: dict, actual: dict, *, order: Sequence[str], atol: float = ATOL, rtol: float = RTOL
) -> list[dict]:
    """Compare each declared stage once in architecture order."""
    if not order or len(set(order)) != len(order):
        raise ValueError("nonempty unique stage order is required")
    if set(expected) != set(order) or set(actual) != set(order):
        raise ValueError("missing or additional capture stage")
    if not np.isfinite([atol, rtol]).all() or min(atol, rtol) < 0:
        raise ValueError("finite nonnegative tolerances are required")
    return [{"stage": stage, **compare_tensor(expected[stage], actual[stage], atol=atol, rtol=rtol)} for stage in order]


def comparison_index(rows: list[dict], stages: Sequence[str]) -> dict:
    """Validate receipt counts before deriving a first divergent stage."""
    if len(rows) != len(stages):
        raise ValueError("comparison stage count differs from expected")
    indexed = {row["stage"]: row for row in rows}
    if len(indexed) != len(rows) or set(indexed) != set(stages):
        raise ValueError("duplicate or missing comparison stage")
    for row in rows:
        count, failed = row["compared_elements"], row["failed_elements"]
        if type(count) is not int or count <= 0 or type(failed) is not int or not 0 <= failed <= count:
            raise ValueError("invalid compared element counts")
        if type(row["passed"]) is not bool or type(row["bitwise_equal"]) is not bool:
            raise ValueError("comparison result must contain boolean outcomes")
        if row["passed"] != (failed == 0) or (row["bitwise_equal"] and failed):
            raise ValueError("inconsistent comparison failure counts")
    return indexed


def summarize(cases: list[dict], *, original_result: dict, expected_cases: int, expected_stages: Sequence[str]) -> dict:
    """Keep the original qualification immutable even when every tap agrees."""
    if expected_cases <= 0 or len(cases) != expected_cases:
        raise ValueError("diagnostic cases count differs from expected")
    if not expected_stages or len(set(expected_stages)) != len(expected_stages):
        raise ValueError("expected stage order must be nonempty and unique")
    if len({case["case"] for case in cases}) != expected_cases:
        raise ValueError("duplicate diagnostic cases")
    first_bitwise, first_failure, elements = None, None, 0
    for case in cases:
        indexed = comparison_index(case["comparisons"], expected_stages)
        for stage in expected_stages:
            row = indexed[stage]
            elements += row["compared_elements"]
            location = {"case": case["case"], "stage": stage}
            if not row["bitwise_equal"] and first_bitwise is None:
                first_bitwise = location
            if not row["passed"] and first_failure is None:
                first_failure = location
    return {
        "actual_counts": {
            "cases": len(cases),
            "stages": len(cases) * len(expected_stages),
            "compared_elements": elements,
        },
        "diagnostic_complete": True,
        "qualified": False,
        "original_result": copy.deepcopy(original_result),
        "first_bitwise_divergence": first_bitwise,
        "first_tolerance_failure": first_failure,
    }
