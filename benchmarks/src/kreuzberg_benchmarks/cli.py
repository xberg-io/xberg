"""Command-line interface for Kreuzberg benchmarks."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import typer
from rich.console import Console

from .benchmarks import KreuzbergBenchmarks
from .models import FlameGraphConfig
from .runner import BenchmarkRunner

app = typer.Typer(help="Kreuzberg Performance Benchmarking Suite")
console = Console()


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

    sync_benchmarks = []
    async_benchmarks = []

    if comparison_only:
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
        timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
        output_file = output_dir / f"{suite_name}_{timestamp}.json"
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
def analyze(
    result_file: Path = typer.Argument(..., help="Benchmark result file to analyze"),
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


if __name__ == "__main__":
    app()
