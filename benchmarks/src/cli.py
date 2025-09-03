"""Command-line interface for Kreuzberg benchmarks."""

# type: ignore[index,unused-ignore]

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import typer
from rich.console import Console
from rich.table import Table

from .benchmarks import KreuzbergBenchmarks
from .models import FlameGraphConfig
from .runner import BenchmarkRunner

app = typer.Typer(help="Kreuzberg Performance Benchmarking Suite")
console = Console()


def _generate_quality_report(data: dict[str, Any], console: Console) -> None:
    """Generate metadata quality report from benchmark results."""
    console.print("\n[bold]METADATA QUALITY REPORT[/bold]")
    console.print("=" * 80)

    quality_results = [
        r
        for r in data["results"]
        if r.get("extraction_quality")
        and r.get("extraction_quality", {}).get("metadata_quality")
    ]

    if not quality_results:
        console.print("[yellow]No extraction quality data available[/yellow]")
        return

    backend_stats: dict[str, dict[str, Any]] = {}
    for result in quality_results:
        metadata = result.get("metadata", {})
        backend = metadata.get("backend", "unknown")

        if backend not in backend_stats:
            backend_stats[backend] = {
                "results": [],
                "file_types": set(),
                "total_metadata_fields": 0,
                "unique_fields": set(),
                "avg_completeness": 0,
                "avg_richness": 0,
            }

        quality = result["extraction_quality"]["metadata_quality"]
        backend_stats[backend]["results"].append(quality)
        backend_stats[backend]["file_types"].add(metadata.get("file_type", "unknown"))
        backend_stats[backend]["total_metadata_fields"] += quality["metadata_count"]
        backend_stats[backend]["unique_fields"].update(quality["metadata_fields"])

    for backend, stats in backend_stats.items():
        results = stats["results"]
        if results:
            stats["avg_completeness"] = sum(
                r["metadata_completeness"] for r in results
            ) / len(results)
            stats["avg_richness"] = sum(r["metadata_richness"] for r in results) / len(
                results
            )
            stats["avg_metadata_count"] = stats["total_metadata_fields"] / len(results)

    table = Table(title="Backend Metadata Quality Comparison")
    table.add_column("Backend", style="bold")
    table.add_column("Files Tested", justify="right")
    table.add_column("Avg Fields", justify="right")
    table.add_column("Unique Fields", justify="right")
    table.add_column("Completeness %", justify="right")
    table.add_column("Richness Score", justify="right")

    for backend in sorted(backend_stats.keys()):
        stats = backend_stats[backend]
        table.add_row(
            backend,
            str(len(stats["results"])),
            f"{stats['avg_metadata_count']:.1f}",
            str(len(stats["unique_fields"])),
            f"{stats['avg_completeness']:.1f}%",
            f"{stats['avg_richness']:.2f}",
        )

    console.print(table)

    console.print("\n[bold]Metadata Quality by File Type and Backend:[/bold]")

    file_type_stats: dict[str, dict[str, list[Any]]] = {}
    for result in quality_results:
        metadata = result.get("metadata", {})
        file_type = metadata.get("file_type", "unknown")
        backend = metadata.get("backend", "unknown")
        quality = result["extraction_quality"]["metadata_quality"]

        if file_type not in file_type_stats:
            file_type_stats[file_type] = {}

        if backend not in file_type_stats[file_type]:
            file_type_stats[file_type][backend] = []

        file_type_stats[file_type][backend].append(quality)

    for file_type in sorted(file_type_stats.keys()):
        console.print(f"\n{file_type.upper()}:")
        for backend in sorted(file_type_stats[file_type].keys()):
            qualities = file_type_stats[file_type][backend]
            avg_count = sum(q["metadata_count"] for q in qualities) / len(qualities)
            has_title = (
                sum(1 for q in qualities if q["has_title"]) / len(qualities) * 100
            )
            has_author = (
                sum(1 for q in qualities if q["has_author"]) / len(qualities) * 100
            )

            console.print(
                f"  {backend}: {avg_count:.1f} fields, "
                f"{has_title:.0f}% with title, {has_author:.0f}% with author"
            )

    console.print("\n[bold]Most Common Metadata Fields:[/bold]")
    all_fields: dict[str, set[str]] = {}
    for backend, stats in backend_stats.items():
        for field in stats["unique_fields"]:
            if field not in all_fields:
                all_fields[field] = set()
            all_fields[field].add(backend)

    sorted_fields = sorted(all_fields.items(), key=lambda x: len(x[1]), reverse=True)[
        :20
    ]

    for field, backends in sorted_fields:
        console.print(f"  {field}: {', '.join(sorted(backends))}")


