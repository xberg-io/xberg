"""Inspect inline ONNX graphs against the exact native tensor interfaces."""

from collections.abc import Iterator, Sequence
from pathlib import Path

import onnx

from .metadata import Geometry

TensorInterface = tuple[str, int, list[int | str]]
GraphInterface = tuple[list[TensorInterface], list[TensorInterface]]


def interfaces(geometry: Geometry) -> dict[str, GraphInterface]:
    """Require fixed image/context axes and one shared dynamic token dimension."""
    context = [1, geometry.context, geometry.hidden]
    return {
        "encoder.onnx": (
            [("pixel_values", onnx.TensorProto.FLOAT, [1, 3, geometry.height, geometry.width])],
            [("last_hidden_state", onnx.TensorProto.FLOAT, context)],
        ),
        "decoder.onnx": (
            [
                ("input_ids", onnx.TensorProto.INT64, [1, "tokens"]),
                ("encoder_hidden_states", onnx.TensorProto.FLOAT, context),
            ],
            [("logits", onnx.TensorProto.FLOAT, [1, "tokens", geometry.vocabulary])],
        ),
    }


def validate_outlets(actual: Sequence[onnx.ValueInfoProto], expected: list[TensorInterface]) -> None:
    """Reject additional cache ports, wrong dtypes and any unplanned dynamic axis."""
    if [item.name for item in actual] != [name for name, _, _ in expected]:
        raise ValueError("ONNX interface contract has unexpected input/output names")
    for item, (_, dtype, shape) in zip(actual, expected, strict=True):
        if not item.type.HasField("tensor_type") or item.type.tensor_type.elem_type != dtype:
            raise ValueError(f"ONNX dtype contract mismatch: {item.name}")
        dimensions = item.type.tensor_type.shape.dim
        if len(dimensions) != len(shape):
            raise ValueError(f"ONNX shape contract rank mismatch: {item.name}")
        for dimension, required in zip(dimensions, shape, strict=True):
            if isinstance(required, str):
                valid = dimension.HasField("dim_param") and dimension.dim_param == required
            else:
                valid = dimension.HasField("dim_value") and dimension.dim_value == required
            if not valid:
                raise ValueError(f"ONNX shape contract mismatch: {item.name}")


def graph_tensors(graph: onnx.GraphProto) -> Iterator[onnx.TensorProto]:
    """Visit nested attribute tensors as well as ordinary learned initializers."""
    if graph.sparse_initializer:
        raise ValueError("sparse ONNX initializers are unsupported")
    yield from graph.initializer
    for node in graph.node:
        for attribute in node.attribute:
            if attribute.HasField("sparse_tensor") or attribute.sparse_tensors:
                raise ValueError("sparse ONNX attribute tensors are unsupported")
            if attribute.HasField("t"):
                yield attribute.t
            yield from attribute.tensors
            if attribute.HasField("g"):
                yield from graph_tensors(attribute.g)
            for nested in attribute.graphs:
                yield from graph_tensors(nested)


def validate_graph(path: Path, expected: GraphInterface) -> None:
    """Never resolve external data, even if an external file is present and pinned."""
    try:
        model = onnx.load(path, load_external_data=False)
        if model.functions or model.training_info:
            raise ValueError("ONNX local functions and training graphs are unsupported")
        if not model.graph.initializer:
            raise ValueError("ONNX learned-weight graph contract requires initializers")
        for tensor in graph_tensors(model.graph):
            if tensor.data_location == onnx.TensorProto.EXTERNAL or tensor.external_data:
                raise ValueError("external ONNX weights are unsupported")
        if [(item.domain, item.version) for item in model.opset_import] != [("", 17)]:
            raise ValueError("ONNX contract requires standard opset 17")
        validate_outlets(model.graph.input, expected[0])
        validate_outlets(model.graph.output, expected[1])
        onnx.checker.check_model(model)
    except (OSError, onnx.checker.ValidationError) as error:
        raise ValueError(f"invalid ONNX graph contract in {path.name}: {error}") from error


def fix_output_dimensions(path: Path, expected: list[TensorInterface]) -> None:
    """Resolve exporter-only symbolic static axes using checked eager output shapes."""
    model = onnx.load(path, load_external_data=False)
    for outlet, (_, _, shape) in zip(model.graph.output, expected, strict=True):
        dimensions = outlet.type.tensor_type.shape.dim
        if len(dimensions) != len(shape):
            raise ValueError(f"exported output shape rank mismatch: {outlet.name}")
        for dimension, required in zip(dimensions, shape, strict=True):
            if isinstance(required, int):
                if dimension.HasField("dim_value") and dimension.dim_value != required:
                    raise ValueError(f"exported output shape mismatch: {outlet.name}")
                dimension.dim_value = required
    onnx.save_model(model, path, save_as_external_data=False)
