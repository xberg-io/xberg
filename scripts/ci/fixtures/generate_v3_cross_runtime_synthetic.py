"""Generate v3_cross_runtime_synthetic.json: seeded synthetic data, derived from no real document or model output.

Usage: python3 generate_v3_cross_runtime_synthetic.py [OUTPUT]  (default: print to stdout)
"""

from __future__ import annotations

import json
import random
import sys

SEED = 20260108
POSITIONS = 300
TIE_STARTS = (40, 120, 200, 260)
MASK_SHAPE = [300, 200, 200]
DESCRIPTION = (
    "Synthetic evidence shaped like a PP-DocLayoutV3 cross-runtime comparison: page 1 has four exact two-way "
    "sort ties whose rows swap rank, page 2 has five mask-bit flips from three boundary logits. Generated "
    "with a fixed seed by generate_v3_cross_runtime_synthetic.py; derived from no real document and no model "
    "output. The 1e-4 epsilon was fitted on real development pages and must be re-checked on held-out pages."
)
FLIPS = [
    (11, 5, 9, 2.2e-6, -1.6e-6),
    (40, 5, 9, 2.2e-6, -1.6e-6),
    (77, 31, 150, 4.1e-5, -1.2e-5),
    (120, 88, 12, 5.5e-6, -9.0e-6),
    (205, 88, 12, 5.5e-6, -9.0e-6),
]


def make_keys(rng: random.Random, tie_starts: tuple[int, ...], jitter: float) -> tuple[list[float], list[float]]:
    value = 10.0
    reference = []
    for _ in range(POSITIONS):
        value += rng.uniform(0.5, 3.0)
        reference.append(round(value, 6))
    for low in tie_starts:
        reference[low + 1] = reference[low]
        for j in range(low + 2, POSITIONS):
            if reference[j] <= reference[j - 1]:
                reference[j] = reference[j - 1] + 0.25
    tied = {t for low in tie_starts for t in (low, low + 1)}
    actual = [k + rng.uniform(-jitter, jitter) if i not in tied else k for i, k in enumerate(reference)]
    for low in tie_starts:
        actual[low + 1] = actual[low]
    for j in range(1, POSITIONS):
        if actual[j] < actual[j - 1]:
            actual[j] = actual[j - 1] + 0.01
    return reference, actual


def build() -> dict:
    rng = random.Random(SEED)
    ref_keys, act_keys = make_keys(rng, TIE_STARTS, 0.03)
    positions = sorted(rng.sample(range(POSITIONS), 128))
    for start in TIE_STARTS:
        for p in (start, start + 1):
            if p not in positions:
                positions.append(p)
    rows = ([rng.choice(positions) for _ in range(POSITIONS - len(positions))] + positions)[:POSITIONS]
    swap = {}
    for start in TIE_STARTS:
        swap[start], swap[start + 1] = start + 1, start
    page_one = {
        "page": "synthetic-1",
        "reference_ranks": rows,
        "actual_ranks": [swap.get(p, p) for p in rows],
        "reference_keys": ref_keys,
        "actual_keys": act_keys,
        "mask_mismatches": [],
    }
    ref_keys2, act_keys2 = make_keys(rng, (), 0.02)
    rows2 = [rng.choice(range(POSITIONS)) for _ in range(POSITIONS)]
    page_two = {
        "page": "synthetic-2",
        "reference_ranks": rows2,
        "actual_ranks": list(rows2),
        "reference_keys": ref_keys2,
        "actual_keys": act_keys2,
        "mask_mismatches": [
            {
                "row": row,
                "y": y,
                "x": x,
                "reference_bit": 1,
                "actual_bit": 0,
                "reference_logit": reference_logit,
                "actual_logit": actual_logit,
            }
            for row, y, x, reference_logit, actual_logit in FLIPS
        ],
    }
    return {"description": DESCRIPTION, "mask_shape": MASK_SHAPE, "pages": [page_one, page_two]}


if __name__ == "__main__":
    text = json.dumps(build(), indent=2) + "\n"
    if len(sys.argv) > 1:
        with open(sys.argv[1], "w") as out:
            out.write(text)
    else:
        sys.stdout.write(text)
