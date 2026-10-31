"""Frozen five-interface export contract matching the experimental native SDK adapter."""

import hashlib
import json
import re

import pytest
from conftest import (
    ATOL,
    COMPANIONS,
    CONTEXT_LENGTH,
    HIDDEN_SIZE,
    IMAGE_SIZE,
    PREFIX_LENGTHS,
    RTOL,
    VOCABULARY,
    native_logits,
    refresh_manifest,
)


def sessions(directory):
    import onnxruntime as ort

    options = ort.SessionOptions()
    options.intra_op_num_threads = 1
    options.inter_op_num_threads = 1
    return tuple(
        ort.InferenceSession(str(directory / name), sess_options=options, providers=["CPUExecutionProvider"])
        for name in ("encoder.onnx", "decoder.onnx")
    )


def spec(outlets):
    return [(item.name, item.type, item.shape) for item in outlets]


def test_seeded_native_fixture_has_a_real_checkpoint_before_any_export(tiny_model):
    model, _, source, before = tiny_model
    assert model.config.model_type == "vision-encoder-decoder"
    assert model.config.encoder.hidden_size == model.config.decoder.n_embd == HIDDEN_SIZE
    assert model.config.decoder.vocab_size == VOCABULARY
    assert "model.safetensors" in before
    assert (source / "model.safetensors").stat().st_size > 0


def test_exports_exact_five_interfaces_with_fixed_image_and_context(export_case):
    api, directory, _, _ = export_case()
    api.validate_export(directory)
    encoder, decoder = sessions(directory)
    assert spec(encoder.get_inputs()) == [("pixel_values", "tensor(float)", [1, 3, IMAGE_SIZE, IMAGE_SIZE])]
    assert spec(encoder.get_outputs()) == [("last_hidden_state", "tensor(float)", [1, CONTEXT_LENGTH, HIDDEN_SIZE])]
    inputs = {item.name: item for item in decoder.get_inputs()}
    assert set(inputs) == {"input_ids", "encoder_hidden_states"}
    assert (inputs["input_ids"].type, inputs["input_ids"].shape[0]) == ("tensor(int64)", 1)
    assert isinstance(inputs["input_ids"].shape[1], str) and inputs["input_ids"].shape[1]
    assert (inputs["encoder_hidden_states"].type, inputs["encoder_hidden_states"].shape) == (
        "tensor(float)",
        [1, CONTEXT_LENGTH, HIDDEN_SIZE],
    )
    (logits,) = decoder.get_outputs()
    assert (logits.name, logits.type, logits.shape[0], logits.shape[2]) == ("logits", "tensor(float)", 1, VOCABULARY)
    assert isinstance(logits.shape[1], str) and logits.shape[1]


def test_full_prefix_native_export_parity_at_every_frozen_length(export_case):
    import numpy as np

    _, directory, model, pixels = export_case()
    encoder, decoder = sessions(directory)
    (context,) = encoder.run(None, {"pixel_values": pixels.numpy()})
    checked = []
    for length in PREFIX_LENGTHS:
        ids, native_context, expected = native_logits(model, pixels, length)
        (actual,) = decoder.run(None, {"input_ids": ids.numpy(), "encoder_hidden_states": context})
        assert actual.shape == (1, length, VOCABULARY)
        assert np.isfinite(actual).all()
        np.testing.assert_allclose(context, native_context.numpy(), atol=ATOL, rtol=RTOL)
        np.testing.assert_allclose(actual, expected.numpy(), atol=ATOL, rtol=RTOL)
        checked.append(length)
    assert checked == [1, 2, 8, 16], "a static trace or missing first-token case cannot pass"


def test_graphs_have_inline_weights_without_cache_or_local_functions(export_case):
    import onnx
    from onnx.external_data_helper import _get_all_tensors

    _, directory, _, _ = export_case()
    for name in ("encoder.onnx", "decoder.onnx"):
        graph = onnx.load(directory / name, load_external_data=False)
        onnx.checker.check_model(graph)
        assert not graph.functions and not graph.training_info and not graph.graph.sparse_initializer
        tensors = list(_get_all_tensors(graph))
        assert tensors and graph.graph.initializer, "the learned-weight fixture must not export as a no-op graph"
        assert all(tensor.data_location != onnx.TensorProto.EXTERNAL and not tensor.external_data for tensor in tensors)
    assert {p.name for p in directory.glob("*.onnx")} == {"encoder.onnx", "decoder.onnx"}


def test_complete_manifest_binds_both_graphs_and_all_six_companions(export_case):
    api, directory, _, _ = export_case()
    pins = json.loads((directory / "sha256.json").read_text())
    assert set(pins) == {*COMPANIONS, "encoder.onnx", "decoder.onnx"}
    for name, digest in pins.items():
        assert digest == hashlib.sha256((directory / name).read_bytes()).hexdigest()
    api.validate_export(directory)


def test_corrupt_graph_fails_for_checksum_before_graph_loading(export_case):
    api, directory, _, _ = export_case()
    path = directory / "encoder.onnx"
    path.write_bytes(path.read_bytes() + b"corruption")
    with pytest.raises(ValueError, match=r"(?i)(checksum|sha-?256|hash)"):
        api.validate_export(directory)


@pytest.mark.parametrize("missing", COMPANIONS)
def test_missing_companion_is_rejected_even_with_refreshed_manifest(export_case, missing):
    api, directory, _, _ = export_case()
    (directory / missing).unlink()
    refresh_manifest(directory)
    with pytest.raises(ValueError, match=re.escape(missing)):
        api.validate_export(directory)


@pytest.mark.parametrize("mutation", ["dtype", "shape"])
def test_rehashed_invalid_tensor_contract_is_rejected_for_its_actual_reason(export_case, mutation):
    import onnx

    api, directory, _, _ = export_case()
    path = directory / "encoder.onnx"
    graph = onnx.load(path)
    pixels = graph.graph.input[0].type.tensor_type
    if mutation == "dtype":
        pixels.elem_type = onnx.TensorProto.INT64
    else:
        pixels.shape.dim[0].dim_value = 2
    onnx.save(graph, path)
    refresh_manifest(directory)
    with pytest.raises(ValueError, match=r"(?i)(dtype|type|shape|contract)"):
        api.validate_export(directory)


def test_rehashed_external_weights_are_rejected_even_when_present_and_pinned(export_case):
    import onnx
    from onnx.external_data_helper import _get_all_tensors

    api, directory, _, _ = export_case()
    path = directory / "encoder.onnx"
    graph = onnx.load(path)
    onnx.save_model(
        graph,
        path,
        save_as_external_data=True,
        all_tensors_to_one_file=True,
        location="weights.bin",
        size_threshold=0,
        convert_attribute=True,
    )
    external = onnx.load(path, load_external_data=False)
    assert any(t.data_location == onnx.TensorProto.EXTERNAL for t in _get_all_tensors(external))
    assert (directory / "weights.bin").stat().st_size > 0
    refresh_manifest(directory)
    with pytest.raises(ValueError, match=r"(?i)external"):
        api.validate_export(directory)


@pytest.mark.parametrize("filename", ["config.json", "generation_config.json"])
def test_rehashed_cache_enabled_metadata_is_rejected(export_case, filename):
    api, directory, _, _ = export_case()
    path = directory / filename
    value = json.loads(path.read_text())
    target = value["decoder"] if filename == "config.json" else value
    target["use_cache"] = True
    path.write_text(json.dumps(value), encoding="utf-8")
    refresh_manifest(directory)
    with pytest.raises(ValueError, match=r"(?i)(cache|greedy)"):
        api.validate_export(directory)
