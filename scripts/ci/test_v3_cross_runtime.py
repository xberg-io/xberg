import json
import math
import sys
from itertools import pairwise
from pathlib import Path

import pytest
import v3_cross_runtime as contract

sys.path.insert(0, str(Path(__file__).parent / "fixtures"))
import generate_v3_cross_runtime_synthetic as generator

FIXTURE = json.loads((Path(__file__).parent / "fixtures/v3_cross_runtime_synthetic.json").read_text())
MASK_SHAPE = FIXTURE["mask_shape"]
PAGE_ONE, PAGE_TWO = FIXTURE["pages"]
EPSILON = contract.MASK_LOGIT_EPSILON


def logits(page: dict) -> dict[tuple[int, int, int], tuple[float, float]]:
    return {(m["row"], m["y"], m["x"]): (m["reference_logit"], m["actual_logit"]) for m in page["mask_mismatches"]}


def mismatches(page: dict) -> list[tuple[int, int, int, int, int]]:
    return [(m["row"], m["y"], m["x"], m["reference_bit"], m["actual_bit"]) for m in page["mask_mismatches"]]


def rank_args(page: dict) -> tuple[list[int], list[int], list[float], list[float]]:
    return page["reference_ranks"], page["actual_ranks"], page["reference_keys"], page["actual_keys"]


def masks(flips: list, table: dict, **overrides: object) -> int:
    arguments = {"differing_bits": len(flips), "reference_shape": MASK_SHAPE, "actual_shape": MASK_SHAPE}
    return contract.compare_masks(flips, table, **{**arguments, **overrides})


def fails(reason: str, call, *args: object, **kwargs: object) -> None:  # noqa: ANN001
    with pytest.raises(ValueError, match=f"^{reason}$"):
        call(*args, **kwargs)


FLIP = [(0, 0, 0, 1, 0)]


def test_synthetic_page_one_rank_differences_are_all_exact_ties() -> None:
    reference, actual, _, _ = rank_args(PAGE_ONE)
    assert sum(a != b for a, b in zip(reference, actual, strict=True)) == 16
    assert contract.compare_ranks(*rank_args(PAGE_ONE)) == 16


def test_synthetic_page_two_ranks_are_identical() -> None:
    assert PAGE_TWO["reference_ranks"] == PAGE_TWO["actual_ranks"]
    assert contract.compare_ranks(*rank_args(PAGE_TWO)) == 0


def test_synthetic_page_ranks_are_not_a_permutation() -> None:
    assert len(set(PAGE_ONE["reference_ranks"])) == 132 < len(PAGE_ONE["reference_ranks"]) == 300


def test_synthetic_page_two_flips_are_boundary_logits() -> None:
    assert len(PAGE_TWO["mask_mismatches"]) == 5
    assert len(set(logits(PAGE_TWO).values())) == 3
    assert masks(mismatches(PAGE_TWO), logits(PAGE_TWO)) == 5


def test_synthetic_page_one_has_no_flips() -> None:
    assert masks([], {}) == 0


def untied_pair(reference: list[int], keys: list[float]) -> tuple[int, int]:
    present = sorted(set(reference))
    return next((p, q) for p, q in pairwise(present) if keys[p] != keys[q])


def swap_values(ranks: list[int], first: int, second: int) -> list[int]:
    return [second if r == first else first if r == second else r for r in ranks]


def test_a_real_reorder_of_non_tied_rows_fails_on_the_tie_run() -> None:
    reference, actual, ref_keys, act_keys = rank_args(PAGE_ONE)
    first, second = untied_pair(reference, ref_keys)
    swapped = swap_values(actual, first, second)
    fails("rank_outside_tie_run", contract.compare_ranks, reference, swapped, ref_keys, act_keys)


def test_a_swap_between_adjacent_unequal_keys_fails() -> None:
    reference, actual, ref_keys, act_keys = rank_args(PAGE_ONE)
    present = sorted(set(reference))
    first, second = next((p, p + 1) for p in present if p + 1 in present and ref_keys[p] != ref_keys[p + 1])
    swapped = swap_values(actual, first, second)
    fails("rank_outside_tie_run", contract.compare_ranks, reference, swapped, ref_keys, act_keys)


def test_a_nudged_tie_key_is_not_a_tie() -> None:
    reference, actual, ref_keys, act_keys = rank_args(PAGE_ONE)
    moved = next(r for r, a in zip(reference, actual, strict=True) if r != a)
    nudged = list(ref_keys)
    nudged[contract.tie_run(ref_keys, moved)[1]] += 1e-9
    fails("tie_structure_mismatch", contract.compare_ranks, reference, actual, nudged, act_keys)


