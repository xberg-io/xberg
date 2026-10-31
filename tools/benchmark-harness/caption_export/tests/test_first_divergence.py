"""Prospective decoder tap contracts; no Torch, sessions, exports or real weights."""

import copy
import importlib
from pathlib import Path

import numpy as np
import onnx
import pytest
from onnx import TensorProto, helper

SCOPE = "/decoder/transformer"
STAGES = ["embedding", *(f"block.{index}" for index in range(12)), "final_norm", "logits"]
ATOL = 1e-5
RTOL = 1e-4
ORIGINAL = {
    "qualified": False,
    "status": "FAIL",
    "producer_head": "993bb0b12c21a57e115915416865eb3bc1a91161",
    "decoder_sha256": "f122e35c66c3000d719ab5273fcd86068bb7bf67e13c81d227a5612c1d62b39e",
    "failed_prefixes": [[image, length] for image in ("zeros", "uniform") for length in (2, 8, 16)],
}


def diagnostic(monkeypatch):
    root = Path(__file__).resolve().parents[1]
    monkeypatch.syspath_prepend(str(root / "src"))
    try:
        module = importlib.import_module("caption_export.first_divergence")
    except ModuleNotFoundError as error:
        if error.name in {"caption_export", "caption_export.first_divergence"}:
            pytest.fail("first-divergence diagnostic feature is not implemented", pytrace=False)
        raise
    assert Path(module.__file__).resolve().is_relative_to(root), "exercise this checkout's actual diagnostic"
    return module


def tiny_graph(*, missing=None, duplicate=None):
    nodes = []
    previous = "hidden"
    for index in range(12):
        scope = f"{SCOPE}/h.{index}/ln_1"
        name = f"{scope}/LayerNormalization"
        if missing == f"block.{index}":
            name = f"{SCOPE}/h.{index}/different_norm/LayerNormalization"
        nodes.append(helper.make_node("LayerNormalization", [previous, "scale", "bias"], [f"norm.{index}"], name=name))
        if duplicate == index:
            nodes.append(
                helper.make_node(
                    "LayerNormalization",
                    [previous, "scale", "bias"],
                    [f"duplicate.{index}"],
                    name=f"{scope}/LayerNormalization_1",
                )
            )
        nodes.append(
            helper.make_node("Add", [previous, f"norm.{index}"], [f"residual.{index}"], name=f"{SCOPE}/h.{index}/Add")
        )
        previous = f"residual.{index}"
    final_scope = "different_final_norm" if missing == "final_norm" else "ln_f"
    nodes.append(
        helper.make_node(
            "LayerNormalization",
            [previous, "scale", "bias"],
            ["normalized"],
            name=f"{SCOPE}/{final_scope}/LayerNormalization",
        )
    )
    nodes.append(helper.make_node("MatMul", ["normalized", "projection"], ["logits"], name="/decoder/lm_head/MatMul"))
    nodes.append(
        helper.make_node(
            "LayerNormalization",
            ["hidden", "scale", "bias"],
            ["unrelated"],
            name="/other_decoder/transformer/h.2/ln_1/LayerNormalization",
        )
    )
    graph = helper.make_graph(
        nodes,
        "tiny-real-operators",
        [helper.make_tensor_value_info("hidden", TensorProto.FLOAT, [1, "tokens", 2])],
        [helper.make_tensor_value_info("logits", TensorProto.FLOAT, [1, "tokens", 3])],
        initializer=[
            helper.make_tensor("scale", TensorProto.FLOAT, [2], [1.0, 1.0]),
            helper.make_tensor("bias", TensorProto.FLOAT, [2], [0.0, 0.0]),
            helper.make_tensor("projection", TensorProto.FLOAT, [2, 3], [-0.2, 0.1, 0.3, 0.4, -0.5, 0.6]),
        ],
    )
    model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", 17)])
    onnx.checker.check_model(model)
    assert len(model.graph.node) >= 26
    assert len(model.graph.initializer) == 3
    return model


def captures(length):
    result = {
        stage: np.full((1, length, 50257 if stage == "logits" else 768), 0.25, dtype=np.float32)
        for stage in reversed(STAGES)
    }
    assert len(result) == 15
    assert sum(value.size for value in result.values()) == length * 61009
    return result


def compare(api, expected, actual):
    return api.compare_taps(expected, actual, order=STAGES, atol=ATOL, rtol=RTOL)


def cases(api):
    return [
        {"case": f"zeros_t{length}", "comparisons": compare(api, captures(length), captures(length))}
        for length in (1, 2)
    ]


def summarize(api, rows, original=None):
    return api.summarize(
        rows,
        original_result=copy.deepcopy(ORIGINAL) if original is None else original,
        expected_cases=2,
        expected_stages=STAGES,
    )


def test_should_resolve_fifteen_unique_taps_in_natural_layer_order(monkeypatch):
    model = tiny_graph()
    taps = diagnostic(monkeypatch).resolve_taps(model, layers=12, scope=SCOPE)
    assert [tap["stage"] for tap in taps] == STAGES
    assert [tap["tensor"] for tap in taps] == [
        "hidden",
        *(f"residual.{index}" for index in range(12)),
        "normalized",
        "logits",
    ]
    assert len({tap["tensor"] for tap in taps}) == 15
    assert STAGES.index("block.2") < STAGES.index("block.10")


@pytest.mark.parametrize("missing", ["block.4", "final_norm"])
def test_should_reject_missing_scoped_norm_before_comparison(monkeypatch, missing):
    model = tiny_graph(missing=missing)
    with pytest.raises(ValueError, match=r"(?i)missing") as error:
        diagnostic(monkeypatch).resolve_taps(model, layers=12, scope=SCOPE)
    assert missing in str(error.value)


