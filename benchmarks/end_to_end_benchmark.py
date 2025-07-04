"""End-to-end reproducible benchmark with proper statistics."""

import asyncio
import statistics
import time
from pathlib import Path
from typing import Any

from kreuzberg import ExtractionConfig, extract_file
from kreuzberg._utils._cache import clear_all_caches


async def run_end_to_end_benchmark(trials: int = 20) -> dict[str, Any]:
    """Run end-to-end benchmark with proper statistical analysis."""

    test_files_dir = Path("tests/test_source_files")
    pdf_files = list(test_files_dir.glob("*.pdf"))

    if not pdf_files:
        raise RuntimeError("No PDF test files found")

    single_file = pdf_files[0]
    config = ExtractionConfig(
        force_ocr=True, ocr_backend="tesseract", extract_tables=True, chunk_content=True
    )

    print("🎯 END-TO-END CACHE BENCHMARK")
    print(f"File: {single_file.name}")
    print(f"Warm cache trials: {trials}")
    print("=" * 60)

    print("\n🔥 COLD RUN (cache population)")
    clear_all_caches()

    start = time.perf_counter()
    cold_result = await extract_file(single_file, config=config)
    cold_duration = time.perf_counter() - start

    print(f"Cold extraction: {cold_duration:.3f}s")
    print(f"Content length: {len(cold_result.content):,} chars")
    print(f"Tables: {len(cold_result.tables)}")
    print(f"Chunks: {len(cold_result.chunks)}")

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

    total_items = sum(cache.get_stats()["cached_results"] for cache in caches.values())  # type: ignore[attr-defined]
    total_size = sum(
        cache.get_stats()["total_cache_size_mb"]  # type: ignore[attr-defined]
        for cache in caches.values()
    )

    print(f"Cache populated: {total_items} items, {total_size:.3f}MB")

    print(f"\n⚡ WARM RUNS (n={trials})")
    warm_times = []

    for trial in range(trials):
        if trial % 5 == 0:
            print(f"  Trial {trial + 1:2d}/{trials}...", end=" ", flush=True)

        start = time.perf_counter()
        warm_result = await extract_file(single_file, config=config)
        duration = time.perf_counter() - start

        warm_times.append(duration)

        if trial % 5 == 4:
            recent_avg = statistics.mean(warm_times[max(0, trial - 4) : trial + 1])
            print(f"avg {recent_avg * 1000:.3f}ms")

    content_match = cold_result.content == warm_result.content
    tables_match = len(cold_result.tables) == len(warm_result.tables)

    warm_mean = statistics.mean(warm_times)
    warm_stdev = statistics.stdev(warm_times) if len(warm_times) > 1 else 0
    warm_median = statistics.median(warm_times)
    warm_min = min(warm_times)
    warm_max = max(warm_times)

    outlier_threshold = 2 * warm_stdev
    warm_filtered = [t for t in warm_times if abs(t - warm_mean) <= outlier_threshold]
    outliers_removed = len(warm_times) - len(warm_filtered)

    if warm_filtered:
        warm_clean_mean = statistics.mean(warm_filtered)
        warm_clean_stdev = (
            statistics.stdev(warm_filtered) if len(warm_filtered) > 1 else 0
        )
    else:
        warm_clean_mean = warm_mean
        warm_clean_stdev = warm_stdev

    import math

    if len(warm_filtered) > 1:
        t_value = 2.09 if len(warm_filtered) < 20 else 1.96
        margin_of_error = t_value * warm_clean_stdev / math.sqrt(len(warm_filtered))
        ci_lower = warm_clean_mean - margin_of_error
        ci_upper = warm_clean_mean + margin_of_error
    else:
        ci_lower = ci_upper = warm_clean_mean

    speedup_mean = cold_duration / warm_clean_mean
    speedup_conservative = cold_duration / warm_max

    cv = (warm_clean_stdev / warm_clean_mean) * 100 if warm_clean_mean > 0 else 0

    print("\n📊 STATISTICAL RESULTS")
    print("=" * 60)

    print("\n🔥 COLD PERFORMANCE:")
    print(f"  Duration: {cold_duration:.3f}s")
    print(f"  Content:  {len(cold_result.content):,} chars")
    print(f"  Tables:   {len(cold_result.tables)}")

    print(
        f"\n⚡ WARM PERFORMANCE (n={len(warm_filtered)}, outliers removed: {outliers_removed}):"
    )
    print(
        f"  Mean:     {warm_clean_mean * 1000:.3f}ms ± {warm_clean_stdev * 1000:.3f}ms"
    )
    print(f"  Median:   {warm_median * 1000:.3f}ms")
    print(f"  Range:    {warm_min * 1000:.3f}ms - {warm_max * 1000:.3f}ms")
    print(f"  95% CI:   {ci_lower * 1000:.3f}ms - {ci_upper * 1000:.3f}ms")
    print(
        f"  CV:       {cv:.1f}% ({'excellent' if cv < 5 else 'good' if cv < 10 else 'variable'})"
    )

    print("\n🚀 PERFORMANCE GAIN:")
    print(f"  Speedup (mean):        {speedup_mean:,.0f}x")
    print(f"  Speedup (conservative): {speedup_conservative:,.0f}x")
    print(
        f"  Time saved:            {cold_duration - warm_clean_mean:.3f}s ({((cold_duration - warm_clean_mean) / cold_duration) * 100:.1f}%)"
    )

    print("\n✅ VALIDATION:")
    print(f"  Content accuracy:  {'✅ PASS' if content_match else '❌ FAIL'}")
    print(f"  Tables consistency: {'✅ PASS' if tables_match else '❌ FAIL'}")
    print(f"  Performance stable: {'✅ YES' if cv < 10 else '⚠️ VARIABLE'}")

    print("\n💾 CACHE EFFICIENCY:")
    for name, cache in caches.items():
        stats = cache.get_stats()  # type: ignore[attr-defined]
        if stats["cached_results"] > 0:
            efficiency = stats["cached_results"] / max(
                stats["total_cache_size_mb"], 0.001
            )
            print(
                f"  {name:>9}: {stats['cached_results']:>3} items, {stats['total_cache_size_mb']:>7.3f}MB ({efficiency:>5.0f} items/MB)"
            )

    print(f"  {'TOTAL':>9}: {total_items:>3} items, {total_size:>7.3f}MB")

    return {
        "trials": trials,
        "outliers_removed": outliers_removed,
        "file_name": single_file.name,
        "cold_duration": cold_duration,
        "cold_content_length": len(cold_result.content),
        "cold_tables": len(cold_result.tables),
        "cold_chunks": len(cold_result.chunks),
        "warm_mean": warm_clean_mean,
        "warm_stdev": warm_clean_stdev,
        "warm_median": warm_median,
        "warm_min": warm_min,
        "warm_max": warm_max,
        "warm_ci_lower": ci_lower,
        "warm_ci_upper": ci_upper,
        "coefficient_of_variation": cv,
        "speedup_mean": speedup_mean,
        "speedup_conservative": speedup_conservative,
        "time_saved_seconds": cold_duration - warm_clean_mean,
        "time_saved_percent": ((cold_duration - warm_clean_mean) / cold_duration) * 100,
        "content_accuracy": content_match,
        "tables_consistency": tables_match,
        "performance_stable": cv < 10,
        "cache_items": total_items,
        "cache_size_mb": total_size,
        "warm_times_raw": warm_times,
        "warm_times_filtered": warm_filtered,
    }