def test_a_tie_the_other_runtime_does_not_share_fails() -> None:
    reference, actual, ref_keys, act_keys = rank_args(PAGE_ONE)
    moved = next(a for r, a in zip(reference, actual, strict=True) if r != a)
    split = list(act_keys)
    split[contract.tie_run(act_keys, moved)[1]] += 1.0
    fails("tie_structure_mismatch", contract.compare_ranks, reference, actual, ref_keys, split)


def test_unmoved_keys_may_differ_numerically_but_not_in_tie_structure() -> None:
    assert contract.compare_ranks([0, 1], [0, 1], [1.0, 2.0], [1.0, 9.0]) == 0
    fails("tie_structure_mismatch", contract.compare_ranks, [0, 1], [0, 1], [1.0, 2.0], [9.0, 9.0])


def test_a_bool_sort_key_is_not_a_float_key() -> None:
    fails("keys_not_finite_floats", contract.compare_ranks, [0, 1], [0, 1], [False, True], [0.0, 1.0])
    fails("keys_not_finite_floats", contract.compare_ranks, [0, 1], [0, 1], [0.0, 1.0], [0.0, True])


def test_more_rows_than_positions_are_allowed_when_a_query_repeats() -> None:
    assert contract.compare_ranks([0, 1, 0], [0, 1, 0], [1.0, 2.0], [1.0, 2.0]) == 0


def test_a_one_ulp_near_tie_fails_closed() -> None:
    tied = [1.0, 1.0]
    near = [1.0, math.nextafter(1.0, 2.0)]
    fails("tie_structure_mismatch", contract.compare_ranks, [0, 1], [1, 0], tied, near)
    fails("tie_structure_mismatch", contract.compare_ranks, [0, 1], [1, 0], near, tied)


def test_numpy_scalars_are_accepted() -> None:
    np = pytest.importorskip("numpy")
    ranks = [np.int64(0), np.int64(1)]
    keys = [np.float64(5.0), np.float32(5.0)]
    assert contract.compare_ranks(ranks, [np.int64(1), np.int64(0)], keys, keys) == 2
    flips = [(0, 0, 0, np.int32(1), np.int32(0))]
    shape = [np.int64(n) for n in MASK_SHAPE]
    table = {(0, 0, 0): (np.float32(1e-6), np.float64(-1e-6))}
    assert masks(flips, table, differing_bits=np.int64(1), reference_shape=shape, actual_shape=shape) == 1
    fails("mask_logit_not_finite", masks, flips, {(0, 0, 0): (np.float32(1e-6), np.float64("nan"))})
    fails("rank_not_int", contract.compare_ranks, [np.float64(0.0)], [np.float64(0.0)], [1.0], [1.0])


def test_one_rank_cannot_map_to_two_ranks() -> None:
    fails("rank_forward_conflict", contract.compare_ranks, [0, 0], [0, 1], [5.0, 5.0], [5.0, 5.0])


def test_two_ranks_cannot_collapse_onto_one() -> None:
    fails("rank_backward_conflict", contract.compare_ranks, [0, 1], [0, 0], [5.0, 5.0], [5.0, 5.0])


def test_a_tie_inside_one_run_may_order_either_way() -> None:
    assert contract.compare_ranks([0, 1, 2], [1, 0, 2], [5.0, 5.0, 7.0], [5.0, 5.0, 7.0]) == 2


def test_repeated_ranks_for_one_query_are_allowed() -> None:
    assert contract.compare_ranks([0, 0, 1], [1, 1, 0], [5.0, 5.0, 7.0], [5.0, 5.0, 7.0]) == 3


@pytest.mark.parametrize(
    ("args", "reason"),
    [
        (([], [], [], []), "empty_input"),
        (([0], [0], [], []), "empty_input"),
        (([0], [0], [1.0, 2.0], [1.0]), "keys_length_mismatch"),
        (([0, 1], [0], [1.0, 2.0], [1.0, 2.0]), "ranks_length_mismatch"),
        (([0, 1], [0, 1], [math.nan, math.nan], [1.0, 2.0]), "keys_not_finite_floats"),
        (([0, 1], [0, 1], [1.0, 2.0], [1.0, math.inf]), "keys_not_finite_floats"),
        (([0, 1], [0, 1], [3.0, 2.0], [3.0, 2.0]), "keys_not_ascending"),
        (([0, 1], [0, 1], [1.0, 2.0], [2.0, 1.0]), "keys_not_ascending"),
        (([True, 1], [True, 1], [1.0, 2.0], [1.0, 2.0]), "rank_not_int"),
        (([0.0, 1], [0.0, 1], [1.0, 2.0], [1.0, 2.0]), "rank_not_int"),
        (([0, 1], [0, 1.0], [1.0, 2.0], [1.0, 2.0]), "rank_not_int"),
        (([0, 2], [0, 2], [1.0, 2.0], [1.0, 2.0]), "rank_out_of_range"),
        (([0, -1], [0, -1], [1.0, 2.0], [1.0, 2.0]), "rank_out_of_range"),
    ],
)
def test_malformed_rank_input_fails_with_its_own_reason(args: tuple, reason: str) -> None:
    fails(reason, contract.compare_ranks, *args)


