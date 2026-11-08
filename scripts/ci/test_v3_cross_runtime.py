import json
from pathlib import Path

import pytest
import v3_cross_runtime as contract

FROZEN = json.loads((Path(__file__).parent / "fixtures/v3_cross_runtime_frozen.json").read_text())
PAGE_ONE, PAGE_TWO = FROZEN


def logits(page: dict) -> dict[tuple[int, int, int], tuple[float, float]]:
    return {(m["row"], m["y"], m["x"]): (m["reference_logit"], m["actual_logit"]) for m in page["mask_mismatches"]}


def mismatches(page: dict) -> list[tuple[int, int, int, int, int]]:
    return [(m["row"], m["y"], m["x"], m["reference_bit"], m["actual_bit"]) for m in page["mask_mismatches"]]


def rank_args(page: dict) -> tuple[list[int], list[int], list[float], list[float]]:
    return page["reference_ranks"], page["actual_ranks"], page["reference_keys"], page["actual_keys"]


def test_frozen_page_one_rank_differences_are_all_exact_ties() -> None:
    reference, actual, _, _ = rank_args(PAGE_ONE)
    assert sum(a != b for a, b in zip(reference, actual, strict=True)) == 24
    assert contract.compare_ranks(*rank_args(PAGE_ONE)) == 24


def test_frozen_page_two_ranks_are_identical() -> None:
    assert PAGE_TWO["reference_ranks"] == PAGE_TWO["actual_ranks"]
    assert contract.compare_ranks(*rank_args(PAGE_TWO)) == 0


def test_frozen_page_two_flips_are_boundary_logits() -> None:
    assert len(PAGE_TWO["mask_mismatches"]) == 5
    assert len({logits(PAGE_TWO)[key] for key in logits(PAGE_TWO)}) == 3
    assert contract.compare_masks(mismatches(PAGE_TWO), logits(PAGE_TWO)) == 5


def test_a_real_reorder_of_non_tied_rows_fails() -> None:
    reference, actual, ref_keys, act_keys = rank_args(PAGE_ONE)
    first, second = 0, 1
    assert ref_keys[first] != ref_keys[second]
    swapped = [second if r == first else first if r == second else r for r in actual]
    with pytest.raises(ValueError, match="rank_mismatch"):
        contract.compare_ranks(reference, swapped, ref_keys, act_keys)


def test_a_swap_between_near_equal_but_unequal_keys_fails() -> None:
    reference, actual, ref_keys, act_keys = rank_args(PAGE_ONE)
    near = next(r for r in range(len(ref_keys) - 1) if 0 < ref_keys[r + 1] - ref_keys[r] < 0.05)
    swapped = [near + 1 if r == near else near if r == near + 1 else r for r in actual]
    with pytest.raises(ValueError, match="rank_mismatch"):
        contract.compare_ranks(reference, swapped, ref_keys, act_keys)


def test_ties_are_compared_exactly_not_by_epsilon() -> None:
    reference, actual, ref_keys, act_keys = rank_args(PAGE_ONE)
    moved = next(r for r, a in zip(reference, actual, strict=True) if r != a)
    nudged = list(ref_keys)
    nudged[moved] = nudged[moved] + 1e-9
    with pytest.raises(ValueError, match="rank_mismatch"):
        contract.compare_ranks(reference, actual, nudged, act_keys)


def test_a_tie_the_other_runtime_does_not_share_fails() -> None:
    reference, actual, ref_keys, act_keys = rank_args(PAGE_ONE)
    moved = next(a for r, a in zip(reference, actual, strict=True) if r != a)
    split = list(act_keys)
    split[moved] = split[moved] + 1.0
    with pytest.raises(ValueError, match="rank_mismatch"):
        contract.compare_ranks(reference, actual, ref_keys, split)


def test_two_rows_cannot_collapse_onto_one_rank() -> None:
    with pytest.raises(ValueError, match="rank_mismatch"):
        contract.compare_ranks([0, 1], [0, 0], [5.0, 5.0], [5.0, 5.0])


def test_a_tie_inside_one_run_may_order_either_way() -> None:
    assert contract.compare_ranks([0, 1, 2], [1, 0, 2], [5.0, 5.0, 7.0], [5.0, 5.0, 7.0]) == 2


def test_a_mask_flip_far_from_the_threshold_fails() -> None:
    far = [(0, 1, 1, 1, 0)]
    with pytest.raises(ValueError, match="mask_mismatch"):
        contract.compare_masks(far, {(0, 1, 1): (0.01, -0.01)})


def test_a_flip_near_zero_on_one_runtime_only_fails() -> None:
    with pytest.raises(ValueError, match="mask_mismatch"):
        contract.compare_masks([(0, 1, 1, 1, 0)], {(0, 1, 1): (1e-6, -0.5)})


def test_the_epsilon_bound_is_inclusive_and_tight() -> None:
    edge = contract.MASK_LOGIT_EPSILON
    assert contract.compare_masks([(0, 0, 0, 1, 0)], {(0, 0, 0): (edge, -edge)}) == 1
    with pytest.raises(ValueError, match="mask_mismatch"):
        contract.compare_masks([(0, 0, 0, 1, 0)], {(0, 0, 0): (edge * 1.01, -edge)})


def test_a_flip_without_logit_evidence_fails() -> None:
    with pytest.raises(ValueError, match="mask_mismatch"):
        contract.compare_masks([(0, 0, 0, 1, 0)], {})


def test_evidence_that_disagrees_with_the_bits_fails() -> None:
    with pytest.raises(ValueError, match="mask_mismatch"):
        contract.compare_masks([(0, 0, 0, 1, 0)], {(0, 0, 0): (-1e-6, -2e-6)})


def test_a_perturbed_frozen_flip_fails() -> None:
    page = json.loads(json.dumps(PAGE_TWO))
    page["mask_mismatches"][0]["reference_logit"] = 0.001
    with pytest.raises(ValueError, match="mask_mismatch"):
        contract.compare_masks(mismatches(page), logits(page))
