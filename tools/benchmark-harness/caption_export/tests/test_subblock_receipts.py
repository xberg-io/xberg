"""Count only declared subblock work and preserve original failed qualification."""

import copy
import importlib

import numpy as np
import pytest

from subblock_fixtures import STAGES, feature, stage_arrays

ORIGINAL = {
    "qualified": False,
    "status": "FAIL",
    "failed_prefixes": [[image, length] for image in ("zeros", "uniform") for length in (2, 8, 16)],
}


def comparisons(api, length):
    expected, actual = stage_arrays(length), stage_arrays(length)
    return api.compare_taps(expected, actual, prefix_length=length)


def test_counts_thirty_two_pairs_not_observer_logits_and_preserves_original_failure(monkeypatch):
    first, second = stage_arrays(1), stage_arrays(2)
    assert sum(value.size for value in (*first.values(), *second.values())) == 55296
    api = feature(monkeypatch, required=("compare_taps", "summarize"))
    original = copy.deepcopy(ORIGINAL)
    before = copy.deepcopy(original)
    records = [{"case": f"zeros_t{length}", "comparisons": comparisons(api, length)} for length in (1, 2)]
    result = api.summarize(records, original_result=original)
    assert result["actual_counts"] == {"cases": 2, "stages": 32, "compared_elements": 55296}
    assert result["diagnostic_complete"] is True and result["qualified"] is False
    assert result["original_result"] == original == before
    assert len(original["failed_prefixes"]) == 6
    result["original_result"]["failed_prefixes"].pop()
    assert original == before, "the report must not alias or mutate the original receipt"


@pytest.mark.parametrize("broken", ["poisoned_element", "missing_stage"])
def test_wrong_values_and_missing_work_fail_at_the_declared_stage(monkeypatch, broken):
    expected, actual = stage_arrays(2), stage_arrays(2)
    assert len(expected) == len(actual) == 16
    if broken == "poisoned_element":
        actual["mlp_fc"][0, 1, 3] += np.float32(1.0)
    else:
        del actual["cross_query"]
    api = feature(monkeypatch, required=("compare_taps",))
    if broken == "missing_stage":
        with pytest.raises(ValueError, match=r"(?i)missing"):
            api.compare_taps(expected, actual, prefix_length=2)
    else:
        rows = api.compare_taps(expected, actual, prefix_length=2)
        assert [row["stage"] for row in rows] == list(STAGES)
        failures = [row for row in rows if not row["passed"]]
        assert len(failures) == 1
        assert failures[0]["stage"] == "mlp_fc"
        assert failures[0]["compared_elements"] == 6144
        assert failures[0]["failed_elements"] == 1
        assert failures[0]["first_failed_index"] == [0, 1, 3]


@pytest.mark.parametrize("broken", [None, "mask", "hf_observer", "ort_observer"])
def test_each_paired_observer_class_requires_two_bitwise_equal_controls(monkeypatch, broken):
    baseline = np.full((1, 2, 7), 0.25, dtype=np.float32)
    assert baseline.size == 14
    api = feature(monkeypatch, required=("validate_controls",))
    comparator = importlib.import_module("caption_export.first_divergence").compare_tensor
    controls = {
        name: [comparator(baseline, baseline, atol=1e-5, rtol=1e-4) for _ in range(2)]
        for name in ("mask", "hf_observer", "ort_observer")
    }
    assert len(controls) == 3 and sum(len(rows) for rows in controls.values()) == 6
    if broken is not None:
        changed = baseline.copy()
        changed[0, 0, 0] = np.nextafter(changed[0, 0, 0], np.float32(1.0))
        controls[broken][0] = comparator(baseline, changed, atol=1e-5, rtol=1e-4)
        assert controls[broken][0]["passed"] is True and controls[broken][0]["bitwise_equal"] is False
        with pytest.raises(ValueError, match=broken):
            api.validate_controls(controls)
    else:
        assert api.validate_controls(controls) == {"mask": 2, "hf_observer": 2, "ort_observer": 2}