@app.command()
def run(
    output_dir: Path = typer.Option(
        Path("results"),
        "--output-dir",
        "-o",
        help="Directory to save benchmark results",
    ),  # noqa: B008
    test_files_dir: Path | None = typer.Option(
        None,
        "--test-files-dir",
        "-t",
        help="Directory containing test files for benchmarking",
    ),
    include_flame: bool = typer.Option(
        False, "--flame", "-f", help="Generate flame graphs for performance profiling"
    ),
    suite_name: str = typer.Option(
        "kreuzberg_sync_vs_async",
        "--suite-name",
        "-s",
        help="Name of the benchmark suite",
    ),
    sync_only: bool = typer.Option(
        False, "--sync-only", help="Run only synchronous benchmarks"
    ),
    async_only: bool = typer.Option(
        False, "--async-only", help="Run only asynchronous benchmarks"
    ),
    comparison_only: bool = typer.Option(
        False,
        "--comparison-only",
        help="Run only direct sync vs async comparison benchmarks",
    ),
    include_stress: bool = typer.Option(
        False, "--stress", help="Include stress test benchmarks"
    ),
    backend_comparison: bool = typer.Option(
        False, "--backend-comparison", help="Run backend comparison benchmarks"
    ),
) -> None:
    """Run Kreuzberg performance benchmarks."""
    console.print("[bold blue]Kreuzberg Performance Benchmarks[/bold blue]")
    console.print(f"Suite: {suite_name}")

    benchmarks = KreuzbergBenchmarks(test_files_dir)

    if not benchmarks.test_files:
        console.print("[red]Error:[/red] No test files found for benchmarking")
        console.print(f"Looking in: {benchmarks.test_files_dir}")
        console.print("Please ensure test files exist or specify --test-files-dir")
        raise typer.Exit(1)

    console.print(f"Found {len(benchmarks.test_files)} test files")

    flame_config = FlameGraphConfig(enabled=include_flame) if include_flame else None

    runner = BenchmarkRunner(console=console, flame_config=flame_config)

    sync_benchmarks: list[tuple[str, Any, dict[str, Any]]] = []
    async_benchmarks: list[tuple[str, Any, dict[str, Any]]] = []

    if backend_comparison:
        backend_benchmarks = benchmarks.get_backend_benchmarks()
        sync_benchmarks = backend_benchmarks
        async_benchmarks = []
    elif comparison_only:
        comparison_benchmarks = benchmarks.get_comparison_benchmarks()

        sync_benchmarks = [
            (name, func, meta)
            for name, func, meta in comparison_benchmarks
            if meta.get("type") == "sync"
        ]
        async_benchmarks = [
            (name, func, meta)
            for name, func, meta in comparison_benchmarks
            if meta.get("type") == "async"
        ]
    else:
        if not async_only:
            sync_benchmarks = benchmarks.get_sync_benchmarks()
            if include_stress:
                sync_benchmarks.extend(benchmarks.get_stress_benchmarks())

        if not sync_only:
            async_benchmarks = benchmarks.get_async_benchmarks()
            if include_stress:
                async_benchmarks.extend(benchmarks.get_stress_benchmarks())

    if not sync_benchmarks and not async_benchmarks:
        console.print("[red]Error:[/red] No benchmarks to run")
        raise typer.Exit(1)

    console.print(
        f"Running {len(sync_benchmarks)} sync + {len(async_benchmarks)} async benchmarks"
    )

    try:
        suite = runner.run_benchmark_suite(
            suite_name=suite_name,
            benchmarks=sync_benchmarks,  # type: ignore[arg-type]
            async_benchmarks=async_benchmarks if async_benchmarks else None,  # type: ignore[arg-type]
        )

        runner.print_summary(suite)

        output_dir.mkdir(parents=True, exist_ok=True)
        output_file = output_dir / f"{suite_name}.json"
        runner.save_results(suite, output_file)

        latest_file = output_dir / "latest.json"
        runner.save_results(suite, latest_file)

        console.print("\n[green]Benchmark completed successfully![/green]")
        console.print(f"Results saved to: {output_file}")

    except KeyboardInterrupt:
        console.print("\n[yellow]Benchmark interrupted by user[/yellow]")
        raise typer.Exit(1)
    except Exception as e:
        console.print(f"\n[red]Benchmark failed:[/red] {e}")
        raise typer.Exit(1)


