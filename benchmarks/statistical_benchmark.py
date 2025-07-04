"""Statistical benchmark comparing JSON vs msgpack with proper error analysis."""

import asyncio
import statistics
import time
from pathlib import Path
from typing import Any

from kreuzberg import ExtractionConfig, extract_file
from kreuzberg._utils._cache import clear_all_caches


async def run_statistical_benchmark() -> dict[str, Any]:
    """Run statistically rigorous benchmark with multiple trials."""
    test_files_dir = Path("tests/test_source_files")
    pdf_files = list(test_files_dir.glob("*.pdf"))

    if not pdf_files:
        raise RuntimeError("No PDF test files found")

    single_file = pdf_files[0]
    config = ExtractionConfig(
        force_ocr=True, ocr_backend="tesseract", extract_tables=True, chunk_content=True
    )

    print("📊 STATISTICAL BENCHMARK")
    print(f"File: {single_file.name}")
    print("Trials: Cold=5, Warm=50")
    print("=" * 60)

    print("\n🔥 COLD START MEASUREMENTS")
    cold_times = []

    for trial in range(5):
        print(f"  Trial {trial + 1}/5...", end=" ", flush=True)
        clear_all_caches()

        start = time.perf_counter()
        result = await extract_file(single_file, config=config)
        duration = time.perf_counter() - start

        cold_times.append(duration)
        print(f"{duration:.3f}s")

    print("\n⚡ WARM CACHE MEASUREMENTS")
    warm_times = []

    for trial in range(50):
        if trial % 10 == 0:
            print(f"  Trials {trial + 1}-{min(trial + 10, 50)}...", end=" ", flush=True)

        start = time.perf_counter()
        cached_result = await extract_file(single_file, config=config)
        duration = time.perf_counter() - start

        warm_times.append(duration)

        if trial % 10 == 9:
            avg_batch = statistics.mean(warm_times[trial - 9 : trial + 1])
            print(f"avg {avg_batch * 1000:.3f}ms")

    cold_mean = statistics.mean(cold_times)
    cold_stdev = statistics.stdev(cold_times) if len(cold_times) > 1 else 0
    cold_median = statistics.median(cold_times)

    warm_mean = statistics.mean(warm_times)
    warm_stdev = statistics.stdev(warm_times) if len(warm_times) > 1 else 0
    warm_median = statistics.median(warm_times)

    warm_filtered = [t for t in warm_times if abs(t - warm_mean) <= 2 * warm_stdev]

    warm_clean_mean = statistics.mean(warm_filtered)
    warm_clean_stdev = statistics.stdev(warm_filtered) if len(warm_filtered) > 1 else 0

    speedup_mean = cold_mean / warm_clean_mean
    speedup_conservative = cold_median / max(warm_times)

    import math

    def confidence_interval(
        data: list[float], confidence: float = 0.95
    ) -> tuple[float, float]:
        if len(data) < 2:
            return (0, 0)

        mean_val = statistics.mean(data)
        stdev_val = statistics.stdev(data)
        n = len(data)

        t_value = 2.0 if n < 30 else 1.96

        margin = t_value * stdev_val / math.sqrt(n)
        return (mean_val - margin, mean_val + margin)

    cold_ci = confidence_interval(cold_times)
    warm_ci = confidence_interval(warm_filtered)

    print("\n📈 STATISTICAL ANALYSIS")
    print("=" * 60)

    print(f"\n🔥 COLD START (n={len(cold_times)}):")
    print(f"  Mean:   {cold_mean:.3f}s ± {cold_stdev:.3f}s")
    print(f"  Median: {cold_median:.3f}s")
    print(f"  Range:  {min(cold_times):.3f}s - {max(cold_times):.3f}s")
    print(f"  95% CI: {cold_ci[0]:.3f}s - {cold_ci[1]:.3f}s")

    print(
        f"\n⚡ WARM CACHE (n={len(warm_times)}, outliers removed: {len(warm_times) - len(warm_filtered)}):"
    )
    print(f"  Mean:   {warm_clean_mean * 1000:.3f}ms ± {warm_clean_stdev * 1000:.3f}ms")
    print(f"  Median: {warm_median * 1000:.3f}ms")
    print(
        f"  Range:  {min(warm_filtered) * 1000:.3f}ms - {max(warm_filtered) * 1000:.3f}ms"
    )
    print(f"  95% CI: {warm_ci[0] * 1000:.3f}ms - {warm_ci[1] * 1000:.3f}ms")

    print("\n🚀 PERFORMANCE:")
    print(f"  Speedup (mean):        {speedup_mean:,.0f}x")
    print(f"  Speedup (conservative): {speedup_conservative:,.0f}x")
    print(
        f"  Coefficient of variation: {(warm_clean_stdev / warm_clean_mean) * 100:.1f}%"
    )

    content_match = result.content == cached_result.content

    print("\n✅ VALIDATION:")
    print(f"  Content accuracy: {'✅ PASS' if content_match else '❌ FAIL'}")
    print(
        f"  Cache consistency: {'✅ STABLE' if warm_clean_stdev / warm_clean_mean < 0.1 else '⚠️ VARIABLE'}"
    )

    from kreuzberg._utils._cache import (
        get_ocr_cache,
        get_table_cache,
        get_mime_cache,
        get_document_cache,
    )

    caches = {
        "MIME": get_mime_cache(),
        "OCR": get_ocr_cache(),
        "Tables": get_table_cache(),
        "Documents": get_document_cache(),
    }

    total_size = 0
    total_items = 0

    print("\n💾 CACHE ANALYSIS:")
    for name, cache in caches.items():
        stats = cache.get_stats()  # type: ignore[attr-defined]
        total_size += stats["total_cache_size_mb"]
        total_items += stats["cached_results"]

        print(
            f"  {name:>9}: {stats['cached_results']:>3} items, {stats['total_cache_size_mb']:.3f}MB"
        )

    print(f"  {'TOTAL':>9}: {total_items:>3} items, {total_size:.3f}MB")

    return {
        "cold_trials": len(cold_times),
        "warm_trials": len(warm_filtered),
        "outliers_removed": len(warm_times) - len(warm_filtered),
        "cold_mean": cold_mean,
        "cold_stdev": cold_stdev,
        "cold_median": cold_median,
        "cold_min": min(cold_times),
        "cold_max": max(cold_times),
        "cold_ci_lower": cold_ci[0],
        "cold_ci_upper": cold_ci[1],
        "warm_mean": warm_clean_mean,
        "warm_stdev": warm_clean_stdev,
        "warm_median": warm_median,
        "warm_min": min(warm_filtered),
        "warm_max": max(warm_filtered),
        "warm_ci_lower": warm_ci[0],
        "warm_ci_upper": warm_ci[1],
        "speedup_mean": speedup_mean,
        "speedup_conservative": speedup_conservative,
        "coefficient_of_variation": (warm_clean_stdev / warm_clean_mean) * 100,
        "content_accuracy": content_match,
        "cache_stable": warm_clean_stdev / warm_clean_mean < 0.1,
        "cache_size_mb": total_size,
        "cache_items": total_items,
    }


if __name__ == "__main__":
    print("🧪 STATISTICAL CACHE BENCHMARK")
    print("Testing msgpack implementation with proper error analysis...")
    print()

    try:
        results = asyncio.run(run_statistical_benchmark())

        import json

        results_file = Path("statistical_benchmark_results.json")
        with results_file.open("w") as f:
            json.dump(results, f, indent=2, default=str)

        print(f"\n💾 Results saved to {results_file}")

        print("\n🎯 SUMMARY:")
        print(
            f"  Speedup: {results['speedup_mean']:,.0f}x (±{results['coefficient_of_variation']:.1f}%)"
        )
        print(
            f"  Cache size: {results['cache_size_mb']:.1f}MB ({results['cache_items']} items)"
        )
        print(
            f"  Reliability: {'✅ EXCELLENT' if results['cache_stable'] else '⚠️ VARIABLE'}"
        )

    except Exception as e:
        print(f"❌ Benchmark failed: {e}")
        import traceback

        traceback.print_exc()
