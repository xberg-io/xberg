"""Prospective selected-block diagnostics; original block-zero contracts stay frozen."""

import copy
import importlib

import numpy as np
import onnx
import onnxruntime as ort
import pytest

from conftest import hashes, native_logits, write_json
from subblock_fixtures import SCOPE, STAGES, feature, stage_arrays, tiny_graph
from test_subblock_hooks import module_state
from test_subblock_plan import checksum, evidence  # noqa: F401
from test_subblock_receipts import ORIGINAL


def selected_graph(block, layers):
    """Rename one real computational block's scope without changing its computation."""
    model = tiny_graph()
    original = model.SerializeToString()
    for node in model.graph.node:
        if node.name.startswith(SCOPE + "/"):
            node.name = node.name.replace(SCOPE, f"/decoder/transformer/h.{block}", 1)
        elif node.name == "/decoder/transformer/h.1/ln_1/LayerNormalization":
            anchor = "ln_f" if block == layers - 1 else f"h.{block + 1}/ln_1"
            node.name = f"/decoder/transformer/{anchor}/LayerNormalization"
    onnx.checker.check_model(model)
    options = ort.SessionOptions()
    options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_DISABLE_ALL
    options.execution_mode = ort.ExecutionMode.ORT_SEQUENTIAL
    options.intra_op_num_threads = options.inter_op_num_threads = 1
    feed = {
        "input_ids": np.zeros((1, 2), dtype=np.int64),
        "encoder_hidden_states": np.full((1, 2, 4), 0.25, dtype=np.float32),
    }
    before = ort.InferenceSession(original, options, providers=["CPUExecutionProvider"]).run(["logits"], feed)[0]
    after = ort.InferenceSession(model.SerializeToString(), options, providers=["CPUExecutionProvider"]).run(
        ["logits"], feed
    )[0]
    assert before.shape == after.shape == (1, 2, 7) and before.size == 14
    assert np.isfinite(after).all() and before.tobytes() == after.tobytes()
    return model


@pytest.mark.parametrize("block", [1, 11])
def test_selected_block_maps_existing_roles_and_final_ln_f_input(monkeypatch, block):
    model = selected_graph(block, 12)
    api = feature(monkeypatch, required=("resolve_taps", "instrument_graph"))
    assert len(api.resolve_taps(tiny_graph(), hidden_size=4)) == 16
    before = model.SerializeToString()
    taps = api.resolve_taps(model, hidden_size=4, block_index=block, layer_count=12)
    assert [tap["stage"] for tap in taps] == list(STAGES)
    assert [tap["tensor"] for tap in taps] == list(STAGES)
    assert len({tap["tensor"] for tap in taps}) == 16
    anchor = next(node for node in model.graph.node if node.output[0] == "next_norm")
    assert taps[-1]["tensor"] == anchor.input[0] == "block_output"
    assert ("/ln_f/" in anchor.name) == (block == 11)
    tapped = api.instrument_graph(model, taps)
    assert len(tapped.graph.output) == 17
    assert model.SerializeToString() == before


@pytest.mark.parametrize("broken", ["missing_role", "foreign_shape"])
def test_selected_final_block_rejects_malformed_producer_at_named_stage(monkeypatch, broken):
    model = selected_graph(11, 12)
    api = feature(monkeypatch, required=("resolve_taps",))
    assert len(api.resolve_taps(model, hidden_size=4, block_index=11, layer_count=12)) == 16
    if broken == "missing_role":
        node = next(node for node in model.graph.node if node.name.endswith("h.11/attn/c_attn/Gemm"))
        node.name = "/foreign/attn/c_attn/Gemm"
    else:
        node = next(node for node in model.graph.node if node.name.endswith("h.11/attn/c_attn/Reshape"))
        node.input[1] = "self_qkv.weight"
    onnx.checker.check_model(model)
    with pytest.raises(ValueError, match=r"(?i)(missing|shape|reshape)") as error:
        api.resolve_taps(model, hidden_size=4, block_index=11, layer_count=12)
    assert "self_qkv" in str(error.value)


def selection_protocol(data, block=11):
    """Specify only the newly selected scope and work; preserve every numerical policy."""
    selected = copy.deepcopy(data)
    selected["protocol"].update(block_index=block, cases=["zeros_t2"], prefix_lengths=[2])
    selected["expected_counts"].update(
        cases=1, paired_stage_comparisons=16, compared_stage_elements=36864, stage_elements_T1=0
    )
    for name in selected["expected_controls"]:
        selected["expected_controls"][name] = 1
    return selected


@pytest.mark.parametrize("block", [1, 11])
def test_freezer_binds_selected_scope_case_and_metadata_derived_layer_bounds(monkeypatch, evidence, block):
    path, original, data = evidence
    before = hashes(original)
    api = feature(monkeypatch, "subblock_plan", ("freeze_plan",))
    default = api.freeze_plan(path, checksum(path), original)
    assert default["protocol"]["cases"] == ["zeros_t1", "zeros_t2"]
    assert default["expected_counts"]["paired_stage_comparisons"] == 32
    selected = selection_protocol(data, block)
    write_json(path, selected)
    plan = api.freeze_plan(path, checksum(path), original)
    assert plan["protocol"] == selected["protocol"]
    assert plan["expected_counts"] == selected["expected_counts"]
    assert plan["expected_controls"] == selected["expected_controls"]
    assert plan["expected_counts"]["ort_total_outlets_including_observer_logits"] == 17
    assert plan["original_evidence"] == default["original_evidence"]
    assert hashes(original) == before