@app.command()
def compare(
    result1: Path = typer.Argument(..., help="First benchmark result file"),
    result2: Path = typer.Argument(..., help="Second benchmark result file"),
    output: Path | None = typer.Option(
        None, "--output", "-o", help="Save comparison to file"
    ),
) -> None:
    """Compare two benchmark results."""
    console.print("[bold blue]Benchmark Comparison[/bold blue]")

    try:
        with open(result1) as f:
            data1 = json.load(f)
        with open(result2) as f:
            data2 = json.load(f)
    except Exception as e:
        console.print(f"[red]Error loading files:[/red] {e}")
        raise typer.Exit(1)

    console.print("Comparing:")
    console.print(f"  {result1.name}: {data1['name']} ({data1['timestamp']})")
    console.print(f"  {result2.name}: {data2['name']} ({data2['timestamp']})")

    rate1 = data1["summary"]["success_rate_percent"]
    rate2 = data2["summary"]["success_rate_percent"]
    console.print(
        f"\nSuccess Rate: {rate1:.1f}% vs {rate2:.1f}% ({rate2 - rate1:+.1f}%)"
    )

    duration1 = data1["summary"]["total_duration_seconds"]
    duration2 = data2["summary"]["total_duration_seconds"]
    console.print(
        f"Total Duration: {duration1:.3f}s vs {duration2:.3f}s ({duration2 - duration1:+.3f}s)"
    )

    if output:
        comparison_data = {
            "comparison_timestamp": datetime.now(timezone.utc).isoformat(),
            "file1": {"path": str(result1), "data": data1},
            "file2": {"path": str(result2), "data": data2},
            "summary": {
                "success_rate_diff": rate2 - rate1,
                "duration_diff": duration2 - duration1,
            },
        }

        with open(output, "w") as f:
            json.dump(comparison_data, f, indent=2)
        console.print(f"\nComparison saved to: {output}")


@app.command()
def tesseract() -> None:
    """Run Tesseract OCR output format benchmarks (DEPRECATED)."""
    console.print(
        "[yellow]This command is deprecated. Use 'baseline', 'statistical', or 'serialization' commands instead.[/yellow]"
    )


@app.command()
def analyze(
    result_file: Path = typer.Argument(..., help="Benchmark result file to analyze"),
    quality_report: bool = typer.Option(
        False, "--quality", "-q", help="Generate metadata quality report"
    ),
) -> None:
    """Analyze benchmark results and generate insights."""
    console.print("[bold blue]Benchmark Analysis[/bold blue]")

    try:
        with open(result_file) as f:
            data = json.load(f)
    except Exception as e:
        console.print(f"[red]Error loading file:[/red] {e}")
        raise typer.Exit(1)

    console.print(f"Analyzing: {data['name']} ({data['timestamp']})")

    successful_results = [
        r for r in data["results"] if r["success"] and r["performance"]
    ]

    if not successful_results:
        console.print("[red]No successful results to analyze[/red]")
        return

    durations = [r["performance"]["duration_seconds"] for r in successful_results]
    memory_peaks = [r["performance"]["memory_peak_mb"] for r in successful_results]

    console.print("\n[bold]Performance Analysis:[/bold]")
    console.print(f"  Successful benchmarks: {len(successful_results)}")
    console.print(f"  Duration range: {min(durations):.3f}s - {max(durations):.3f}s")
    console.print(
        f"  Memory range: {min(memory_peaks):.1f}MB - {max(memory_peaks):.1f}MB"
    )

    sync_results = [r for r in successful_results if "sync" in r["name"]]
    async_results = [r for r in successful_results if "async" in r["name"]]

    if sync_results and async_results:
        sync_avg = sum(
            r["performance"]["duration_seconds"] for r in sync_results
        ) / len(sync_results)
        async_avg = sum(
            r["performance"]["duration_seconds"] for r in async_results
        ) / len(async_results)

        console.print("\n[bold]Sync vs Async:[/bold]")
        console.print(
            f"  Sync average: {sync_avg:.3f}s ({len(sync_results)} benchmarks)"
        )
        console.print(
            f"  Async average: {async_avg:.3f}s ({len(async_results)} benchmarks)"
        )
        console.print(f"  Performance difference: {async_avg - sync_avg:+.3f}s")

    if quality_report:
        _generate_quality_report(data, console)


