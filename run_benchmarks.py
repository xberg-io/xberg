"""Quick benchmark runner for Kreuzberg sync vs async performance.
This is a simplified version that works with the current setup.
"""

from __future__ import annotations

import asyncio
import json
import platform
import sys
import time
from pathlib import Path
from typing import Any, Callable

import psutil

from kreuzberg import batch_extract_file, extract_file, extract_file_sync


def collect_system_info() -> dict[str, Any]:
    """Collect system information."""
    cpu_info = psutil.cpu_freq()
    memory_info = psutil.virtual_memory()

    return {
        "platform": platform.platform(),
        "python_version": sys.version,
        "cpu_count": psutil.cpu_count(),
        "cpu_freq_max": cpu_info.max if cpu_info else 0.0,
        "memory_total_gb": memory_info.total / (1024**3),
        "architecture": platform.architecture()[0],
        "machine": platform.machine(),
    }


def benchmark_sync_function(name: str, func: Callable[..., Any], *args: Any, **kwargs: Any) -> dict[str, Any]:
    """Benchmark a synchronous function."""
    process = psutil.Process()
    start_memory = process.memory_info().rss / (1024 * 1024)

    start_time = time.perf_counter()
    try:
        result = func(*args, **kwargs)
        duration = time.perf_counter() - start_time

        end_memory = process.memory_info().rss / (1024 * 1024)
        memory_used = end_memory - start_memory

        return {
            "name": name,
            "success": True,
            "duration_seconds": duration,
            "memory_mb": memory_used,
            "result_size": len(result.content) if hasattr(result, "content") else len(str(result)),
        }
    except Exception as e:  # noqa: BLE001
        duration = time.perf_counter() - start_time
        return {
            "name": name,
            "success": False,
            "duration_seconds": duration,
            "memory_mb": 0,
            "error": str(e),
        }


async def benchmark_async_function(name: str, func: Callable[..., Any], *args: Any, **kwargs: Any) -> dict[str, Any]:
    """Benchmark an asynchronous function."""
    process = psutil.Process()
    start_memory = process.memory_info().rss / (1024 * 1024)

    start_time = time.perf_counter()
    try:
        if asyncio.iscoroutinefunction(func):
            result = await func(*args, **kwargs)
        else:
            result = func(*args, **kwargs)
        duration = time.perf_counter() - start_time

        end_memory = process.memory_info().rss / (1024 * 1024)
        memory_used = end_memory - start_memory

        return {
            "name": name,
            "success": True,
            "duration_seconds": duration,
            "memory_mb": memory_used,
            "result_size": len(result.content) if hasattr(result, "content") else len(str(result)),
        }
    except Exception as e:  # noqa: BLE001
        duration = time.perf_counter() - start_time
        return {
            "name": name,
            "success": False,
            "duration_seconds": duration,
            "memory_mb": 0,
            "error": str(e),
        }


async def run_comparison_benchmarks() -> list[dict[str, Any]] | None:
    """Run sync vs async comparison benchmarks."""
    test_files_dir = Path("tests/test_source_files")
    if not test_files_dir.exists():
        return None

    test_files: list[Path] = []
    for ext in [".md", ".html", ".pdf", ".docx"]:
        test_files.extend(test_files_dir.glob(f"*{ext}"))

    if not test_files:
        return None

    test_files = test_files[:5]

    results = []

    for test_file in test_files:
        sync_result = benchmark_sync_function(f"sync_{test_file.stem}", extract_file_sync, test_file)
        results.append(sync_result)

        async_result = await benchmark_async_function(f"async_{test_file.stem}", extract_file, test_file)
        results.append(async_result)

    batch_size = 3
    if len(test_files) >= batch_size:
        batch_files = test_files[:batch_size]

        sync_batch_result = benchmark_sync_function(
            "sync_batch_sequential", lambda files: [extract_file_sync(f) for f in files], batch_files
        )
        results.append(sync_batch_result)

        async_batch_result = await benchmark_async_function("async_batch_concurrent", batch_extract_file, batch_files)
        results.append(async_batch_result)

    return results


def print_summary(results: list[dict[str, Any]], _system_info: dict[str, Any]) -> None:
    """Print benchmark summary."""
    successful_results = [r for r in results if r["success"]]
    sync_results = [r for r in successful_results if r["name"].startswith("sync_")]
    async_results = [r for r in successful_results if r["name"].startswith("async_")]

    if sync_results:
        sync_avg = sum(r["duration_seconds"] for r in sync_results) / len(sync_results)

    if async_results:
        async_avg = sum(r["duration_seconds"] for r in async_results) / len(async_results)

    if sync_results and async_results:
        async_avg - sync_avg

    for result in results:
        "✓" if result["success"] else "✗"
        result["duration_seconds"]
        result.get("memory_mb", 0)


def save_results(results: list[dict[str, Any]], system_info: dict[str, Any], output_file: str) -> None:
    """Save results to JSON file."""
    output_data = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "system_info": system_info,
        "summary": {
            "total_benchmarks": len(results),
            "successful_benchmarks": len([r for r in results if r["success"]]),
            "success_rate_percent": len([r for r in results if r["success"]]) / len(results) * 100,
        },
        "results": results,
    }

    Path(output_file).parent.mkdir(parents=True, exist_ok=True)
    with Path(output_file).open("w") as f:
        json.dump(output_data, f, indent=2)


async def main() -> None:
    """Main benchmark runner."""
    system_info = collect_system_info()

    start_time = time.perf_counter()
    results = await run_comparison_benchmarks()
    time.perf_counter() - start_time

    if results:
        print_summary(results, system_info)
        save_results(results, system_info, "benchmark_results.json")
    else:
        pass


if __name__ == "__main__":
    asyncio.run(main())