def test_should_reject_duplicate_scoped_norm_instead_of_picking_first(monkeypatch):
    model = tiny_graph(duplicate=4)
    with pytest.raises(ValueError, match=r"(?i)(duplicate|ambiguous)") as error:
        diagnostic(monkeypatch).resolve_taps(model, layers=12, scope=SCOPE)
    assert "block.4" in str(error.value)


def test_should_add_only_diagnostic_outlets_without_mutating_original_graph(monkeypatch, tmp_path):
    model = tiny_graph()
    source = tmp_path / "original.onnx"
    onnx.save_model(model, source)
    before = source.read_bytes()
    api = diagnostic(monkeypatch)
    taps = api.resolve_taps(model, layers=12, scope=SCOPE)
    tapped = api.instrument_graph(model, taps, hidden_size=2)
    onnx.checker.check_model(tapped)
    assert tapped is not model
    assert model.SerializeToString() == before
    assert source.read_bytes() == before
    assert [node.SerializeToString() for node in tapped.graph.node] == [
        node.SerializeToString() for node in model.graph.node
    ]
    assert [tensor.SerializeToString() for tensor in tapped.graph.initializer] == [
        tensor.SerializeToString() for tensor in model.graph.initializer
    ]
    assert [output.name for output in model.graph.output] == ["logits"]
    assert len(tapped.graph.output) == 15
    assert {output.name for output in tapped.graph.output} == {tap["tensor"] for tap in taps}


def test_should_count_thirty_pairs_and_preserve_original_failed_qualification(monkeypatch):
    expected = captures(1)
    actual = captures(2)
    assert sum(tensor.size for tensor in (*expected.values(), *actual.values())) == 183027
    api = diagnostic(monkeypatch)
    original = copy.deepcopy(ORIGINAL)
    before = copy.deepcopy(original)
    report = summarize(api, cases(api), original)
    assert report["actual_counts"] == {"cases": 2, "stages": 30, "compared_elements": 183027}
    assert report["diagnostic_complete"] is True
    assert report["qualified"] is False
    assert report["original_result"] == before
    assert original == before
    assert report["first_bitwise_divergence"] is None
    assert report["first_tolerance_failure"] is None


def test_should_distinguish_first_bitwise_drift_from_first_tolerance_failure(monkeypatch):
    expected, actual = captures(1), captures(1)
    actual["block.2"][0, 0, 0] = np.nextafter(np.float32(0.25), np.float32(1.0))
    actual["block.10"][0, 0, 0] = 0.5
    api = diagnostic(monkeypatch)
    rows = compare(api, expected, actual)
    assert [row["stage"] for row in rows] == STAGES
    early = next(row for row in rows if row["stage"] == "block.2")
    assert early["passed"] is True and early["bitwise_equal"] is False
    records = cases(api)
    records[0]["comparisons"] = sorted(rows, key=lambda row: row["stage"])
    report = summarize(api, records)
    assert report["first_bitwise_divergence"] == {"case": "zeros_t1", "stage": "block.2"}
    assert report["first_tolerance_failure"] == {"case": "zeros_t1", "stage": "block.10"}
    assert report["qualified"] is False


def test_should_detect_one_poisoned_element_at_its_actual_stage(monkeypatch):
    expected, actual = captures(2), captures(2)
    actual["block.4"][0, 1, 3] += 1.0
    api = diagnostic(monkeypatch)
    rows = compare(api, expected, actual)
    failures = [row for row in rows if not row["passed"]]
    assert len(failures) == 1
    assert failures[0]["stage"] == "block.4"
    assert failures[0]["compared_elements"] == 1536
    assert failures[0]["failed_elements"] == 1
    assert failures[0]["first_failed_index"] == [0, 1, 3]


def test_should_treat_signed_zero_as_bitwise_drift_without_numeric_failure(monkeypatch):
    expected, actual = captures(1), captures(1)
    expected["embedding"][0, 0, 0] = 0.0
    actual["embedding"][0, 0, 0] = -0.0
    row = compare(diagnostic(monkeypatch), expected, actual)[0]
    assert row["stage"] == "embedding"
    assert row["passed"] is True
    assert row["failed_elements"] == 0
    assert row["bitwise_equal"] is False


@pytest.mark.parametrize("broken", ["zero_cases", "missing_stage", "duplicate_stage"])
def test_should_reject_empty_or_incomplete_job_counts(monkeypatch, broken):
    captures(1)
    api = diagnostic(monkeypatch)
    records = cases(api)
    if broken == "zero_cases":
        records = []
    elif broken == "missing_stage":
        records[0]["comparisons"].pop()
    else:
        records[0]["comparisons"][-1] = copy.deepcopy(records[0]["comparisons"][-2])
    with pytest.raises(ValueError, match=r"(?i)(count|missing|duplicate|stage|cases)"):
        summarize(api, records)


@pytest.mark.parametrize("broken", ["missing_capture", "float64", "wrong_shape", "nonfinite"])
def test_should_fail_closed_for_invalid_actual_capture(monkeypatch, broken):
    expected, actual = captures(1), captures(1)
    if broken == "missing_capture":
        del actual["block.4"]
    elif broken == "float64":
        actual["block.4"] = actual["block.4"].astype(np.float64)
    elif broken == "wrong_shape":
        actual["block.4"] = actual["block.4"][:, :, :-1]
    else:
        actual["block.4"][0, 0, 0] = np.nan
    with pytest.raises(ValueError, match=r"(?i)(missing|float32|dtype|shape|finite)"):
        compare(diagnostic(monkeypatch), expected, actual)