@app.command()
def baseline(
    output_file: Path | None = typer.Option(
        None, "--output", "-o", help="Save results to file"
    ),
) -> None:
    """Run baseline performance benchmark."""
    import asyncio
    import time
    from kreuzberg import extract_file_sync, batch_extract_file
    from kreuzberg._utils._cache import clear_all_caches

    console.print("[bold blue]Baseline Performance Benchmark[/bold blue]")

    test_files_dir = Path("tests/test_source_files")
    test_files = list(test_files_dir.glob("*.pdf"))

    if not test_files:
        console.print("[red]No test files found in tests/test_source_files[/red]")
        return

    single_file = test_files[0]
    mixed_files = test_files[:3] if len(test_files) >= 3 else [single_file] * 3

    results = {}
    console.print(f"Testing with file: {single_file.name}")

    # Cold cache test
    clear_all_caches()
    start_time = time.time()
    result = extract_file_sync(single_file)
    cold_duration = time.time() - start_time

    # Warm cache test
    start_time = time.time()
    result = extract_file_sync(single_file)
    warm_duration = time.time() - start_time

    # Batch test
    clear_all_caches()
    start_time = time.time()

    async def run_batch() -> list[Any]:
        return await batch_extract_file(mixed_files)

    asyncio.run(run_batch())
    batch_duration = time.time() - start_time

    results = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "test_file": str(single_file),
        "cold_cache_duration": cold_duration,
        "warm_cache_duration": warm_duration,
        "batch_duration": batch_duration,
        "speedup_factor": cold_duration / warm_duration if warm_duration > 0 else 0,
        "content_length": len(result.content),
    }

    # Display results
    console.print(f"\n[green]Cold cache extraction: {cold_duration:.3f}s[/green]")
    console.print(f"[green]Warm cache extraction: {warm_duration:.3f}s[/green]")
    console.print(f"[green]Batch extraction: {batch_duration:.3f}s[/green]")
    console.print(f"[yellow]Cache speedup: {results['speedup_factor']:.2f}x[/yellow]")

    if output_file:
        with open(output_file, "w") as f:
            json.dump(results, f, indent=2)
        console.print(f"[blue]Results saved to {output_file}[/blue]")
    else:
        # Save to default location
        output_dir = Path("results")
        output_dir.mkdir(parents=True, exist_ok=True)
        default_file = output_dir / "baseline.json"
        with open(default_file, "w") as f:
            json.dump(results, f, indent=2)
        console.print(f"[blue]Results saved to {default_file}[/blue]")