def test_a_mask_flip_far_from_the_threshold_fails() -> None:
    fails("mask_logit_beyond_epsilon", masks, FLIP, {(0, 0, 0): (0.01, -0.01)})


def test_a_flip_near_zero_on_one_runtime_only_fails() -> None:
    fails("mask_logit_beyond_epsilon", masks, FLIP, {(0, 0, 0): (1e-6, -0.5)})
    fails("mask_logit_beyond_epsilon", masks, FLIP, {(0, 0, 0): (0.5, -1e-6)})


def test_the_epsilon_bound_is_inclusive_and_tight() -> None:
    assert masks(FLIP, {(0, 0, 0): (EPSILON, -EPSILON)}) == 1
    fails("mask_logit_beyond_epsilon", masks, FLIP, {(0, 0, 0): (EPSILON * 1.01, -EPSILON)})


@pytest.mark.parametrize("bad", [math.nan, math.inf, -math.inf])
def test_a_non_finite_logit_on_either_side_fails(bad: float) -> None:
    fails("mask_logit_not_finite", masks, FLIP, {(0, 0, 0): (1e-6, bad)})
    fails("mask_logit_not_finite", masks, FLIP, {(0, 0, 0): (bad, -1e-6)})


@pytest.mark.parametrize("bad", [math.nan, math.inf, 0.0, -1e-4, True, None])
def test_an_invalid_epsilon_fails(bad: object) -> None:
    fails("epsilon_invalid", masks, FLIP, {(0, 0, 0): (1e-6, -1e-6)}, epsilon=bad)


def test_a_flip_without_logit_evidence_fails() -> None:
    fails("mask_logit_missing", masks, FLIP, {})


def test_evidence_that_disagrees_with_the_bits_fails() -> None:
    fails("mask_bit_logit_disagree", masks, FLIP, {(0, 0, 0): (-1e-6, -2e-6)})


def test_equal_bits_are_not_a_flip() -> None:
    fails("mask_not_a_flip", masks, [(0, 0, 0, 1, 1)], {(0, 0, 0): (1e-6, 2e-6)})


def test_duplicate_mismatch_entries_fail() -> None:
    fails("mask_duplicate_entry", masks, FLIP * 3, {(0, 0, 0): (1e-6, -1e-6)})


def test_a_truncated_mismatch_list_fails() -> None:
    fails("mask_list_incomplete", masks, FLIP, {(0, 0, 0): (1e-6, -1e-6)}, differing_bits=100)


def test_mask_shapes_must_match_and_be_three_dimensional() -> None:
    table = {(0, 0, 0): (1e-6, -1e-6)}
    fails("mask_shape_mismatch", masks, FLIP, table, actual_shape=[300, 200, 199])
    fails("mask_shape_invalid", masks, FLIP, table, reference_shape=[], actual_shape=[])
    fails("mask_shape_invalid", masks, FLIP, table, reference_shape=[200, 200], actual_shape=[200, 200])


def test_a_zero_or_non_integer_mask_dimension_fails() -> None:
    for shape in ([0, 200, 200], [300, 0, 200], [300, 200, -1], [300.0, 200, 200], [300, 200, 200.5]):
        fails("mask_shape_invalid", masks, [], {}, reference_shape=shape, actual_shape=shape)


def test_a_position_outside_the_mask_fails() -> None:
    fails("mask_position_out_of_range", masks, [(300, 0, 0, 1, 0)], {(300, 0, 0): (1e-6, -1e-6)})


def test_a_perturbed_synthetic_flip_fails() -> None:
    page = json.loads(json.dumps(PAGE_TWO))
    page["mask_mismatches"][0]["reference_logit"] = 0.001
    fails("mask_logit_beyond_epsilon", masks, mismatches(page), logits(page))


def canonical(data: object) -> bytes:
    return json.dumps(data, sort_keys=True, separators=(",", ":")).encode()


def test_the_committed_fixture_is_what_the_seeded_generator_produces() -> None:
    assert canonical(generator.build()) == canonical(FIXTURE)
    assert canonical(generator.build()) == canonical(generator.build())


def test_a_changed_seed_no_longer_matches_the_committed_fixture(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(generator, "SEED", generator.SEED + 1)
    assert canonical(generator.build()) != canonical(FIXTURE)


@pytest.mark.parametrize("bits", [(True, False), (1.0, 0), (1, 0.0)])
def test_mask_bits_must_be_plain_integers(bits: tuple) -> None:
    fails("mask_bit_not_int", masks, [(0, 0, 0, *bits)], {(0, 0, 0): (1e-6, -1e-6)})
