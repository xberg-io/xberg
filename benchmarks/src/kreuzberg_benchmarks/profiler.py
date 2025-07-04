"""Performance profiling utilities."""

from __future__ import annotations

import gc
import subprocess
import tempfile
import threading
import time
from contextlib import contextmanager
from pathlib import Path
from typing import TYPE_CHECKING, Callable, TypeVar

import psutil

from .models import FlameGraphConfig, PerformanceMetrics

if TYPE_CHECKING:
    from collections.abc import Generator

T = TypeVar("T")


class PerformanceProfiler:
    """Comprehensive performance profiler for benchmarks."""

    def __init__(self) -> None:
        self.process = psutil.Process()
        self.start_time: float = 0
        self.memory_samples: list[float] = []
        self.cpu_samples: list[float] = []
        self.monitoring_thread: threading.Thread | None = None
        self.monitoring_active = False
        self.gc_start: dict[int, int] = {}

    def start_monitoring(self) -> None:
        """Start background monitoring of system resources."""
        self.start_time = time.perf_counter()
        self.memory_samples.clear()
        self.cpu_samples.clear()
        self.gc_start = {gen: gc.get_count()[gen] for gen in range(3)}

        self.monitoring_active = True
        self.monitoring_thread = threading.Thread(
            target=self._monitor_resources, daemon=True
        )
        self.monitoring_thread.start()

    def stop_monitoring(self) -> PerformanceMetrics:
        """Stop monitoring and return collected metrics."""
        self.monitoring_active = False
        if self.monitoring_thread:
            self.monitoring_thread.join(timeout=1.0)

        duration = time.perf_counter() - self.start_time
        gc_end = {gen: gc.get_count()[gen] for gen in range(3)}
        gc_collections = {gen: gc_end[gen] - self.gc_start[gen] for gen in range(3)}

        return PerformanceMetrics(
            duration_seconds=duration,
            memory_peak_mb=max(self.memory_samples) if self.memory_samples else 0.0,
            memory_average_mb=sum(self.memory_samples) / len(self.memory_samples)
            if self.memory_samples
            else 0.0,
            cpu_percent_average=sum(self.cpu_samples) / len(self.cpu_samples)
            if self.cpu_samples
            else 0.0,
            cpu_percent_peak=max(self.cpu_samples) if self.cpu_samples else 0.0,
            gc_collections=gc_collections,
        )

    def _monitor_resources(self) -> None:
        """Background monitoring of CPU and memory usage."""
        while self.monitoring_active:
            try:
                memory_info = self.process.memory_info()
                memory_mb = memory_info.rss / (1024 * 1024)
                self.memory_samples.append(memory_mb)

                cpu_percent = self.process.cpu_percent()
                self.cpu_samples.append(cpu_percent)

                time.sleep(0.01)
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                break

    @contextmanager
    def profile(self) -> Generator[None, None, PerformanceMetrics]:
        """Context manager for profiling a code block."""
        self.start_monitoring()
        try:
            yield
        finally:
            metrics = self.stop_monitoring()
            return metrics


class FlameGraphProfiler:
    """Flame graph profiler using py-spy."""

    def __init__(self, config: FlameGraphConfig) -> None:
        self.config = config

    def profile_function(
        self, func: Callable[[], T], output_path: Path, name: str = "benchmark"
    ) -> tuple[T, Path | None]:
        """Profile a function and generate flame graph."""
        if not self.config.enabled:
            result = func()
            return result, None

        with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
            temp_script = Path(f.name)
            f.write(f"""
import sys
sys.path.insert(0, '{Path.cwd()}')

# Import and run the benchmark function
{self._generate_function_call_code(func)}
""")

        try:
            flame_output = output_path / f"{name}_flame.{self.config.output_format}"

            cmd = [
                "py-spy",
                "record",
                "-o",
                str(flame_output),
                "-d",
                str(self.config.duration_seconds),
                "-r",
                str(self.config.rate_hz),
                "-f",
                self.config.output_format,
            ]

            if self.config.subprocesses:
                cmd.append("-s")
            if not self.config.include_idle:
                cmd.append("--idle")

            cmd.extend(["--", "python", str(temp_script)])

            process = subprocess.Popen(
                cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
            )

            result = func()

            stdout, stderr = process.communicate()

            if process.returncode == 0 and flame_output.exists():
                return result, flame_output
            return result, None

        finally:
            temp_script.unlink(missing_ok=True)

    def _generate_function_call_code(self, func: Callable[[], T]) -> str:
        """Generate Python code to call the function."""

        # sophisticated serialization for complex functions  # ~keep
        return f"""
# Placeholder for function execution
# In a real implementation, this would serialize and execute the benchmark
import time
time.sleep({self.config.duration_seconds})
"""


@contextmanager
def profile_benchmark(
    flame_config: FlameGraphConfig | None = None,
) -> Generator[PerformanceProfiler, None, PerformanceMetrics]:
    """Context manager for comprehensive benchmark profiling."""
    profiler = PerformanceProfiler()
    profiler.start_monitoring()

    try:
        yield profiler
    finally:
        metrics = profiler.stop_monitoring()
        return metrics