@app.command()
def statistical(
    trials: int = typer.Option(5, "--trials", "-t", help="Number of trials to run"),
    output_file: Path | None = typer.Option(
        None, "--output", "-o", help="Save results to file"
    ),
) -> None:
    """Run statistical benchmark with multiple trials."""
    import asyncio
    import statistics
    import time
    from kreuzberg import ExtractionConfig, extract_file
    from kreuzberg._utils._cache import clear_all_caches

    console.print("[bold blue]Statistical Performance Benchmark[/bold blue]")

    test_files_dir = Path("tests/test_source_files")
    pdf_files = list(test_files_dir.glob("*.pdf"))

    if not pdf_files:
        console.print("[red]No PDF test files found[/red]")
        return

    single_file = pdf_files[0]
    config = ExtractionConfig(
        force_ocr=True, ocr_backend="tesseract", extract_tables=True, chunk_content=True
    )

    console.print(f"File: {single_file.name}")
    console.print(f"Trials: {trials}")
    console.print("=" * 60)

    cold_times = []
    warm_times = []

    async def run_trial() -> tuple[float, float]:
        # Cold cache trial
        clear_all_caches()
        start_time = time.time()
        await extract_file(single_file, config=config)
        cold_time = time.time() - start_time
        cold_times.append(cold_time)

        # Warm cache trial
        start_time = time.time()
        await extract_file(single_file, config=config)
        warm_time = time.time() - start_time
        warm_times.append(warm_time)

        return cold_time, warm_time

    console.print("Running trials...")
    for i in range(trials):
        cold_time, warm_time = asyncio.run(run_trial())
        console.print(f"Trial {i + 1}: Cold={cold_time:.3f}s, Warm={warm_time:.3f}s")

    results = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "test_file": str(single_file),
        "trials": trials,
        "cold_cache": {
            "mean": statistics.mean(cold_times),
            "stdev": statistics.stdev(cold_times) if len(cold_times) > 1 else 0,
            "median": statistics.median(cold_times),
            "min": min(cold_times),
            "max": max(cold_times),
            "raw_times": cold_times,
        },
        "warm_cache": {
            "mean": statistics.mean(warm_times),
            "stdev": statistics.stdev(warm_times) if len(warm_times) > 1 else 0,
            "median": statistics.median(warm_times),
            "min": min(warm_times),
            "max": max(warm_times),
            "raw_times": warm_times,
        },
    }

    # Display summary
    cold_cache = results["cold_cache"]
    warm_cache = results["warm_cache"]

    console.print(
        f"\n[green]Cold cache - Mean: {cold_cache['mean']:.3f}s ± {cold_cache['stdev']:.3f}s[/green]"  # type: ignore[index]
    )
    console.print(
        f"[green]Warm cache - Mean: {warm_cache['mean']:.3f}s ± {warm_cache['stdev']:.3f}s[/green]"  # type: ignore[index]
    )
    speedup = cold_cache["mean"] / warm_cache["mean"] if warm_cache["mean"] > 0 else 0  # type: ignore[index]
    console.print(f"[yellow]Average speedup: {speedup:.2f}x[/yellow]")

    if output_file:
        with open(output_file, "w") as f:
            json.dump(results, f, indent=2)
        console.print(f"[blue]Results saved to {output_file}[/blue]")
    else:
        # Save to default location
        output_dir = Path("results")
        output_dir.mkdir(parents=True, exist_ok=True)
        default_file = output_dir / "statistical.json"
        with open(default_file, "w") as f:
            json.dump(results, f, indent=2)
        console.print(f"[blue]Results saved to {default_file}[/blue]")


