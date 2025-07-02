"""Compare benchmark results for performance regression detection."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def load_benchmark_results(file_path: Path) -> dict[str, Any]:
    """Load benchmark results from JSON file."""
    try:
        with file_path.open() as f:
            data: dict[str, Any] = json.load(f)
            return data
    except (FileNotFoundError, json.JSONDecodeError) as e:
        print(f"Error loading benchmark results: {e}")  # noqa: T201
        sys.exit(1)


def compare_benchmarks(baseline: dict[str, Any], current: dict[str, Any], threshold: float = 0.2) -> bool:
    """Compare benchmark results and detect regressions.

    Args:
        baseline: Baseline benchmark results
        current: Current benchmark results
        threshold: Performance regression threshold (e.g., 0.2 = 20% slower)

    Returns:
        True if no significant regressions detected, False otherwise
    """
    baseline_benchmarks = {b["name"]: b for b in baseline["benchmarks"]}
    current_benchmarks = {b["name"]: b for b in current["benchmarks"]}

    regressions = []
    improvements = []

    for name, current_bench in current_benchmarks.items():
        if name not in baseline_benchmarks:
            continue

        baseline_bench = baseline_benchmarks[name]

        if not current_bench.get("success", True) or not baseline_bench.get("success", True):
            continue

        baseline_duration = baseline_bench["duration"]
        current_duration = current_bench["duration"]

        if baseline_duration > 0:
            change_ratio = (current_duration - baseline_duration) / baseline_duration
            change_percent = change_ratio * 100

            improvement_threshold = -0.05
            if change_ratio > threshold:
                regressions.append((name, change_percent, baseline_duration, current_duration))
            elif change_ratio < improvement_threshold:
                improvements.append((name, abs(change_percent), baseline_duration, current_duration))

    print(f"Found {len(improvements)} improvements and {len(regressions)} regressions")  # noqa: T201

    if improvements:
        for name, improvement, baseline_dur, current_dur in improvements:
            print(f"IMPROVEMENT: {name} {improvement:.1f}% faster ({baseline_dur:.3f}s -> {current_dur:.3f}s)")  # noqa: T201

    if regressions:
        for name, regression, baseline_dur, current_dur in regressions:
            print(f"REGRESSION: {name} {regression:.1f}% slower ({baseline_dur:.3f}s -> {current_dur:.3f}s)")  # noqa: T201
        return False

    return True


def main() -> None:
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Compare benchmark results for performance regression detection")
    parser.add_argument("baseline", type=Path, help="Path to baseline benchmark results JSON")
    parser.add_argument("current", type=Path, help="Path to current benchmark results JSON")
    parser.add_argument(
        "--threshold", type=float, default=0.2, help="Performance regression threshold as decimal (default: 0.2 = 20%%)"
    )
    parser.add_argument(
        "--fail-on-regression", action="store_true", help="Exit with non-zero code if regressions detected"
    )

    args = parser.parse_args()

    baseline = load_benchmark_results(args.baseline)
    current = load_benchmark_results(args.current)

    no_regressions = compare_benchmarks(baseline, current, args.threshold)

    if args.fail_on_regression and not no_regressions:
        sys.exit(1)


if __name__ == "__main__":
    main()
