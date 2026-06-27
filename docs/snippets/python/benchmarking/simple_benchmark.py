```python title="simple_benchmark.py"
import asyncio
import time

from xberg import ExtractInput, ExtractionConfig, extract


async def benchmark_extractions():
    config = ExtractionConfig(use_cache=False)
    input = ExtractInput.from_uri("document.pdf")
    num_runs = 10

    start = time.perf_counter()
    for _ in range(num_runs):
        await extract(input, config)
    sequential_duration = time.perf_counter() - start
    avg_sequential = sequential_duration / num_runs

    print(f"Sequential extraction ({num_runs} runs):")
    print(f"  - Total time: {sequential_duration:.3f}s")
    print(f"  - Average: {avg_sequential:.3f}s per extraction")

    start = time.perf_counter()
    tasks = [extract(input, config) for _ in range(num_runs)]
    await asyncio.gather(*tasks)
    parallel_duration = time.perf_counter() - start

    print(f"\nParallel extraction ({num_runs} runs):")
    print(f"  - Total time: {parallel_duration:.3f}s")
    print(f"  - Average: {parallel_duration / num_runs:.3f}s per extraction")
    print(f"  - Speedup: {sequential_duration / parallel_duration:.1f}x")

    cache_config = ExtractionConfig(use_cache=True)

    print("\nFirst extraction (populates cache)...")
    start = time.perf_counter()
    await extract(input, cache_config)
    first_duration = time.perf_counter() - start
    print(f"  - Time: {first_duration:.3f}s")

    print("Second extraction (from cache)...")
    start = time.perf_counter()
    await extract(input, cache_config)
    cached_duration = time.perf_counter() - start
    print(f"  - Time: {cached_duration:.3f}s")
    print(f"  - Cache speedup: {first_duration / cached_duration:.1f}x")


if __name__ == "__main__":
    asyncio.run(benchmark_extractions())
```
