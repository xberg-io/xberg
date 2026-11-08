"""Compare PP-DocLayoutV3 outputs from two runtimes, tolerating only exact sort ties and boundary logits.

The reading-order column (`fetch_name_0[:, 6]`) is the position of a query in an ascending sort of a
per-query key. Rows whose keys are exactly equal may order either way between runtimes, so a rank
difference is accepted only inside one maximal run of exactly equal keys that both runtimes agree on.

A mask bit is the sign of a gathered logit (`sigmoid(logit) > 0.5`). A flipped bit is accepted only when
the logit is numerically on the boundary in both runtimes, `|logit| <= MASK_LOGIT_EPSILON`.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

# Measured on the frozen development pages: the five flipped bits come from three gathered logits whose
# largest magnitude is 5.998e-5 (pinned ORT) and 1.962e-5 (pinned Paddle), at most 7.96e-5 apart. 1e-4
# covers those with 1.67x headroom; a flip whose logit sits further from zero than that still fails.
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


def compare_ranks(
    reference_ranks: Sequence[int],
    actual_ranks: Sequence[int],
    reference_keys: Sequence[float],
    actual_keys: Sequence[float],
) -> int:
    """Require equal rank orders modulo exact ties; return the number of rows that moved inside a tie.

    `*_keys[rank]` is the sort key at that sorted position in each runtime.
    """
    require(len(reference_ranks) == len(actual_ranks), "rank_mismatch")
    require(len(reference_keys) == len(actual_keys), "rank_mismatch")
    size = len(reference_keys)
    forward: dict[int, int] = {}
    backward: dict[int, int] = {}
    moved = 0
    for expected, got in zip(reference_ranks, actual_ranks, strict=True):
        require(0 <= expected < size and 0 <= got < size, "rank_mismatch")
        require(forward.setdefault(expected, got) == got, "rank_mismatch")
        require(backward.setdefault(got, expected) == expected, "rank_mismatch")
        if expected == got:
            continue
        low, high = tie_run(reference_keys, expected)
        require(low <= got <= high, "rank_mismatch")
        require(tie_run(actual_keys, got) == (low, high), "rank_mismatch")
        require(actual_keys[got] == reference_keys[expected], "rank_mismatch")
        moved += 1
    return moved


def compare_masks(
    mismatches: Sequence[tuple[int, int, int, int, int]],
    logits: Mapping[tuple[int, int, int], tuple[float, float]],
    epsilon: float = MASK_LOGIT_EPSILON,
) -> int:
    """Require every differing mask bit to be a sign flip of a boundary logit; return the flip count.

    `mismatches` holds `(row, y, x, reference_bit, actual_bit)`; `logits` maps `(row, y, x)` to the
    gathered `(reference_logit, actual_logit)`.
    """
    for row, y, x, reference_bit, actual_bit in mismatches:
        require((row, y, x) in logits, "mask_mismatch")
        reference_logit, actual_logit = logits[row, y, x]
        require(reference_bit != actual_bit, "mask_mismatch")
        require(int(reference_logit > 0) == reference_bit and int(actual_logit > 0) == actual_bit, "mask_mismatch")
        require(max(abs(reference_logit), abs(actual_logit)) <= epsilon, "mask_mismatch")
    return len(mismatches)
