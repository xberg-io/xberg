"""Compare PP-DocLayoutV3 outputs from two runtimes, tolerating only exact sort ties and boundary logits.

This is a test utility. It is not yet called by the native-Paddle/ORT replay runner, which must replace
its two `array_equal` checks (column 6 and the masks) with `compare_ranks` and `compare_masks`.

The reading-order column (`fetch_name_0[:, 6]`) is the position of a query in an ascending sort of a
per-query key. Rows whose keys are exactly equal may order either way between runtimes.

A mask bit is the sign of a gathered logit (`sigmoid(logit) > 0.5`). A flipped bit is accepted only when
the logit is numerically on the boundary in both runtimes, `|logit| <= MASK_LOGIT_EPSILON`.
"""

from __future__ import annotations

import math
from itertools import pairwise
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

# Fitted on two development pages, so it must be re-checked on held-out pages: the five flipped bits there
# come from three gathered logits whose largest magnitude is 5.998e-5 (pinned ORT) and 1.962e-5 (pinned
# Paddle), at most 7.96e-5 apart. 1e-4 covers those with 1.67x headroom; a flip whose logit sits further
# from zero than that still fails.
MASK_LOGIT_EPSILON = 1e-4


def require(condition: bool, reason: str) -> None:
    if not condition:
        raise ValueError(reason)


def tie_run(keys: Sequence[float], position: int) -> tuple[int, int]:
    """Return the inclusive bounds of the maximal run of keys exactly equal to `keys[position]`."""
    low = high = position
    while low > 0 and keys[low - 1] == keys[position]:
        low -= 1
    while high + 1 < len(keys) and keys[high + 1] == keys[position]:
        high += 1
    return low, high


def validate_keys(keys: Sequence[float]) -> None:
    require(all(type(k) is float and math.isfinite(k) for k in keys), "keys_not_finite_floats")
    require(all(a <= b for a, b in pairwise(keys)), "keys_not_ascending")


def compare_ranks(
    reference_ranks: Sequence[int],
    actual_ranks: Sequence[int],
    reference_keys: Sequence[float],
    actual_keys: Sequence[float],
) -> int:
    """Require equal rank orders modulo exact ties; return the number of rows that moved inside a tie.

    `*_keys[rank]` is the sort key at that sorted position in each runtime. Policy for the keys:

    - both arrays must be non-empty, equal length, finite and ascending;
    - their exact-tie structure must be identical at every position (the same positions are exactly equal
      to their neighbours), so a tie cannot be invented or lost on either side;
    - non-tie key values may differ numerically between runtimes (observed up to 0.038), they are not compared.

    `*_ranks` hold one rank per output row. Ranks are not a permutation: a query that yields several
    rows repeats its rank (128 distinct values over 300 rows on the frozen page). The row check is
    therefore that a rank value maps to one rank value and back, and that every rank that changed
    stays inside its tie run.
    """
    require(len(reference_keys) > 0 and len(reference_ranks) > 0, "empty_input")
    require(len(reference_keys) == len(actual_keys), "keys_length_mismatch")
    require(len(reference_ranks) == len(actual_ranks), "ranks_length_mismatch")
    size = len(reference_keys)
    require(len(reference_ranks) <= size, "more_rows_than_positions")
    validate_keys(reference_keys)
    validate_keys(actual_keys)
    require(
        all(tie_run(reference_keys, i) == tie_run(actual_keys, i) for i in range(size)),
        "tie_structure_mismatch",
    )
    forward: dict[int, int] = {}
    backward: dict[int, int] = {}
    moved = 0
    for expected, got in zip(reference_ranks, actual_ranks, strict=True):
        require(type(expected) is int and type(got) is int, "rank_not_int")
        require(0 <= expected < size and 0 <= got < size, "rank_out_of_range")
        require(forward.setdefault(expected, got) == got, "rank_forward_conflict")
        require(backward.setdefault(got, expected) == expected, "rank_backward_conflict")
        if expected != got:
            low, high = tie_run(reference_keys, expected)
            require(low <= got <= high, "rank_outside_tie_run")
            moved += 1
    return moved


def compare_masks(
    mismatches: Sequence[tuple[int, int, int, int, int]],
    logits: Mapping[tuple[int, int, int], tuple[float, float]],
    *,
    differing_bits: int,
    reference_shape: Sequence[int],
    actual_shape: Sequence[int],
    epsilon: float = MASK_LOGIT_EPSILON,
) -> int:
    """Require every differing mask bit to be a sign flip of a boundary logit; return the flip count.

    `mismatches` holds `(row, y, x, reference_bit, actual_bit)`; `logits` maps `(row, y, x)` to the
    gathered `(reference_logit, actual_logit)`. The caller must derive `mismatches` from a full diff of
    the two mask arrays and pass that diff's count as `differing_bits` (for example
    `np.count_nonzero(a != b)`) so a truncated list cannot pass.
    """
    require(
        isinstance(epsilon, float | int) and not isinstance(epsilon, bool) and math.isfinite(epsilon) and epsilon > 0,
        "epsilon_invalid",
    )
    require(len(reference_shape) > 0 and all(type(n) is int and n > 0 for n in reference_shape), "mask_shape_invalid")
    require(list(reference_shape) == list(actual_shape), "mask_shape_mismatch")
    require(len(reference_shape) == 3, "mask_shape_invalid")
    require(type(differing_bits) is int and differing_bits == len(mismatches), "mask_list_incomplete")
    require(len({m[:3] for m in mismatches}) == len(mismatches), "mask_duplicate_entry")
    rows, height, width = reference_shape
    for row, y, x, reference_bit, actual_bit in mismatches:
        require(0 <= row < rows and 0 <= y < height and 0 <= x < width, "mask_position_out_of_range")
        require((row, y, x) in logits, "mask_logit_missing")
        reference_logit, actual_logit = logits[row, y, x]
        require(math.isfinite(reference_logit) and math.isfinite(actual_logit), "mask_logit_not_finite")
        require({reference_bit, actual_bit} == {0, 1}, "mask_not_a_flip")
        require(
            int(reference_logit > 0) == reference_bit and int(actual_logit > 0) == actual_bit, "mask_bit_logit_disagree"
        )
        require(abs(reference_logit) <= epsilon and abs(actual_logit) <= epsilon, "mask_logit_beyond_epsilon")
    return len(mismatches)