@pytest.mark.parametrize("broken", ["negative_block", "past_last", "empty", "unknown", "duplicate", "metadata"])
def test_freezer_rejects_invalid_selection_against_original_metadata(monkeypatch, evidence, broken):
    path, original, data = evidence
    api = feature(monkeypatch, "subblock_plan", ("freeze_plan",))
    selected = selection_protocol(data)
    write_json(path, selected)
    assert api.freeze_plan(path, checksum(path), original)["protocol"]["block_index"] == 11
    if broken in {"negative_block", "past_last"}:
        selected["protocol"]["block_index"] = -1 if broken == "negative_block" else 12
    elif broken == "metadata":
        original_plan = original / "plan.json"
        import json

        metadata = json.loads(original_plan.read_text())
        metadata["geometry"]["layers"] = 2
        write_json(original_plan, metadata)
    else:
        selected["protocol"]["cases"] = {"empty": [], "unknown": ["uniform_t2"], "duplicate": ["zeros_t2", "zeros_t2"]}[
            broken
        ]
    write_json(path, selected)
    before = hashes(original)
    with pytest.raises(ValueError, match=r"(?i)(block|layer|case|selection)"):
        api.freeze_plan(path, checksum(path), original)
    assert hashes(original) == before


@pytest.mark.parametrize("block", [1, 11])
def test_hooks_observe_selected_real_gpt2_block_and_remove_all_handles(monkeypatch, tiny_model, block):
    import torch

    model, pixels, source, source_hashes = tiny_model
    prototype = model.decoder.transformer.h[0]
    model.decoder.transformer.h = torch.nn.ModuleList([prototype, *[copy.deepcopy(prototype) for _ in range(11)]])
    model.decoder.config.n_layer = model.config.decoder.n_layer = 12
    ids, context, native = native_logits(model, pixels, 2)
    assert len(model.decoder.transformer.h) == model.decoder.config.n_layer == 12
    state = module_state(model)
    api = feature(monkeypatch, "subblock_hooks", ("capture_native",))
    observed = []
    handle = model.decoder.transformer.h[block].register_forward_pre_hook(
        lambda _module, args: observed.append(args[0].detach().numpy().copy())
    )
    try:
        with torch.inference_mode():
            capture = api.capture_native(model, ids.numpy(), context.numpy(), block_index=block)
    finally:
        handle.remove()
    assert len(observed) == 1 and capture["calls"] == dict.fromkeys(STAGES, 1)
    assert capture["captures"]["block_input"].tobytes() == observed[0].tobytes()
    assert capture["logits"].tobytes() == native.numpy().tobytes()
    assert module_state(model) == state and hashes(source) == source_hashes


def test_one_case_receipt_counts_only_t2_and_retains_six_original_failures(monkeypatch):
    api = feature(monkeypatch, required=("compare_taps", "summarize"))
    arrays = stage_arrays(2)
    rows = api.compare_taps(arrays, arrays, prefix_length=2)
    assert len(rows) == 16 and sum(row["compared_elements"] for row in rows) == 36864
    original = copy.deepcopy(ORIGINAL)
    case = {"case": "zeros_t2", "comparisons": rows, "hook_calls": dict.fromkeys(STAGES, 1)}
    result = api.summarize([case], original_result=original, case_names=["zeros_t2"])
    assert result["actual_counts"] == {"cases": 1, "stages": 16, "compared_elements": 36864}
    assert result["qualified"] is False and result["diagnostic_complete"] is True
    assert result["original_result"] == original == ORIGINAL and len(original["failed_prefixes"]) == 6
    runner = feature(monkeypatch, "subblock_run", ("execute",))
    counts = runner._counts([case], [{"stage": name} for name in STAGES])
    assert counts == {
        "cases": 1,
        "paired_stage_comparisons": 16,
        "compared_stage_elements": 36864,
        "hooks_per_case": 16,
        "ort_stage_outlets": 16,
        "ort_total_outlets_including_observer_logits": 17,
        "stage_elements_T1": 0,
        "stage_elements_T2": 36864,
    }


@pytest.mark.parametrize("broken", [None, "mask", "hf_observer", "ort_observer"])
def test_one_case_observers_require_all_three_nonempty_bit_equal_comparisons(monkeypatch, broken):
    api = feature(monkeypatch, required=("validate_controls",))
    comparator = importlib.import_module("caption_export.first_divergence").compare_tensor
    values = np.full((1, 2, 7), 0.25, dtype=np.float32)
    controls = {
        name: [comparator(values, values, atol=1e-5, rtol=1e-4)] for name in ("mask", "hf_observer", "ort_observer")
    }
    assert sum(row["compared_elements"] for rows in controls.values() for row in rows) == 42
    if broken is not None:
        changed = values.copy()
        changed[0, 0, 0] = np.nextafter(changed[0, 0, 0], np.float32(1.0))
        controls[broken][0] = comparator(values, changed, atol=1e-5, rtol=1e-4)
        assert controls[broken][0]["passed"] is True and not controls[broken][0]["bitwise_equal"]
        with pytest.raises(ValueError, match=broken):
            api.validate_controls(controls, expected_cases=1)
    else:
        assert api.validate_controls(controls, expected_cases=1) == dict.fromkeys(controls, 1)