@app.command()
def serialization(
    output_file: Path | None = typer.Option(
        None, "--output", "-o", help="Save results to file"
    ),
) -> None:
    """Benchmark serialization performance (JSON vs msgpack)."""
    import time
    import statistics
    from kreuzberg._types import ExtractionResult

    console.print("[bold blue]Serialization Performance Benchmark[/bold blue]")

    # Create test data
    large_content = "This is a realistic OCR result content. " * 500
    metadata: dict[str, Any] = {
        "file_path": "/some/long/path/to/document.pdf",
        "ocr_backend": "tesseract",
        "ocr_config": {"language": "eng", "psm": 3},
        "processing_time": 15.234,
        "confidence_scores": [0.95, 0.87, 0.92, 0.88, 0.94],
        "page_count": 10,
    }

    test_result = ExtractionResult(
        content=large_content,
        mime_type="text/plain",
        metadata=metadata,  # type: ignore[arg-type]
        chunks=["chunk1", "chunk2", "chunk3"] * 20,
    )

    cache_data = {
        "type": "ExtractionResult",
        "data": test_result,
        "timestamp": time.time(),
        "version": "1.0.0",
    }

    trials = 100
    json_serialize_times = []
    json_deserialize_times = []

    # JSON benchmarks
    console.print("Benchmarking JSON serialization...")
    for _ in range(trials):
        start_time = time.time()
        json_str = json.dumps(cache_data, default=str)
        json_serialize_times.append(time.time() - start_time)

        start_time = time.time()
        json.loads(json_str)
        json_deserialize_times.append(time.time() - start_time)

    # Try msgpack if available
    msgpack_serialize_times = []
    msgpack_deserialize_times = []

    try:
        import msgpack  # type: ignore[import-not-found]

        console.print("Benchmarking msgpack serialization...")

        for _ in range(trials):
            start_time = time.time()
            msgpack_bytes = msgpack.packb(cache_data, default=str)
            msgpack_serialize_times.append(time.time() - start_time)

            start_time = time.time()
            msgpack.unpackb(msgpack_bytes, raw=False)
            msgpack_deserialize_times.append(time.time() - start_time)

    except ImportError:
        console.print(
            "[yellow]msgpack not available - skipping msgpack benchmarks[/yellow]"
        )

    results = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "trials": trials,
        "data_size_chars": len(large_content),
        "json": {
            "serialize_mean": statistics.mean(json_serialize_times),
            "serialize_stdev": statistics.stdev(json_serialize_times),
            "deserialize_mean": statistics.mean(json_deserialize_times),
            "deserialize_stdev": statistics.stdev(json_deserialize_times),
        },
    }

    if msgpack_serialize_times:
        results["msgpack"] = {
            "serialize_mean": statistics.mean(msgpack_serialize_times),
            "serialize_stdev": statistics.stdev(msgpack_serialize_times),
            "deserialize_mean": statistics.mean(msgpack_deserialize_times),
            "deserialize_stdev": statistics.stdev(msgpack_deserialize_times),
        }

        # Calculate speedup
        json_data = results["json"]
        msgpack_data = results["msgpack"]

        json_total = json_data["serialize_mean"] + json_data["deserialize_mean"]  # type: ignore[index]
        msgpack_total = (
            msgpack_data["serialize_mean"] + msgpack_data["deserialize_mean"]  # type: ignore[index]
        )
        speedup = json_total / msgpack_total if msgpack_total > 0 else 0
        results["msgpack_speedup"] = speedup

    # Display results
    json_data = results["json"]
    console.print(
        f"\n[green]JSON serialize: {json_data['serialize_mean']:.6f}s ± {json_data['serialize_stdev']:.6f}s[/green]"  # type: ignore[index]
    )
    console.print(
        f"[green]JSON deserialize: {json_data['deserialize_mean']:.6f}s ± {json_data['deserialize_stdev']:.6f}s[/green]"  # type: ignore[index]
    )

    if "msgpack" in results:
        msgpack_data = results["msgpack"]
        console.print(
            f"[green]msgpack serialize: {msgpack_data['serialize_mean']:.6f}s ± {msgpack_data['serialize_stdev']:.6f}s[/green]"  # type: ignore[index]
        )
        console.print(
            f"[green]msgpack deserialize: {msgpack_data['deserialize_mean']:.6f}s ± {msgpack_data['deserialize_stdev']:.6f}s[/green]"  # type: ignore[index]
        )
        speedup_val = results["msgpack_speedup"]
        console.print(f"[yellow]msgpack speedup: {speedup_val:.2f}x[/yellow]")

    if output_file:
        with open(output_file, "w") as f:
            json.dump(results, f, indent=2)
        console.print(f"[blue]Results saved to {output_file}[/blue]")
    else:
        # Save to default location
        output_dir = Path("results")
        output_dir.mkdir(parents=True, exist_ok=True)
        default_file = output_dir / "serialization.json"
        with open(default_file, "w") as f:
            json.dump(results, f, indent=2)
        console.print(f"[blue]Results saved to {default_file}[/blue]")


if __name__ == "__main__":
    app()