if __name__ == "__main__":
    print("🧪 REPRODUCIBLE CACHE BENCHMARK")
    print("Testing msgpack implementation with statistical rigor...")
    print()

    try:
        results = asyncio.run(run_end_to_end_benchmark(trials=30))

        import json
        from datetime import datetime

        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        results_file = Path(f"benchmark_msgpack_{timestamp}.json")

        with results_file.open("w") as f:
            json.dump(results, f, indent=2, default=str)

        print(f"\n💾 Results saved to {results_file}")

        print("\n🎯 EXECUTIVE SUMMARY")
        print("=" * 60)
        print(
            f"🚀 Performance:   {results['speedup_mean']:,.0f}x speedup ({results['coefficient_of_variation']:.1f}% CV)"
        )
        print(
            f"💾 Cache size:    {results['cache_size_mb']:.1f}MB ({results['cache_items']} items)"
        )
        print(
            f"⚡ Consistency:   {'✅ Excellent' if results['coefficient_of_variation'] < 5 else '✅ Good' if results['coefficient_of_variation'] < 10 else '⚠️ Variable'}"
        )
        print(
            f"✅ Accuracy:     {'✅ Perfect' if results['content_accuracy'] and results['tables_consistency'] else '❌ Issues detected'}"
        )

        if results["performance_stable"] and results["content_accuracy"]:
            print("\n🏆 VERDICT: MSGPACK IMPLEMENTATION IS PRODUCTION READY")
        else:
            print("\n⚠️ VERDICT: NEEDS INVESTIGATION")

    except Exception as e:
        print(f"❌ Benchmark failed: {e}")
        import traceback

        traceback.print_exc()
